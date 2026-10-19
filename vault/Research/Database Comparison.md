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

**Lesson recorded:** the first SQLite layout (`WITHOUT ROWID` for multi-KB rows) was 4× slower than the corrected one. A backend comparison is only as good as each backend's layout, so each layout choice is written down in its file note ([[sqlite]], [[redb_store]]).
