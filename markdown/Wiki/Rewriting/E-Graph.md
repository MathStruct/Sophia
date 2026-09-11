# E-Graph

An e-graph is a data structure that represents a *set* of equivalent expressions compactly. It consists of:

- **e-nodes** — an operator together with a list of child **e-classes** (not child e-nodes),
- **e-classes** — equivalence classes of e-nodes, maintained in a union-find.

Because an e-node's children are e-classes, a single e-graph of modest size can represent exponentially many equivalent terms. Merging two e-classes asserts that everything in one is equivalent to everything in the other, and *congruence closure* then propagates the consequence: if `a ≡ b`, then `f(a) ≡ f(b)` for every `f` already in the graph, automatically.

E-graphs originated in automated theorem proving (congruence closure for decision procedures, Nelson–Oppen) and were rediscovered for compilers as the substrate of [[Equality Saturation]]. `egg` is the standard modern implementation, in Rust; `egglog` recasts the same idea as [[Datalog]] over a relational store.

For [[Start Here]]'s "insert equivalency proofs for two distinct UUIDs", the e-graph is the natural in-memory form of the [[Equivalence and Witnesses|EQUIV]] relation: an e-class *is* an equivalence class of hashes, congruence closure *is* the requirement that equivalence be a congruence, and proof extraction produces the rewrite-chain witness.

Note the structural similarity to [[Hash Consing]]: both intern terms and make equality cheap. Hash consing gives one canonical representative per *structurally identical* term; an e-graph gives one representative class per *semantically equivalent* set.

## Related

- [[Equality Saturation]]
- [[Term Rewriting System]]
- [[Hash Consing]]
- [[Equivalence and Witnesses]]
- [[State of the Art - Equality Saturation and E-Graphs]]
