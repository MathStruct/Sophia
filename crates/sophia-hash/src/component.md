#implementation #algorithm

> Recursive groups: strongly connected components, and the canonical order of a component's members. Source: `component.rs`.
>
> Sources: Tarjan, *Depth-First Search and Linear Graph Algorithms*, SIAM J. Comput. 1972; Moonkale Code Graph Schema, "WP-66 normalizer v1 → Recursive components" (the ordering rule, adopted unchanged); [[Adopting from Unison]] row 4 (Unison's different ordering). code: `component.rs`
>
> Theory: [[Theory of the v0 Store]] §1.

## `strongly_connected`

Iterative Tarjan, so deep reference chains cannot overflow the stack. It returns components **dependency-first**: a component comes after every component it references. That is the order in which digests can be computed, because a reference to an earlier component is written as that component's finished digest.

## `canonical_order`

For each member as a candidate root:
1. A first-visit traversal assigns indices (breadth-first over members, stream order within a member).
2. Every member is encoded with those indices, and the encodings are concatenated in index order.

The smallest serialisation wins. When several roots tie, each member gets its smallest index across them, so structurally indistinguishable (automorphic) members may share an index, and with it an id. That is correct, because such members really are interchangeable. All candidates share one byte `Budget`, so a large recursive group fails with an error instead of taking unbounded time.

## Difficulties

- The cost is quadratic in the component's size (n roots × n members). Real recursive groups are small (the synthetic corpus uses pairs), but a generated or macro-expanded group of thousands would hit the budget. The error is typed, and the definition cannot be stored. → a later problem note if it happens on real code.
- Unison orders cycles differently (by members' hashes with the cycle removed). Sophia follows Moonkale for consistency. Unison's own hashes are kept as an attribute when Unison code is ingested.
