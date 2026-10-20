#open-problem

> **Status: partly closed (2026-10-20).** Rule and predicate order are now normalised **exactly**: a minimum over all traversals, with every tie tried, within a budget, and refused beyond it (SPEC-007 §4.1; property-tested). Literal order remains part of the identity, by design. Related: [[SPEC-007 Sophia Query Language]] §4.1, `crates/sophia-query/src/canon.md`.

**Problem.** A query's meaning does not depend on:
- the order of literals in a body (conjunction commutes);
- the order of rules (union commutes);
- the names of predicates and variables.

The canonical form removes names, predicate order, rule order and duplicates, but **not literal order**. (Until the M3a review, rule order was settled by at most eight rounds of renumbering and sorting, which did not guarantee invariance. That is fixed.) So `def(x, :method) && refs(x, _)` and `refs(x, _) && def(x, :method)` have different ids.

**Why not normalise it.** A complete canonical form of a conjunctive query is graph canonisation: the body is a hypergraph over variables. Sorting literals depends on variable numbering, and variable numbering depends on literal order. Rule order had the same circularity (rule bytes depend on predicate numbering, which depends on rule order). It is now solved exactly, by searching every tie (SPEC-007 §4.1). The same search over literal orders would multiply by the factorial of each body's length, which is why literal order is kept.

**What v0 does.** Literal order is part of the identity, like argument order in code. Equivalent queries with different ids are related by **equivalence facts**, exactly as equivalent programs are (SPEC-006, [[Equivalence and Witnesses]]).

**Possible fix.**
- Canonical labelling of the body hypergraph (nauty or bliss style, or Weisfeiler–Leman refinement with a tie-break), applied to bodies, which are small.
- Or the quotient technique of `sophia-hash::component` (automorphic members as one class), applied to literals.

Worth doing if real use shows many reordered duplicates.
