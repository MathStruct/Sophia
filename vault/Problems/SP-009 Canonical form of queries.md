#open-problem

> **Status: open, accepted for v0 (2026-10-20).** Related: [[SPEC-007 Sophia Query Language]] §4.1, `crates/sophia-query/src/canon.md`.

**Problem.** A query's meaning does not depend on:
- the order of literals in a body (conjunction commutes);
- the order of rules (union commutes);
- the names of predicates and variables.

The canonical form removes names, predicate order, rule order and duplicates, but **not literal order**. So `def(x, :method) && refs(x, _)` and `refs(x, _) && def(x, :method)` have different ids.

**Why not normalise it.** A complete canonical form of a conjunctive query is graph canonisation: the body is a hypergraph over variables. Sorting literals depends on variable numbering, and variable numbering depends on literal order. The rule-order normalisation already iterates for this reason, and in symmetric cases it settles on the smallest of the forms it saw, which is deterministic but not provably canonical.

**What v0 does.** Literal order is part of the identity, like argument order in code. Equivalent queries with different ids are related by **equivalence facts**, exactly as equivalent programs are (SPEC-006, [[Equivalence and Witnesses]]).

**Possible fix.**
- Canonical labelling of the body hypergraph (nauty or bliss style, or Weisfeiler–Leman refinement with a tie-break), applied to bodies, which are small.
- Or the quotient technique of `sophia-hash::component` (automorphic members as one class), applied to literals.

Worth doing if real use shows many reordered duplicates.
