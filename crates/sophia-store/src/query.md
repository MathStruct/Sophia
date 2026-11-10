#implementation

> Results and limits of read-only text queries. Source: `query.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §11; [[SPEC-004 Moonkale Interface]] §5. code: `query.rs`

`Table` (columns, rows of `Value`, `truncated`), `QueryLimits` (row cap, step budget, byte budget), and `VIEWS`, the names of the views a query may read.

**The byte budget** (Fable's review, 2026-11-10, [[Review 2026-11-10 Fable (external)]], S-3): `max_bytes` (default 64 MiB) cuts a result when its values pass it, and sets `truncated`. `max_value_bytes`, at most 16 MiB, bounds each single value the backend computes. `Value::size` is a text's or blob's length, or 8 bytes otherwise.
