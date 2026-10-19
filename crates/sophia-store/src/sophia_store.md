#implementation

> Crate root of `sophia-store`: the store for definitions of any language, with interchangeable backends. Source: `sophia_store.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]]; [[S-ADR-0005 SQLite first behind a store trait]]; [[Database Comparison]]. code: `sophia_store.rs`, [[backend]], [[logic]], [[types]], [[codec]], [[query]], [[sqlite]], [[redb_store]], [[error]]

## Layout

- **[[backend]]**: the `ReadTx`/`WriteTx`/`Backend` traits, which are typed operations inside transactions.
- **[[logic]]**: `Store<B>`, the store's rules, written once against those traits: sequence numbers, names, lineages, nominal entities and method tables, annotations, facts and staleness.
- **Backends**, behind features: [[sqlite]] (default; real tables, views and the query guard) and [[redb_store]] (pure-Rust key-value store).

## Tests

`tests/conformance.rs` runs the behavioural tests on **every** backend (SPEC-001 §13): 14 core tests, settings, world snapshots, and the SQL guard test. `tests/symmetric.rs` is the regression test for symmetric recursive groups. Adding a backend means adding one line to each.

## What is not here yet

Rendering profiles (SPEC-005, M2), the `datalog` dialect (M3), branches, gc, migration (SPEC-001 §4.5), DuckDB and LMDB backends (M2).
