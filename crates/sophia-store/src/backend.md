#implementation

> The interface every storage backend implements. Source: `backend.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §12. code: `backend.rs`

`Backend::read` and `Backend::write` run a closure inside a transaction: `write` commits on `Ok` and rolls back on `Err`. The closure receives `&dyn ReadTx` or `&mut dyn WriteTx`, both object-safe sets of typed operations:
- records, components, dependants;
- names;
- the log;
- nominal entities and relations;
- lineages;
- annotations, facts;
- meta.

**Why typed operations rather than a raw key-value interface:** SQLite can then keep real tables, which the read-only `sql` views need, while a key-value store keeps byte tables. The rules live once, in [[logic]].

**Why closures:** a transaction's lifetime is tied to the backend's own transaction type. Closures keep that out of the public types, which Moonkale's adapter needs to be free of driver types (SPEC-004).
