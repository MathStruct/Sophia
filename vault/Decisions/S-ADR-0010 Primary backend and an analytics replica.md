#design

> **Status: proposed (2026-10-20, Claude), after the four-backend comparison.** Related: [[S-ADR-0005 SQLite first behind a store trait]], [[Database Comparison]], [[Benchmark Results]], SR-64, SR-66.

## Context

All four candidates (SQLite, redb, LMDB, DuckDB) are implemented behind one `Backend` trait and pass the same conformance suite. Measured on synthetic code at 10⁵ definitions:
- LMDB is fastest at ingest, point lookups and closure walks;
- SQLite is smallest and the only primary candidate with a view-restricted SQL guard;
- redb is pure Rust;
- DuckDB is about 100× slower on small transactions, but 3–20× faster on analytical queries.

## Options

1. **SQLite primary**: what Moonkale ships, queryable, small.
2. **LMDB primary**: the fastest. But it has no query language (Moonkale would lose the `sql` dialect on such a store), 511-byte keys, and a fixed map size.
3. **Either primary, plus a DuckDB analytics replica**, loaded in bulk from the log, for statistics such as SR-66's precompilation analytics.

## Proposal

- **SQLite primary now, plus option 3 for analytics later.**
- Re-decide the primary backend after B-15 (edit-to-graph latency through Moonkale) and B-16 (real code). If LMDB's advantage holds there and matters to the user, LMDB can become the primary store, with the `sql` dialect served from the replica.

## Needs from Daniel

Agreement, or a preference for performance (LMDB) over queryability (SQLite) now.
