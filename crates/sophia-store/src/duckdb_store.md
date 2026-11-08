#implementation

> The DuckDB backend: the SQLite backend's tables and views on DuckDB's columnar engine. Source: `duckdb_store.rs`.
>
> Sources: DuckDB (duckdb.org), `duckdb` crate 1.10506 with `bundled`; [[Database Comparison]]; [[S-ADR-0010 Primary backend and an analytics replica]]. code: `duckdb_store.rs`

## Shape

The file was derived from [[sqlite]] and changed where DuckDB differs:
- `$n` parameters;
- no `WITHOUT ROWID`;
- `starts_with`;
- `excluded.` in upserts;
- `octet_length`.

The schema and the views are otherwise the same, so the same `sql` queries run on both.

## The query guard is weaker than SQLite's

DuckDB has no authorizer. So:
- the text must be one statement;
- it runs as a subquery (`SELECT * FROM (…)`), which admits only queries;
- it runs inside a transaction that is always rolled back;
- external access (files, extensions) is switched off for the connection at open;
- a timer interrupts it after `max_steps_k` milliseconds.

It does **not** restrict queries to the documented views: tables can be read, read-only. The test tries `DELETE`, `INSERT`, two statements, `ATTACH`, `read_csv` and a runaway query.

## Findings (2026-10-20)

- **Reserved words:** `by` and `at` are column names in the SQLite schema, and keywords in DuckDB. `by` is quoted, and the log's `at` is stored as `logged_at` (the `log` view still calls it `at`).
- **`LIMIT -1` is an error** in DuckDB, while SQLite reads it as "no limit". "No limit" was passed as `usize::MAX as i64`, which is −1. It is now clamped in both backends.
- **About 100× slower than SQLite on Sophia's workload** of many small transactions (about 100 definitions/s ingest, about 200 µs lookups), and **3–20× faster on analytical joins and scans**. So DuckDB belongs as an analytics replica loaded in bulk (the Appender API), not as the primary store.
- **The bundled build compiles DuckDB's C++ engine** (minutes on first build). The backend is behind the `duckdb` feature and is not in Moonkale's default build.

**Access paths** (2026-10-20): DuckDB overrides `kind_of`, `refs_of`, `all_kinds` and `all_refs` with SQL. Without the scan overrides, Sophia's "count by kind" query made 10⁴ point queries and took 1.9 s; with them it takes 7.6 ms.

**Fixes from Gemini's review, 2026-11-07 ([[Review 2026-11-07 Gemini (external)]]); fixed 2026-11-08:**
- **`annotation(id)`** reads by primary key, as in [[sqlite]] (S-BUG-02).
- **`query_readonly` reads only the views** (§5.3). DuckDB has no authorizer, so `only_views` serialises the query with `json_serialize_sql` and walks the tree. The rules:
  - every table it names must be a documented view or one of its own CTEs, with no schema or catalog;
  - a CTE may not shadow a stored table;
  - table functions other than `range`, `generate_series` and `unnest` are refused (`query_table` and `query` would reach the tables).
  - This needs the `json` feature of the `duckdb` crate, added to the workspace dependency: about 3½ minutes more on a fresh build. `serde_json` reads the tree.
  - Test: `duckdb_queries_are_read_only_capped_and_timed` checks 11 refused and 4 allowed queries, and fails without the check.
- **The time budget waits on a `Condvar`** (S-PERF-01). It used to poll every 2 ms, and a query waited for the poll to see it was done. Measured with 500 queries of `SELECT path FROM names` on an empty store, release build, three runs each (a temporary test, deleted): **2.10–2.12 ms per query before, 0.67–0.70 ms after**, including the two extra queries of the view check.
