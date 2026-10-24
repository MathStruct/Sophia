#implementation

> The SQLite backend. Source: `sqlite.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §11, §12; [[SPEC-004 Moonkale Interface]] §5; SQLite documentation, *The WITHOUT ROWID Optimization* (§4: when to use it). code: `sqlite.rs`

## Layout

Tables are prefixed `t_`. Views are named as in SPEC-001 §11 (`definitions`, `names`, `refs`, `nominals`, `relation`, `lineage`, `annotations`, `facts`, `log`) and expose ids as lowercase hex.

The connection uses WAL mode with `synchronous = NORMAL`. Writes take an `IMMEDIATE` transaction, and reads run in a deferred transaction that is rolled back.

## The read-only guard

`query_readonly(sql, limits)` enforces read-only access; it does not rely on the caller:
- an **authorizer** allows only `SELECT`, reads of the views (and of tables only when read *through* a view), and functions other than `load_extension`, `readfile`, `writefile` and `edit`;
- `stmt.readonly()` is checked;
- a **progress handler** aborts after `max_steps_k` thousand VM steps;
- rows past `max_rows` set `truncated`.

The test tries `DELETE`, a direct table read, `INSERT`, `PRAGMA`, `ATTACH`, `CREATE` and a runaway recursive query, and the store stays usable after each.

## Difficulties met (2026-10-20)

- **`WITHOUT ROWID` was wrong for definitions.** Rows are a few KB, and SQLite's documentation says `WITHOUT ROWID` suits rows smaller than about 1/20 of a page. With it, lookup by id took 10–18 µs. With a rowid table and a unique index on `id`, it takes 2–4.5 µs (4×), and closure walks are 5× faster ([[Benchmark Results]]). The small tables keep `WITHOUT ROWID`.
- **Three queries per record** (the row, then its references, then its nominal references) also cost time. The reference lists are now stored on the row as blobs, and `t_ref` is kept only for reverse lookups (dependants).
- `rusqlite` does not read `u64` columns, so they are read as `i64` and cast. Sequence numbers stay far below 2⁶³.

**`query_compiled`** (2026-10-20): runs SQL that `sophia-query` generated from a checked query. The guard is the same as for `query_readonly` (only `SELECT`, no writes, a step budget), except that it may read the base tables `t_*`, so their indexes apply. The views would hide them behind `lower(hex(…))`. It is never given text from users or agents. SQLite also overrides the access paths `kind_of`, `refs_of`, `all_kinds` and `all_refs` with direct index reads.

**`Sqlite::from_bytes(bytes)`** (2026-10-24, SPEC-009 §4): a store opened read-only from the bytes of its file, in memory, for Moonkale's web client. Three things were needed to make it work under wasm, found by running `tests/wasm_read.rs`:
- a WAL-mode file (Sophia's are) makes SQLite open `-wal` and `-shm` files even to read, which fails in memory with "unable to open database file"; the copy's header bytes 18 and 19 are set from 2 (WAL) to 1 (rollback journal), which is safe for a complete file;
- on wasm, `Connection::open_in_memory` does not open under `sqlite-wasm-rs`; a named database in its default in-memory VFS does;
- `PRAGMA temp_store = MEMORY`, since there is no file system for temporary tables.

**Prefix reads seek the index (2026-10-24, Moonkale P-232, found by Moonkale's Codex in a profile).** Two prefix reads scanned their tables:
- `meta_prefix` compared `substr(key, 1, length(?1))`. It now reads the key range from the prefix to `prefix_successor(prefix)`, the least string above every extension: the last character that can be raised is raised, surrogates are skipped, and there is no upper bound for the empty prefix.
- `names_under` used one predicate with `OR`. It now makes separate seeks for the root, the exact name, and the range `prefix.` … `prefix/`.

`prefix_reads_use_the_index` checks with `EXPLAIN QUERY PLAN` that the statements `SEARCH` and never `SCAN`. The old forms show `SCAN t_meta` and `SCAN t_name` on Moonkale's store. `prefix_reads_agree` (conformance, all four backends) covers namespace boundaries, quoted segments, `é` and U+10FFFF keys, and limits.

On a collected copy of Moonkale's store (`listing_timing … rs 5`, release), `Interface::all(100000)` goes from 274 ms to 120 ms, with the same output digest; it was 2089 ms before the namespace-edge fix. `children(ns:)` goes from 1.1 to 0.5 ms.
