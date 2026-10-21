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
| `changes_since` | `changes_since(seq, limit)`, with the keys affected and `reprint_all` for profile changes |
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
