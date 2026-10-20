#implementation

> The redb backend: a pure-Rust copy-on-write B-tree key-value store. Source: `redb_store.rs`.
>
> Sources: redb (github.com/cberner/redb), version 4.4; [[Database Comparison]]. code: `redb_store.rs`

## Layout

Sixteen byte-to-byte tables. Values are encoded with [[codec]]. Secondary indexes are key-only tables whose keys are concatenations:
- `component ‖ index ‖ id`;
- `dst ‖ src` for dependants;
- `target ‖ path`;
- `lineage ‖ id` and `id ‖ lineage`;
- `subject ‖ id`;
- `kind ‖ 0 ‖ id`.

The log is keyed by big-endian `seq`, so a range scan is in order.

Since 2026-10-20 the tables and logic live in [[kv]], which is shared with LMDB. This file only adapts redb's read and write transactions. `Redb::compact` reclaims space for a fair size comparison.

## Findings (2026-10-20, [[Benchmark Results]])

- Fastest reads: ~1–1.4 µs per record by id, and closure walks 3× faster than SQLite.
- Fastest ingest.
- **Largest files:** about 10–17 KB per definition against SQLite's 6.8 KB. redb does not compact on its own (copy-on-write pages). `Database::compact` should be measured before concluding anything.
- No query language: the `sql` dialect is SQLite-only. Moonkale would see only structural queries on a redb store.
