#implementation

> The crate root of `sophia-query`, Sophia's own query language. Source: `sophia_query.rs`.
>
> Sources: [[SPEC-007 Sophia Query Language]]; [[S-ADR-0011 Sophia has its own query algebra]]; [[S-ADR-0012 The query IR is a separate IR on shared primitives]]. code: `sophia_query.rs`

- `prepare_text` and `prepare`: parse, make canonical ([[canon]]), check ([[check]]), and compute the magic-set rewrite ([[magic]]). The result is a `Prepared` query with its content id.
- `Prepared::run`: the native executor ([[eval]]) inside one read transaction, on any backend. `Strategy::Auto` runs the magic-set program when there is one; `Strategy::Plain` runs the query as written.
- `Prepared::to_sql` and `run_sql`: the SQL translation ([[sql]]) and its execution on SQLite (`Sqlite::query_compiled`).
- `Prepared::record` and `from_record`, `save` and `load`: queries stored as definitions of the language `sophia-query`.
- `snapshot`, `run_cached`: result identity (query id, snapshot, parameters) and a result cache in meta rows, which are not logged, so caching does not move the snapshot. The snapshot id, the lookup and the evaluation share one read transaction. Only complete results are cached, and the caller's row cap is applied to hits.
- `run_cancellable`: a run another thread can stop.
- `Rows`: typed result rows; `to_table` turns them into the store's `Table`, with ids as hex text like the `sql` views.
