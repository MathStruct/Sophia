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
