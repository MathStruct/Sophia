#implementation

> The store over any ordered byte-to-byte key-value engine: the tables, their key layouts, and the `ReadTx`/`WriteTx` logic, written once for [[redb_store]] and [[lmdb_store]]. Source: `kv.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §12. code: `kv.rs`

An engine implements two small traits:
- `Kv`: `get`, a prefix `scan` from a start key, `last_key`, `len`;
- `KvWrite`: `put`, `del`.

Blanket implementations then give `ReadTx` and `WriteTx`.

The sixteen `Table`s and their key layouts:
- `component ‖ index ‖ id`;
- `dst ‖ src` for dependants;
- `target ‖ path` for names;
- `lineage ‖ id` and `id ‖ lineage`;
- `subject ‖ id` for annotations and facts;
- `kind ‖ 0 ‖ id`;
- the log, keyed by big-endian seq.

Values use [[codec]]. The layouts were written for redb and moved here unchanged when LMDB arrived, so the two engines are compared on identical layouts.
