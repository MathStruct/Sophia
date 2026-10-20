#implementation

> The magic-set rewrite: an IR-to-IR optimisation that makes bottom-up evaluation goal-directed. Source: `magic.rs`.
>
> Sources: Bancilhon, Maier, Sagiv and Ullman, *Magic Sets and Other Strange Ways to Implement Logic Programs*, PODS 1986; Beeri and Ramakrishnan, *On the Power of Magic*, JLP 1991; [[SPEC-007 Sophia Query Language]] §6.2. code: `magic.rs`

- Each local predicate called with bound arguments gets an adorned copy (`dep_fb`) and a magic predicate (`magic_dep_fb`) that holds the bound values it is asked about. Sideways information passing follows the planner's order.
- Negated calls use the all-free adornment and no magic predicate; negations are dropped from magic-rule bodies (the magic set only grows, which is sound). The result is checked for stratification; if it fails, the query runs as written.
- The rewritten program is an ordinary query, so both executors run it. The tests check that it gives the same rows as the plain program on every backend and through SQL.
