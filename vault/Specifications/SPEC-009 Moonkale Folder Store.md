#design #definition

> **Draft, a proposal by Claude (Sophia), 2026-10-23. Agreed by the Moonkale agent the same day, including the conditions in §7 and the `binding()` API in §8. **Accepted by Daniel on 2026-10-24**, together with SP-015 and a `v0.1.0` tag once §2, §3, SP-015 and the wasm read-only test are done.** **Implemented 2026-10-24:** §2, §3 and §7's error and stale check (`crates/sophia-store/src/folder.rs`, [[folder]]; the interface's derived mode and `text_source`). §4's read-only test passes on wasm32 under Node (4 of 4: browse, print, query, writes refused; `tests/wasm_read.rs`), after three fixes for SQLite in memory and a query deadline without a clock. Not run in a browser yet. Open: the tag (§5). It answers Moonkale's five questions for Moonkale's WP-21 after Moonkale's ADR-0029, which Daniel accepted on 2026-10-23 ("Agree!"): Sophia stores Moonkale's code graph, one store per opened folder under `.moonkale/`, and **files stay the truth**. The store is a derived cache: deleting it loses nothing.
>
> Sources: Moonkale `markdown/decisions/ADR-0029 Sophia stores the code graph.md` (master `fb6b16e5`), ADR-0028 (files or the graph); [[SPEC-001 Sophia Store v0]], [[SPEC-004 Moonkale Interface]], [[SPEC-006 Worlds, Artifacts and Evidence]] §1a (source records); [[SP-008 Edits span several transactions]], [[SP-015 Stored definitions deeper than the parse limit cannot be edited]]. code: `crates/sophia-ingest/src/source.rs`, `sophia_ingest.rs` (`store_items`), `crates/sophia-store/src/logic.rs`.

## 1. The write path: Moonkale hands Sophia finished records

**Proposal: yes, Moonkale's index produces the records; Sophia does not parse.** One parse, on Moonkale's side, with the same crate (`moonkale-content-id`) that Sophia's ingest uses, so the records are the ones `sophia-ingest/src/code.rs` would make.

What Moonkale hands over, per file:

| Part | Content | Notes |
|---|---|---|
| source | the file's path (relative to the folder) and its text | stored as a source record (`lang = source`, SPEC-006 §1a). See below for why the text is kept |
| records | `Record`s with WP-66 ids: `lang` (`julia`, `rust`, `python`), `normaliser = moonkale-wp66@7`, `kind`, `component`, `index`, `component_size`, canonical `bytes` (references written as target digests), `display` (the definition's name), `refs` (targets outside the definition's own component) | exactly `code::to_record`'s output |
| spans | per record, its byte range in the file | become `source-span` facts, so `fetch_text` shows the file's own text |
| names | `(path, record)` for each definition that binds a name, in Sophia's dotted form (`jl.Pkg.Sub.f`) | Moonkale computes the path. Sophia's rule is `binding()` in `sophia_ingest.rs`; sharing it as a function is better than restating it |
| generic functions | `(path, member records)` for each (module, name) with methods, Julia's ownership rule ([[Julia Multiple Dispatch and World Age]], [[Julia Conformance]]) | nominal entities with members, as `store_items` makes them |

**What Sophia checks on write:** the store's own invariants (a record's kind follows from its content, SP-012; a component's members agree), and that every `refs` target is stored or in the same batch. **What it does not check:** that a WP-66 id is the right digest of its bytes. Sophia's `Record::verify` is for Sophia's own Merkle ids; a WP-66 id can only be recomputed with `moonkale-content-id`. Because the source text is stored, `sophia-ingest` can rebuild and compare the records of a folder store at any time (`reconstruct`, as for LinearAlgebra), as an audit, not on every write.

**Why store the source text** although the files are the truth: (a) the web client may not have the files, and `fetch_text` needs them; (b) rebuilding and checking needs them (above); (c) the cost is the size of the folder's code, small next to the derived records (about 10× source, [[Current Inventory]]). If Moonkale prefers, the text can be left out and only its hash kept; then (a) and (b) are lost for that store.

## 2. Incremental updates: "replace everything that came from file F"

**Proposal: a per-file manifest, and one write transaction for a set of files.**

- **Provenance key:** the file's path relative to the folder. It lives in a **file manifest** (a record, `lang = sophia-ingest`, kind `file-ingest`, like today's ingest manifest), which lists the source id, the normaliser, and every name, record, span and generic-function membership the file contributed. The meta row `file/<path>` points at the file's current manifest. Names do **not** carry provenance; a name is a name.
- **The operation:** `Store::replace_files(&[FileIngest])`, one write transaction for any number of files:
  1. compute, per file, the difference between its old manifest and the new contribution;
  2. remove names that are gone, add new ones, move names whose target changed (logged as updates, so the change feed and lineages work as today);
  3. update generic-function memberships; a generic function's name lives while any file contributes a member;
  4. record the spans, replace the file manifests, update `file/<path>`.
- **Moved names** (a definition moves from file A to file B) are only consistent when A and B are replaced in the **same** call: otherwise B's add finds the name still owned by A. So a change touching several files is one `replace_files` call. A name claimed by two files in one call is an error that names both files.
- **Staleness:** a file is stale when its text's hash differs from its manifest's source id; the whole store is stale when the meta row `folder/normaliser` differs from Moonkale's current normaliser (rebuild everything, as for any normaliser bump).
- This needs a single-transaction write path in `logic.rs`, which also closes [[SP-008 Edits span several transactions]] for this case.

## 3. Files-as-truth is a store mode, enforced

**Proposal: a meta row `store/mode = derived`, set when the store is created.** In that mode:
- `Interface::write_text`, `create_text`, `create_dir`, `rename`, `delete` and settings writes are refused with a read-only error ("derived store: edit the files"), so a misused call cannot reprint code;
- `replace_files` is the only content write;
- `fetch_text` prints the file's text through spans, never a reprint (it already does for Julia records since `4c2aaab3`).

A convention ("Moonkale never calls the write path") would work until someone calls it; a mode costs one meta read per write.

## 4. Wasm

**Measured (2026-10-23, `cargo build --target wasm32-unknown-unknown`, debug):** `sophia-hash`, `sophialang-core`, `sophialang-syntax`, `sophia-store` (with `redb`, or with `sqlite`), `sophia-query` (`sqlite`) and `sophia-interface` (`sqlite`, or `redb`) **compile**. With rusqlite 0.40, SQLite on this target comes from `sqlite-wasm-rs` 0.5.5 (SQLite compiled to wasm, with its own VFS layer), so no C toolchain is needed. **Nothing has run in a browser yet.**

**Known runtime hazards** (from reading the code, not observed): writes stamp the log with `SystemTime::now()`, and `parse_text` spawns a thread; both panic on `wasm32-unknown-unknown`. `Interface::open` may write meta rows on first open. A read-only client avoids the first two; the third needs a read-only open.

**Proposal: SQLite.** The folder store is then one file format on the server, the desktop and the web; the `sql` dialect needs SQLite; Moonkale's own state is SQLite already. The web client opens the store **read-only**, from its bytes (an in-memory database) or from the browser's origin-private file system (OPFS). redb also compiles and stays the fallback.

**Next step:** a `wasm-bindgen-test` that opens a store from bytes and runs `children`, `neighbours`, `fetch_text` and a `query`, plus a read-only `Interface::open`. Only then is "the store builds for wasm" (ADR-0029 gate 3) true in the sense Moonkale needs.

## 5. Versioning

**Proposal:** tag `v0.1.0` (the workspace version already) when:
1. §1–§3 are implemented with tests on all four backends (`replace_files`, the derived mode);
2. [[SP-015 Stored definitions deeper than the parse limit cannot be edited]] is closed (revised 2026-10-24: no depth limit; every pass depth-safe);
3. the wasm read-only test of §4 passes.

Afterwards 0.x semantics: a breaking API change bumps the minor version. A change of any content id is a migration (SPEC-006 §6) whatever the version, and is coordinated with Moonkale as v7 was. Tagging is Daniel's decision.

## 6. Open

- Whether Moonkale's index also hands over **call edges to generic functions** (Sophia's `uses_generic`, drawn but not part of ids) and other edges that are not in `refs`. Proposal: yes, as facts about the calling record, so they are replaced with the file.
- How Moonkale shares `binding()` (the name a definition binds), so both sides cannot drift.
- Rust and Python names (`store_code`'s rules are per file, not per crate or package yet).

## 7. Moonkale's review (2026-10-23), folded in

Agreed in substance. The conditions and answers, which amend the sections above:

- **§1 Which sources are stored:** only files Moonkale's index actually parses: core-language code files, not gitignored, under the index's size cap. Never configuration, `.env` or anything else, so secrets are not copied into `.moonkale/`. Moonkale enforces the selection; Sophia stores what it is given.
- **§1 Stale text:** `fetch_text` must not present stored text as the file when the file has changed. Sophia cannot see the files (in the browser it never can), so the check belongs to whoever reads the file: in derived mode `fetch_text` also returns the **path and source id** the text came from, and the caller compares the source id with the hash of the current file (`sophia_ingest::source::source_record(path, text).id`, the same function Sophia uses). On a mismatch the store is stale for that file; Moonkale reads the file itself.
- **§1 Checks:** accepted as written (only Moonkale can verify WP-66 ids; acceptable for a derived cache).
- **§2 Batches:** Moonkale sends every file changed in one watcher debounce window as one `replace_files` call; a rename is remove-old plus add-new in that call. A name claimed by a file **outside** the batch is a structured error, `StoreError::NameClaimed { name, owner: <path> }`, so Moonkale can retry with that file added.
- **§3** derived mode: enforced. **§4** wasm: SQLite, read-only from bytes first, OPFS later; the read-path `wasm-bindgen-test` is Moonkale's gate 3.
- **§6 Generic call edges:** yes, facts about the calling record, replaced with its file.
- **§6 `binding()`:** moves into `moonkale-content-id`, owned by Moonkale and versioned with the normaliser: a naming change is a `moonkale-wp66@N` bump on both sides. What moves, and the signature Sophia asks for, are in §8.
- **§6 Rust and Python names**, Moonkale's proposal, also in `moonkale-content-id`:
  - Rust: `rs.<crate>.<module path>.<item>` (crate from the nearest `Cargo.toml`, module path from the module tree); impl methods `rs.<crate>.<mod>.<Type>.<method>`; trait impls keyed by type and trait.
  - Python: `py.<package>.<module>.<name>` (directories with `__init__.py`, or the `pyproject.toml` root); class members `py.<pkg>.<mod>.<Class>.<name>`.
  - Unknown root: `rs.<path-derived>` / `py.<path-derived>`, flagged.

## 8. `binding()` in `moonkale-content-id`: what Sophia needs

Today the rule is spread over `sophia-ingest`: `code::julia_scopes`, `code::julia_extends` and `BASE_SUBMODULES` (which function a qualified or imported method extends), `sophia_ingest::binding` (method of a generic function, or a named definition), and `entity_path` with `segment` (the dotted spelling). Proposed public API, so both sides call one function:

```rust
pub enum Binding {
    /// A method of the generic function at this absolute path (module path + name).
    Method(Vec<String>),
    /// A directly named definition (type, constant, macro, module item).
    Named(Vec<String>),
}

/// What a definition binds, or None (unnamed; a qualified method it cannot
/// place, such as `Base.$f`). `package` is the root module or crate.
pub fn binding(def: &Definition, package: &str) -> Option<Binding>;

/// The dotted name: `entity_path("jl", ["LinearAlgebra", "BLAS", "gemm!"])`
/// = `jl.LinearAlgebra.BLAS.gemm!`; a segment that is not a plain identifier
/// (letters, digits, `_`, `!`, not starting with a digit) is backquoted.
pub fn entity_path(prefix: &str, key: &[String]) -> String;
```

where `Definition` is the crate's own per-definition output, carrying what `binding` reads: name, syntax kind, whether the name is qualified, the module path, and the resolved `extends` (computed inside the crate by today's `julia_scopes` / `julia_extends` logic, including `BASE_SUBMODULES`).

Requirements from Sophia's side, for every language:
1. **Valid store paths:** every segment passes Sophia's `validate_path` (a plain identifier, or a backquoted segment without whitespace or backquotes). Names that cannot be spelled so (Julia `var"…"` with spaces) bind nothing and are reported.
2. **Deterministic** from the files alone, whatever the order files are read in.
3. **Collisions are explicit:** several definitions under one path are a `Method` group (a nominal entity with members), never two names. This matters for Rust: `impl Foo<i32> { fn new }` and `impl Foo<u8> { fn new }` both give `rs.c.m.Foo.new`; for trait impls, the trait should be a backquoted segment (`` rs.c.m.Foo.`fmt::Display`.fmt ``) so that two traits' methods do not collide.
4. **Sophia's golden test** (`tests/golden-moonkale/`) gains the expected `binding` and `entity_path` for every golden definition, so a drift fails on both sides.

**Plan after acceptance:** Moonkale moves `julia_scopes`, `julia_extends` and `BASE_SUBMODULES` into `moonkale-content-id`, adds Rust and Python naming, and extends its golden data with `binding` and `entity_path` per definition; then `sophia-ingest` pins that revision and deletes its copy. Sophia implements `replace_files`, the derived mode and `NameClaimed` (§2, §3, §7), and the wasm read-path test (§4).
