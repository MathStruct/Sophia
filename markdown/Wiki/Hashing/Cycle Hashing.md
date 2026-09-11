# Cycle Hashing

[[Merkle DAG|Merkle hashing]] is defined by recursion on children, so it is well-founded only on an acyclic graph. Mutually recursive definitions — `isEven` calling `isOdd` calling `isEven` — form a cycle, and the recursion never bottoms out. Every content-addressed code system hits this; [[Unison]] does, and so does [[Hashing and Identity|Sophia]].

The standard construction has four steps:

1. **Find strongly connected components** (Tarjan). The *condensation* — the graph of SCCs — is acyclic, so ordinary Merkle hashing is well-defined on it.
2. **Hash each SCC as a unit**, with its external dependencies folded in by their (already-defined) hashes.
3. **Canonically order the members within the SCC** without reference to their names. Do this by iterative colour refinement: start each member with a colour derived from its node tag and its external dependency hashes, then repeatedly recolour by the multiset of neighbour colours until the partition is stable. This is 1-dimensional Weisfeiler–Leman / Paige–Tarjan partition refinement.
4. **Break remaining ties.** If refinement leaves a non-singleton colour class, its members are genuinely symmetric under the refinement; order them by the lexicographically least serialisation and record the choice.

Each member then receives $h_i = H(h_"scc" ‖ i)$ for its canonical index `i`.

Step 4 is where the theory is uncomfortable: canonical labelling of a general graph is not known to be in P, and is closely related to graph isomorphism. In practice this does not bite, because SCCs in real code are tiny (2–5 definitions) and colour refinement separates them immediately. It is still an adversarial input away from being expensive, so implementations should cap SCC size with a hard error rather than degrading quietly.

## Related

- [[Merkle DAG]]
- [[Hashing and Identity]]
- [[Content-Addressed Code]]
- [[Unison Codebase]]
