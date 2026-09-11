# Equality Saturation

Equality saturation replaces a compiler's ordered pass pipeline with a two-phase process over an [[E-Graph|e-graph]]:

1. **Saturate.** Repeatedly match every rewrite rule against the e-graph and, on each match, *add* the equivalence rather than performing the rewrite. Nothing is destroyed. Continue until no rule produces a new equivalence, or a node/time budget is exhausted.
2. **Extract.** Choose one representative per e-class so as to minimise a cost model.

The point is that it dissolves the **phase-ordering problem**. A traditional pipeline must decide whether to inline before or after constant folding, and either choice loses opportunities the other would have found ([[LLVM Pass]]). Saturation applies both and defers the choice to extraction, where global cost information is available.

Extraction is the hard half. As an integer program with $x_n = 1$ iff e-node `n` is chosen:

$ min sum_n c(n) x_n quad "s.t." quad ∀ "class" C: sum_(n ∈ C) x_n ≥ 1, quad x_n ≤ x_m ∀ m ∈ "children"(n) $

plus an acyclicity constraint on the extracted DAG, which is what makes the problem NP-hard in general. Greedy bottom-up extraction is the practical default, with ILP reserved for small hot regions.

The other practical limit is **e-graph explosion**: associativity and commutativity rules in particular generate enormous numbers of e-nodes, so real systems saturate under strict budgets rather than to true fixpoint.

## Related

- [[E-Graph]]
- [[Term Rewriting System]]
- [[Confluence and Termination]]
- [[Compilation as Query]]
- [[State of the Art - Equality Saturation and E-Graphs]]
