#implementation

> Crate root of `sophia-interface`: what Moonkale's `sources/sophia` adapter wraps. Source: `sophia_interface.rs`.
>
> Sources: [[SPEC-004 Moonkale Interface]]; Moonkale `packages/core/src/source/`. code: `sophia_interface.rs`, [[edit]], [[profiles]]

## Shape

`Interface<B>` is a store opened for **one reader**; the reader id is given at `open` (SPEC-004 §3a). Its public types are:
- `Key`: `Content(digest)` → Moonkale's `NodeId::from_content`; `Derived(s)` → `NodeId::derive(source, s)`;
- `NodeView`, `EdgeView`, `Subgraph`;
- `Direction`, `Change`, `Risk`;
- `Written`, `EditError`, and `Diagnostic` with byte spans.

None of them is a Moonkale, driver or async type.

## Operations, mapped to Moonkale's `Source`

| Moonkale | Interface |
|---|---|
| `Query::Node` | `node(key)` |
| `Query::Children` | `children(key)`: sub-namespaces, then names, each sorted; the root also holds `sophia:Settings`. A skip scan over `names_from` (below) |
| `Query::Neighbours` | `neighbours(key, depth, dir, limit)` |
| `Query::All` | `all(limit)`: namespaces, names, definitions and named nominal entities, with `Contains`, `Names` and the dependency edges among them; `truncated` when the cap cuts |
| `Query::Text { "sql" }` | `sql(query, limits)` (SQLite): table plus subgraph |
| `fetch_text` | `fetch_text(key)`: names print their whole recursive group in the reader's profile; definitions print read-only; settings print the reader's profile; records that are not sophialang print as described below |
| `apply(WriteText)` | `write_text(key, expected, text)` |
| `apply(CreateText)` | `create_text(ns, name, text)` |
| `apply(CreateDir)` | `create_dir(ns, name)` |
| `apply(Rename)` | `rename(key, to)` (`/` ↔ `.`) |
| `apply(Delete)` | `delete(key)` |
| `changes_since` | `changes_since(seq, limit)`, with the keys affected and `reprint_all` for profile changes; a collected record (`remove-definition`, SR-146) affects its content key |
| `classify` | `Interface::classify(dialect)` |

## Edit tokens

`fetch_text` returns, as a name's version, the edit token from `text_token(path)`: a hash of every name the text covers, with versions and targets (SPEC-004 §3b). `write_text` requires the same token. The profile is deliberately not part of it, because the parser reads every editable profile.

## Tests

`tests/acceptance.rs` covers SPEC-004 §9 item by item, on SQLite and redb (11 tests):
- browsing, including relations as nominal entities with `MethodOf` members;
- editing, including a no-op re-save, a parse error, a conflict, and an edit inside a recursive group;
- a rename in the change feed;
- switching profiles (reprint, edit in Rust, another reader still sees Julia);
- creating definitions, a relation and a rule from text;
- `sql`.

## Difficulties met

- **A deadlock:** `neighbours` called `children` inside an open read, and SQLite's connection mutex is not reentrant. Fixed with `children_in(tx, …)`. Rule kept for this crate: never call a public method from inside a `store.read` closure.
- **Not atomic across steps:** an edit is several store transactions (add records, update names, attach relation members). A failure halfway leaves a consistent but partial state, never a corrupt one. A single-transaction batch API is a later improvement ([[SP-008 Edits span several transactions]]).

**The `query` dialect** (2026-10-20, [[SPEC-007 Sophia Query Language]] §8): `query(text, limits)` runs Sophia's own query language on any backend and returns the table (ids as hex text) plus a node for every id cell. `classify("query")` is `ReadOnly`. A stored query's name prints in the query syntax (`print_target`).

## Records that are not sophialang (2026-10-21)

Moonkale's desktop check found that `fetch_text` failed for every Julia record of an ingested package (146 of 945 names in LinearAlgebra: structs, abstract types, consts) and for the ingest manifest: `print_target` decoded every record as sophialang. Now `foreign_text` prints:
- a **source file** (`lang = source`): its bytes, after a `#` header;
- a **definition ingested from another language**: a `#` header (language, kind, id, normaliser, file and byte range) and the **source text it was read from**, through its `source-span` fact ([[types]]). One id can have several spans, because the name is not part of a definition's identity (`struct LAPACKException … end` and `struct RankDeficientException … end` are one record in LinearAlgebra); the span whose text contains the printed name is chosen;
- anything else (an ingest manifest, a Metamath statement, a Julia record ingested before spans were recorded): a `#` summary with display, size and references.

Saving such a text parses it as sophialang and fails or translates; there is no Julia write path. Stores ingested before 2026-10-21 have no spans: re-ingest the package.

## Listing (2026-10-21)

`children` lists a namespace's direct children with a **skip scan**: names in batches of 512 along the name index (`names_from`), and on meeting a sub-namespace, a jump past it. The cost grows with the number of children, not with the number of names below them. Nodes are built from the scanned entries, without a second lookup per name. Measured with `examples/listing_timing.rs` on a copy of B-15's 100 000-definition store (SQLite, best of 5, warm):

| Call | Before | After |
|---|---|---|
| `children(ns:)` (2 nodes) | 32 ms | 0.1 ms |
| `children(ns:gen)` (100 000 names) | 133 ms | 75 ms |
| `all(100000)` | 418 ms | 326 ms |

Moonkale first measured 0.9 s, 0.9 s and 3.3 s through its adapter; that was a cold page cache (the 680 MB file evicted). Warm, through the adapter (Moonkale's `examples/browse.rs`, reported by the Moonkale agent): root 44 ms → 0.3–1 ms, `ns:gen` 270 → 240 ms, `all` 572 → 490–500 ms. The adapter adds about 110 ms per 100 000 nodes (conversion, id-to-key map, a mutex lock per node), on Moonkale's side.

`tests/acceptance.rs` now also covers listing around namespace boundaries (`a.b`, `a.b!`, `a.b.c`, ``a.`b.c` ``, `a.b0`, `ab.x`), foreign records printing, and diagnostics; 45 tests on four backends.

## Deep definitions and large graphs (2026-10-23)

Moonkale's coarse graph query called `all(2 000 000)` on a copy of B-15's 100 000-name store, and a Tokio blocking worker (2 MiB stack) overflowed (Moonkale P-229). Two causes, both fixed:
- **Recursion per nesting level:** `edges_at` decoded each definition and walked its term to find `Calls`. Generated definitions nest up to 700 levels (encoded fields); decoding, walking and dropping the term recurse once per level. Reproduced: in a debug build, decoding and walking every record on a 2 MiB thread overflows; in release it does not. `Calls` now come from `encode::called_refs`, a scan of the bytes with an explicit stack.
- **A quadratic edge check:** `all` de-duplicated edges with `Vec::contains`. With 739 273 edges it did not finish in several minutes. Now a `HashSet` (`Prop` and `EdgeView` derive `Eq` and `Hash`).

`all(2 000 000)` on that store now returns 199 256 nodes and 739 273 edges on a 2 MiB thread: 2.6 s in a release build, 14.9 s in a debug build. Still recursive and unbounded: printing (`fetch_text`) and decoding a deep definition for display. Such texts are also beyond the parse limit ([[SP-015 Stored definitions deeper than the parse limit cannot be edited]]).

## Folder stores (2026-10-24, SPEC-009)

- In a store marked derived (`Store::set_derived`), `write_text`, `create_text`, `create_dir`, `rename`, `delete` and settings writes return `EditError::Refused`: the files are the truth.
- `fetch_text` prints a folder store's definitions from the file manifest's span (header `# read-only: from <path> bytes a..b`), before falling back to `source-span` facts.
- `text_source(key)` returns the file, source id and span a definition's text came from; the caller compares the source id with `sophia_store::folder::source_record(path, current text).id` to tell whether the store is stale for that file.
- Tested in `tests/acceptance.rs` (`folder_store_is_read_only_and_prints_files`, four backends).

## The web client's read path (2026-10-24, SPEC-009 §4)

`Interface::open_read_only(backend, reader)` opens without writing (no built-in profile rows; missing ones read as Julia's). With `Sqlite::from_bytes` it is what Moonkale's web client would use. `tests/wasm_read.rs` runs on `wasm32-unknown-unknown` under Node, against `tests/fixtures/web.sophia` (108 KB, written by the ignored native test in `tests/wasm_fixture.rs`): it browses (`children`, `neighbours`, `all`), prints a sophialang definition and a folder-store file's text, gets `text_source`, runs a recursive `query`, and checks that writes are refused. **4 of 4 pass.** Not run in a browser yet (the runner needs geckodriver or chromedriver, not installed).

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked --root $SOPHIA_CORPORA/tools   # must match the wasm-bindgen crate
PATH=$SOPHIA_CORPORA/tools/bin:$PATH CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test -p sophia-interface --no-default-features --features sqlite --target wasm32-unknown-unknown --test wasm_read
```

`tests/wasm_deep.rs` (2026-10-24): the read path (normalise and encode, decode with display names, equality, clone, hash, print in two profiles, drop) on sum, lambda and let chains 100 000 deep, in wasm: 3 of 3 pass under Node and in headless Firefox (geckodriver 0.37.1). Before the explicit stacks, 10 000 levels already raised `RangeError: Maximum call stack size exceeded` under Node. `tests/wasm_read.rs` also passes in Firefox:

```sh
WASM_BINDGEN_USE_BROWSER=1 GECKODRIVER=$(which geckodriver) PATH=$SOPHIA_CORPORA/tools/bin:$PATH \
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test -p sophia-interface --no-default-features --features sqlite --target wasm32-unknown-unknown --test wasm_deep
```

Since the parser and elaborator use explicit stacks too (2026-10-24, later), `tests/wasm_deep.rs` also checks `Debug` and parses the printed text back to the same id in both profiles: 3 of 3 under Node (22 s, debug build) and in Firefox (19 s).
