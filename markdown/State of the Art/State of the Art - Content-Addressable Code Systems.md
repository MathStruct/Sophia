# State of the Art - Content-Addressable Code Systems

Systems that identify data (including code) by a hash of its own content rather than by a name or location:

- **[[Unison]]** — the most directly relevant precedent, explicitly named in [[Start Here]]. Every definition is stored keyed by a hash of its syntax tree with inline references to dependencies resolved by their hashes; see [[Content-Addressed Code]] and [[Unison Codebase]].
- **Nix / NixOS** — package builds are addressed by a hash of their full build recipe (and, with content-addressed derivations, increasingly by a hash of the build *output* itself), so identical builds are automatically deduplicated and irrelevant changes upstream don't force rebuilds downstream. Conceptually the same "reproducible UUID from the dependency tree" idea from [[Start Here]], applied to whole packages instead of individual statements.
- **Git** — every object (blob, tree, commit) is addressed by the SHA of its content; a tree object is addressed by a hash that depends on the hashes of everything it contains, which is precisely the "hash of the tree of dependencies" construction. Git is a working example of a content-addressed Merkle DAG store, just not one that understands code semantics.
- **IPFS / IPLD** — a content-addressed, hash-linked data model (Merkle DAGs, like Git generalized to arbitrary structured data) intended as a general substrate for content-addressed storage across many data types, including potentially code.
- **Dhall** — a configuration language whose imports can be pinned to a semantic hash of the imported expression, guaranteeing that an import can't silently change meaning; a smaller-scale, well-documented example of hash-pinning applied specifically to *typed program terms* rather than opaque files.

## The shared limitation

All of these hash *syntax* (or raw bytes), not *semantics*: alpha-renaming or trivial reformatting can already be normalized away by most of these systems, but two semantically-equivalent programs written differently (a loop vs. a fold, or the same algorithm in Julia vs. C++) get different hashes. That gap is exactly what [[Start Here]]'s "insert equivalency proofs for two distinct UUIDs" is meant to bridge, and is the same gap [[State of the Art - Equality Saturation and E-Graphs]] addresses from the compiler-optimization side.

## Related

- [[Unison]]
- [[Content-Addressed Code]]
- [[State of the Art - Equality Saturation and E-Graphs]]
- [[State of the Art - Graph Databases for Code]]
