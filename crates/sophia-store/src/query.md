#implementation

> Results and limits of read-only text queries. Source: `query.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §11; [[SPEC-004 Moonkale Interface]] §5. code: `query.rs`

`Table` (columns, rows of `Value`, `truncated`), `QueryLimits` (row cap, step budget), and `VIEWS`, the names of the views a query may read.
