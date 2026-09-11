# Contextual Equivalence

Two program fragments are contextually (observationally) equivalent if no surrounding program can tell them apart:

$ t ≃_"ctx" u quad ⟺ quad ∀ C[dot]. space (C[t] ⇓ ⟺ C[u] ⇓) $

where `C` ranges over all well-typed contexts and `⇓` is the chosen *observation* — usually termination, sometimes termination-with-a-particular-value, sometimes a trace of I/O.

This is the gold-standard notion of program equality, and the reason is structural: it is by definition the **coarsest congruence** that respects the observation. Coarsest means it identifies as much as possible without being wrong; congruence means it is preserved by every context, which is exactly what licenses substituting one for the other inside a larger program.

The catch is the universal quantification over all contexts, which makes direct proof infeasible. The two standard escapes are [[Logical Relations]] (build a type-indexed relation that is a congruence by construction) and [[Bisimulation]] (co-inductive, better suited to state and effects).

Three observations about the definition that matter for [[Equivalence and Witnesses|Sophia's use of it]]:

- **Changing the observation changes the relation.** If timing or allocation counts as observable, far fewer programs are equivalent. This is exactly what the `modulo` annotation on an `EQUIV` edge records: which observations were excluded.
- **Congruence is what makes it useful**, and it is precisely the property that `asserted` and `tested` equivalences lack.
- **Across two languages the definition does not even typecheck** — `C` would have to be a context in *which* language? That gap is what a cross-language relation `R` has to fill.

## Related

- [[Logical Relations]]
- [[Bisimulation]]
- [[Operational Semantics]]
- [[Equivalence and Witnesses]]
