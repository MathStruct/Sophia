#design

> **Status: proposed (2026-10-17, Claude).** Related: [[Requirements]] SR-17, SR-39; [[SPEC-001 Sophia Store v0]] §13; [[State of the Art - Graph Databases for Code]].

## Context

Daniel listed LadybugDB, HelixDB, TypeDB, FalkorDB, DuckDB and TursoDB. The earlier vault leaned towards DuckDB. Moonkale's needs (reply §3):
- an embedded store that builds on Linux, macOS and Windows;
- the server opens it, never the wasm client;
- it can be swapped behind Sophia's interface.

LadybugDB has no macOS or Windows build yet (Moonkale P-144). Moonkale's own state is in SQLite (ADR-0014).

## Options

| | For | Against |
|---|---|---|
| SQLite | embedded everywhere; already in Moonkale; transactional small writes; `SQLite.jl` | recursive queries only through `WITH RECURSIVE`; no graph language |
| DuckDB | embedded, fast analytics, good for bulk reads | weaker for many small transactional writes |
| LadybugDB (Cypher) | a real graph query language | Linux only for now |
| TursoDB | SQLite-compatible, with replication | young |
| HelixDB, FalkorDB, TypeDB | graph or typed models | server-shaped, or not embedded on every platform |

## Decision

SQLite for v0, behind `sophia_store::Store`. The canonical encoding, not the database, is the contract. A second backend (DuckDB for analytics, a graph database for Cypher) is added when a query needs it. Moonkale shows Sophia's own dialects (`sql` now, `datalog` later) whatever the backend is.

## Amendment (2026-10-20)

Daniel asked to try several databases and compare them (SR-64). SQLite stays the **default** until the comparison says otherwise. redb, DuckDB and LMDB are implemented behind the same `Backend` trait and benchmarked ([[Database Comparison]], [[Benchmark Plan]]).

## Consequences

- v0's "graph queries" are SQL views plus `WITH RECURSIVE`, and later Sophia's own Datalog.
- Benchmarks at 10⁵ to 10⁶ definitions decide whether a second backend is needed. They are not a reason to choose one now.
