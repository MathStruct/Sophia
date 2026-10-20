#implementation

> Translation of a query into SQL for SQLite: one recursive common table expression per local predicate, over the base tables. Source: `sql.rs`.
>
> Sources: [[SPEC-007 Sophia Query Language]] §7; SQLite, *The WITH Clause*. code: `sql.rs`

- Positive literals become `FROM` items, shared variables become equalities, negation becomes `NOT EXISTS`, `startswith` becomes `substr(…) = …`, `count` becomes `count(DISTINCT …)` with `GROUP BY`.
- Parameters and constants are inlined as literals (`X'…'` for ids).
- Mutual recursion and non-linear recursion return `Error::Unsupported`. The native executor still runs those queries: SQL is a target, not the definition.
- The generated SQL reads the base tables (`t_*`) so their indexes apply; it runs through `Sqlite::query_compiled`, which keeps the read-only authorizer and the step budget.
