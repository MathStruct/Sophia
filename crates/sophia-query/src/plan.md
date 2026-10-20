#implementation

> The greedy literal-ordering planner, shared by the magic-set rewrite and the executors. Source: `plan.rs`.
>
> Sources: [[SPEC-007 Sophia Query Language]] §6.1. code: `plan.rs`

- `access_columns` says which columns of each base relation have an access path on every backend (for example `refs`: both columns, through `refs_of` and `dependants`).
- `order` schedules filters as soon as their variables are bound, then prefers base literals that can use an access path, then local literals with bound arguments, and full scans last. Variables that occur only inside one negated literal are existential.
- `wide` marks lookups that return a large share of a relation: by kind, and by collection. They rank after local literals (SPEC-007 §6.1, measured on set.mm).
- `check_safe` uses `order` to report a variable that no literal binds.
