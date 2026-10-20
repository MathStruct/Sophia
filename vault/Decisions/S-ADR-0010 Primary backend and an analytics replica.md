#design

> **Status: accepted in an amended form (2026-10-20).** It was proposed by Claude after the four-backend comparison. Daniel then forwarded a review ([[Review 2026-10-20 Query Language (external)]]) whose recommendation he sent on as his direction: *"I would not decide between SQLite and LMDB yet. Keep SQLite as the default, but implement and benchmark LMDB as an experimental backend."* This ADR now records that. Daniel can overrule it with one line. Related: [[S-ADR-0005 SQLite first behind a store trait]], [[S-ADR-0011 Sophia has its own query algebra]], [[Database Comparison]], [[Benchmark Results]], SR-64, SR-66, SR-104.

## Context

All four candidates (SQLite, redb, LMDB, DuckDB) are implemented behind one `Backend` trait and pass the same conformance suite. Measured on synthetic code at 10⁵ definitions:
- LMDB is fastest at ingest, point lookups and closure walks;
- SQLite is smallest, and the only primary candidate with a view-restricted SQL guard;
- redb is pure Rust;
- DuckDB is about 100× slower on small transactions, but 3–20× faster on analytical queries.

The original worry about LMDB was that Moonkale would lose its query language on such a store. **[[S-ADR-0011 Sophia has its own query algebra]] removes that worry:** Sophia's own query language runs on every backend, LMDB included (B-17).

## Decision

1. **SQLite stays the default** for now.
2. **LMDB stays an experimental backend.** It is implemented and passes every suite, including the query suite. It is benchmarked, but it is not the default.
3. **No permanent choice yet.** The primary backend is chosen after:
   - B-15 (edit-to-graph latency through Moonkale);
   - B-16 (real code);
   - B-17 (query workloads, now measured).
4. **LMDB's single writer** is measured under concurrent agents before LMDB serves a multi-agent server (SR-104). It is probably fine for desktop Moonkale.
5. **DuckDB as an analytics replica** stays an option for statistics (SR-66). It is not built now: more backends are not the priority (SR-105).

## Consequences

- The `sql` dialect stays SQLite-only (and DuckDB). The `query` dialect works everywhere. Moonkale should prefer `query`.
- The backend choice no longer constrains the query language. It is now a pure performance and operations question.
