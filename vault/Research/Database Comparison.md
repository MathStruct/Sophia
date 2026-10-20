#comparison

> Daniel asked to try several databases and compare them (SR-64). This page states the candidates, what is measured, and the results as they come in. The comparison runs the same conformance tests and the same benchmarks ([[Benchmark Plan]] B-03 to B-06, B-11) on each backend behind `sophia_store::Backend`. Until the numbers say otherwise, SQLite is the default ([[S-ADR-0005 SQLite first behind a store trait]]).
>
> Sources: SQLite (sqlite.org; `rusqlite` with `bundled`); redb (github.com/cberner/redb); DuckDB (duckdb.org; the `duckdb` crate with `bundled`); LMDB (Chu, *MDB: A Memory-Mapped Database and Backend for OpenLDAP*, 2011; the `heed` crate); LadybugDB (Moonkale `research/Database Backends.md`); [[State of the Art - Graph Databases for Code]]. code: `crates/sophia-store-*`.

## Candidates

| Backend | Model | Embedded? | Platforms | Query language | Expected strength | Expected weakness |
|---|---|---|---|---|---|---|
| **SQLite** | relational, B-tree, WAL | yes | all, including Android and iOS | SQL, `WITH RECURSIVE` | transactional small writes; ubiquity; views for queries | recursive graph queries; a single writer |
| **redb** | typed key-value, copy-on-write B-tree, pure Rust | yes | all (no C toolchain) | none (Rust API) | hash lookups; no FFI; simple build | no query language: SQL views must be emulated |
| **DuckDB** | columnar, vectorised | yes | Linux, macOS, Windows (Android unclear) | SQL | analytics and scans (SR-66 precompilation statistics) | many small transactional writes |
| **LMDB** (heed) | memory-mapped B+tree | yes | Linux, macOS, Windows, Android | none (key-value) | very fast reads, zero-copy | one writer; fixed map size |
| **LadybugDB** | property graph | yes | Linux only for now (Moonkale P-144) | Cypher | multi-hop graph queries | platform coverage |
| HelixDB, FalkorDB, TypeDB, TursoDB | graph and vector; graph (Redis); typed; SQLite-compatible | mixed | mixed | various | — | server-shaped or young; considered after the first four |

## Results

Each run adds a dated table per benchmark and a one-paragraph conclusion. A backend that fails the conformance suite is recorded here with the failing test.

### 2026-10-20: SQLite and redb (M1)

Both pass the conformance suite (14 tests each). The full tables are in [[Benchmark Results]].

| | SQLite | redb |
|---|---|---|
| ingest at 10⁵ (defs/s) | 9 200 | 12 400 |
| record by id at 10⁵ | 4.5 µs | 1.4 µs |
| closure of 5 713 definitions | 22.8 ms | 7.5 ms |
| bytes on disk per definition | 6 800 | 10 800 (no compaction) |
| query language | SQL views, enforced read-only | none |
| build | C (bundled), every platform | pure Rust, every platform |

**Interim conclusion:** redb is faster at everything the editor does per keystroke. SQLite is smaller, queryable, and already in Moonkale. **SQLite stays the default** for now, because the `sql` dialect and Moonkale's familiarity matter more than 3 µs per lookup. That is only a defensible default until DuckDB and LMDB are measured (M2), and until real code (not synthetic) is measured.

### 2026-10-20 (evening): under Sophia's own query model

The forwarded review asked for a comparison of SQLite, LMDB and redb **under the proposed query model**: Sophia's own algebra on top, backends as access paths ([[SPEC-007 Sophia Query Language]], [[S-ADR-0011 Sophia has its own query algebra]]). DuckDB is included for completeness. Numbers are B-17 at 10⁴ definitions ([[Benchmark Results]]).

| | SQLite | redb | LMDB | DuckDB |
|---|---|---|---|---|
| Sophia's query language | yes, native executor | yes, native | yes, native | yes, native |
| the same query as SQL | **yes** (`WITH RECURSIVE` over base tables; mutual recursion refused) | — | — | not yet (§11 of SPEC-007) |
| access paths for the 8 base relations | all by index (`t_ref` both ways, kind, name target) | all; `refs_of` decodes the record (no forward-reference table) | as redb | all by index, but each point query costs ~0.2 ms |
| set-at-a-time scans (`all_kinds`, `all_refs`) | SQL | index-table scans | index-table scans | SQL |
| transitive dependants, magic sets (per probe) | 0.171 ms native, 0.167 ms SQL | 0.180 ms | **0.132 ms** | 18.8 ms |
| transitive dependencies (per probe) | 0.97 ms native, **0.84 ms** SQL | 1.22 ms | 1.00 ms | 146 ms |
| count by kind | 10.1 ms native, **4.6 ms** SQL | 5.4 ms | 4.8 ms | 7.6 ms |
| unreferenced definitions (negation) | 19.9 ms native, **2.7 ms** SQL | 13.6 ms | 8.3 ms | 2.0 s |
| names by prefix | 3.8 ms native, **0.44 ms** SQL | 3.5 ms | 3.0 ms | 6.4 ms |
| zero-copy reads | no (copies out of its pager) | possible (`AccessGuard`) | **yes**: 0.21 µs borrowed against 0.87 µs copied and decoded ([[Zero-Copy Storage of Canonical Graphs]]) | no |
| writers | one (WAL; readers concurrent) | one | one (lock-free readers) | one |

**At 10⁵ (SQLite, redb, LMDB):**
- **transitive dependants per probe:** LMDB 0.37 ms; redb 0.49 ms; SQLite 0.64 ms native, 0.61 ms SQL;
- **count by kind:** LMDB 51 ms, redb 65 ms; SQLite 106 ms native, 57 ms SQL;
- **unreferenced definitions:** LMDB 99 ms, redb 168 ms; SQLite 310 ms native, 32 ms SQL.

**What this says for the backend choice:**
- **The query language no longer decides it.** LMDB, with no query language of its own, runs Sophia's queries fastest of the native backends.
- **SQLite has one advantage left: a second, independent executor.** Its SQL engine wins set-at-a-time workloads by 2–9× today, and it cross-checks the native executor in the tests. Most of that gap is in Sophia's executor (tuple-at-a-time, no prefix access path), not in the storage. LMDB's native count by kind is already about equal to SQLite's SQL.
- **Goal-directed evaluation matters more than storage:** 4.5 orders of magnitude, against 1.3× between the best and worst primary candidates.
- **Still open for the decision ([[S-ADR-0010 Primary backend and an analytics replica]]):**
  - B-15, edit latency through Moonkale;
  - B-16, real code;
  - LMDB's single writer under concurrent agents (SR-104).

### 2026-10-20 (later): all four candidates (M2)

All four pass the conformance suite and the interface acceptance tests. Full tables are in [[Benchmark Results]].

| | SQLite | redb | LMDB | DuckDB |
|---|---|---|---|---|
| ingest at 10⁵ (defs/s) | 7 700 | 10 300 | **17 600** | ~100 (10⁴) |
| record by id at 10⁵ | 4.9 µs | 1.3 µs | **0.9 µs** | ~200 µs (10⁴) |
| closure of 5 713 definitions | 26.9 ms | 9.2 ms | **8.0 ms** | — |
| bytes per definition at 10⁵ | **6 800** | 10 800 | 8 600 | 13 800 (10⁴) |
| analytics (join, scan) | 12–90 ms | — | — | **4 ms** |
| query language | SQL, view-restricted guard | none | none | SQL, structural guard |
| build | C, bundled | pure Rust | C (small), bundled | C++, large bundled build |
| limits found | — | file grows without compaction | 511-byte keys; fixed map size; one writer; no empty keys | reserved words (`by`, `at`); no negative `LIMIT`; slow small transactions |

**Recommendation** ([[S-ADR-0010 Primary backend and an analytics replica]], since accepted in an amended form):
- SQLite stays the default for Moonkale now. It is queryable, the smallest, and already shipped.
- LMDB is the performance candidate, to be re-measured on real code (B-16) and on edit latency (B-15) before any switch.
- DuckDB becomes a bulk-loaded analytics replica, never the primary store.

**Bugs the second pair of backends exposed:**
- LMDB rejects a range scan from the empty key.
- DuckDB rejects `LIMIT -1`, which SQLite silently accepted as "no limit".

Both are fixed in the backends, and the conformance suite now lists without a limit from the empty prefix on every backend, so the same mistake cannot recur.

**Lesson recorded:** the first SQLite layout (`WITHOUT ROWID` for multi-KB rows) was 4× slower than the corrected one. A backend comparison is only as good as each backend's layout, so each layout choice is written down in its file note ([[sqlite]], [[redb_store]]).
