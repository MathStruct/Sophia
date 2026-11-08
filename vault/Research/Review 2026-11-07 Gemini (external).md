#reference #open-problem

> A cross-repository review of Moonkale and Sophia, written by Gemini on 2026-11-07 and placed beside the Moonkale folders by Daniel (`~/Code/Moonkale/Moonkale-Sophia-Review/`, outside both repositories). Part 01 is about Sophia, and part 04 is about the boundary. It read the code statically, plus two Python scripts. Several findings describe Sophia as it was before 2026-10-24: for example, it says SPEC-010 is missing, and that sophia-ingest has its own copy of the normaliser. This note records each Sophia point, **whether it was right on the current code** (`07ce70e`), and what was done. The Moonkale findings (parts 02, 03 and Moonkale's side of 04) are the Moonkale agent's to triage. Triaged 2026-11-08 by Claude (Sophia). **Daniel said to fix them the same day (SR-153), and the five small fixes are done, in `v0.1.9`** (see below).
>
> Sources: the review's files `01_…`, `04_…`, `README.md`, `scripts/divergence_report.json`. code: the files named below.

## Verdicts

| ID | claim | verdict | evidence |
|---|---|---|---|
| X-BUG-01 | Moonkale's normaliser gives automorphic members of a recursive component the same index and digest, yet keeps `component_size = group.len()` | **right, and it reaches Sophia's store** | Probe (temporary test, deleted): `fn f() -> i32 { g() }` / `fn g() -> i32 { f() }` through `sophia_ingest::code::file` gives `f` and `g` the same id, index 0, size 2. `store_code` accepts it and stores 1 member for a component of size 2. (`Store::verify` also returns `false`, but that is no evidence: it re-hashes with `sophia-hash`'s rules, so it returns `false` for every Moonkale-normalised record, a lone `fn h() -> i32 { 1 }` included, as SPEC-009 §1 says. Second probe, 2026-11-08.) **Fixed on Moonkale's side in `06fd112c`** (normaliser 7 → 8, size = number of classes); Sophia's pin has not moved yet. The DFS-versus-BFS part does not matter: Moonkale's ids were never meant to equal `sophia-hash`'s. The review's own script reports `false` for all three of its checks. Fix in `moonkale-content-id` (Moonkale's, an identity change). Sophia proposal: refuse on write a component whose stored members differ from `component_size` |
| S-BUG-02 | `SqliteTx::annotation` / `DuckTx::annotation` look up by id through `t_annotation_subject` | **right** | `sqlite.rs:559`: `WHERE id = ?1` on a table keyed `(subject, id)` scans it. An annotation with no subjects returns `None` there, but `Some` on the KV backends (backend equivalence broken). Fix: read `t_annotation` by its primary key |
| S-BUG-03 | magic sets emit a self-rule for a negated subgoal | **wrong** | The negated branch's magic predicate is the negated relation's. It equals the guard only for `p :- not p`, which stratification refuses. A duplicate rule would be harmless under set semantics. The review hedges this itself |
| S-BUG-04 | `apply` / `write_text` check the token, then write in many transactions | **right, known** | [[SP-008 Edits span several transactions]], open since 2026-10-20 |
| S-BUG-05 | `text_source` picks among files by `span text contains name` | **right** | `folder.rs:345`: two definitions with one digest, where the other's text contains the name as a substring, show the wrong text. `Interface::foreign_text` delegates to `text_source` (not duplicated, as the review says). Fix: prefer the file whose `owner/` row holds the name |
| S-BUG-06 | `run_cached` never evicts, and a miss writes | **right, known** | [[SP-014 The query result cache is never evicted]], open. A miss is a write transaction (`sophia_query.rs:399`) |
| S-BUG-01 | `child_entries` skip-scan misses names | **no bug stated** | The text reasons its way to "`{sub}/` is safe" and stops |
| S-PERF-01 | DuckDB `query_readonly` spawns a timer thread that polls every 2 ms | **right** | `duckdb_store.rs:151`. A query can wait up to 2 ms for the join. Fix: a `Condvar` with `wait_timeout` |
| S-PERF-02 | `load_ext` does N+1 `tx.record` lookups | **wrong (stale)** | `eval.rs:310` scans `Base::Def` with one `all_kinds()` call |
| S-PERF-03 | `gc_records` decodes every manifest on every save | **wrong (stale)** | No `gc_records` exists. `collect` (`folder.rs:891`) starts from the dropped records and uses indexed lookups (names, memberships, dependants) |
| S-PERF-04 | `component_names` scans a whole namespace | **wrong (stale)** | There is no such function. `export_snapshot` reads a namespace's subtree, because it exports that namespace |
| X-BUG-02 | ~1 450 lines of normaliser and Julia scope code duplicated between `moonkale-content-id` and `sophia-ingest`, drifted (`ambiguous_imports`, backtick stripping) | **wrong (stale)** | Since 2026-10-24 (`5f08ce6`), `sophia-ingest` imports parsing, normal form, ids, `julia_scopes` and `julia_extends` from `moonkale-content-id` (`code.rs:10`, `code.rs:29`), so Moonkale's `ambiguous_imports` applies to both. There is no backtick stripping in `sophia-ingest`; the review's script found none |
| §5.1 | SPEC-010 missing | **wrong (stale)** | [[SPEC-010 Store-Truth Folders and Generated Files]], implemented 2026-10-28 |
| §5.2 | Code Map omits `folder.rs` | **wrong** | listed in [[Code Map]] |
| §5.3 | DuckDB `query_readonly` has no table restriction, unlike SQLite's authorizer | **right** | User SQL on a DuckDB store can read `t_meta` (manifests and source texts). File access is already off (`enable_external_access = false`, `duckdb_store.rs:124`). Fix: refuse a statement that names a table other than the views, or open the query on a connection that only has the views |
| X-BUG-04 | store-truth folders can't write foreign code | **Moonkale's side** | Sophia provides `replace_files_checked` and `propose_files` (SPEC-010 §2, §5). Moonkale's message "await SPEC-010" predates them |

## Done (2026-11-08, `v0.1.9`)

Each fix has a test that fails without it and passes with it:
1. **S-BUG-02:** SQLite and DuckDB read an annotation by its primary key ([[sqlite]], [[duckdb_store]]).
2. **S-BUG-05:** source text comes from the file that owns the name, then by whole-word match ([[folder]], [[sophia_interface]]).
3. **S-PERF-01:** the DuckDB time budget waits on a `Condvar`. 2.1 ms → 0.69 ms per small query, measured ([[duckdb_store]]).
4. **§5.3:** DuckDB queries can read only the views, checked on the parse tree. This adds the `duckdb` crate's `json` feature ([[duckdb_store]]).
5. **X-BUG-01, Sophia's half:** writes refuse a component whose stored members don't match its size (`IncompleteComponent`, [[logic]]).

Also done:
- **`Store::verify`** returns a `Verification`. Moonkale's records come back as `Foreign` (size checked only), not `false` ([[logic]]).
- **The `moonkale-content-id` pin** moved to `06fd112c` (normaliser 8), with LinearAlgebra re-ingested and compared ([[code]]).

All 333 workspace tests pass. Clippy shows only the earlier warning in `tests/folder.rs`, and both wasm tests pass under Node.

Still open and known: SP-008 (one transaction per edit) and SP-014 (cache eviction).
