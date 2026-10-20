#implementation

> Static checks of a query: arity, sorts by unification, safety, stratification. Source: `check.rs`.
>
> Sources: [[SPEC-007 Sophia Query Language]] §3.3, §4.2. code: `check.rs`

- Sorts: union-find over parameters, predicate columns, variables and constants. A conflict names both sides ("column 2 of `def` is Text but constant 3 is Int").
- Safety: every head and output variable is bound by a positive literal; every literal can be scheduled ([[plan]]).
- Stratification: strongly connected components of the predicate graph (Tarjan from `sophia-hash`); a negative edge inside a component is refused.
- The result `Checked` holds the output sorts: the query's result type.
