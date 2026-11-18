#implementation

> The crate root of `sophia-query`, Sophia's own query language. Source: `sophia_query.rs`.
>
> Sources: [[SPEC-007 Sophia Query Language]]; [[S-ADR-0011 Sophia has its own query algebra]]; [[S-ADR-0012 The query IR is a separate IR on shared primitives]]. code: `sophia_query.rs`

- `prepare_text` and `prepare`: parse, make canonical ([[canon]]), check ([[check]]), and compute the magic-set rewrite ([[magic]]). The result is a `Prepared` query with its content id.
- `Prepared::run`: the native executor ([[eval]]) inside one read transaction, on any backend. `Strategy::Auto` runs the magic-set program when there is one; `Strategy::Plain` runs the query as written.
- `Prepared::to_sql` and `run_sql`: the SQL translation ([[sql]]) and its execution on SQLite (`Sqlite::query_compiled`).
- `Prepared::record` and `from_record`, `save` and `load`: queries stored as definitions of the language `sophia-query`.
- `Prepared::reads`, `snapshot`, `run_cached`: result identity (query id, snapshot, parameters) and a result cache in meta rows, which are not logged, so caching does not move the snapshot. **The snapshot covers only what the query reads** (SP-010, step 1, B-016): `reads` is the set of base relations its rules and select mention, plus `def` and `nominal` when it reads `annotation` or `fact` (their full scans enumerate subjects through those two). The snapshot (`snapshot-v1`) hashes the store id and the epoch of each relation in that set (`sophia_store::logic::relation_epoch`), so a logged change to another relation keeps the result. The snapshot id, the lookup and the evaluation share one read transaction. Only complete results are cached, and the caller's row cap is applied to hits. Entries are keyed `query-result/<read set>/<snapshot>/<result id>`, the read set as two hex digits (`read_set_tag`, bit i for `Base::ALL[i]`). A miss that writes removes, in the same write transaction, every entry whose snapshot is not the current one for its read set, and every key in another layout, so only current results stay (SP-014, option 2). What it does not do: it does not track which keys a run looked up (SP-010 step 3), and the snapshot is not content-addressed (step 2), so two stores in the same state still give different snapshots.
- `run_cancellable`: a run another thread can stop.
- `bounded` (Unix; the bound itself Linux only): `run_bounded` runs a query in a worker process under a hard memory bound, for untrusted queries ([[bounded]], B-017). `Error::MemoryBound` and `Error::Worker` are its errors.
- `Rows`: typed result rows; `to_table` turns them into the store's `Table`, with ids as hex text like the `sql` views.
