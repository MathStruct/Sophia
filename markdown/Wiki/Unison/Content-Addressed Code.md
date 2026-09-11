# Content-Addressed Code

Content-addressed code means a definition's identity is a cryptographic hash computed from its own normalized syntax tree (with references to *other* definitions folded in by their hashes, not their names), rather than a human-chosen name or file location. Two definitions that are structurally identical get the same hash automatically; renaming a function doesn't change its hash or require recompiling anything that depends on it, because dependents reference the hash, not the name.

This is the same move [[Start Here]] proposes: "calculate unique reproducible UUIDs from the tree of dependencies." It solves several problems at once — there is no naming/dependency-resolution step separate from identity, caching and incremental compilation become trivial (same hash ⇒ already compiled), and it gives a natural key for storing code as nodes in a graph database.

It does *not* by itself solve semantic equivalence: two definitions that compute the same thing via different syntax get different hashes. That's a separate problem, addressed by explicit equivalency annotations/proofs (see [[Start Here]]) or by normalization/canonicalization before hashing, as in [[State of the Art - Equality Saturation and E-Graphs]].

## Related

- [[Unison]]
- [[Unison Codebase]]
- [[State of the Art - Content-Addressable Code Systems]]
- [[State of the Art - Equality Saturation and E-Graphs]]
