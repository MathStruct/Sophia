# Merkle DAG

A Merkle DAG is a directed acyclic graph in which every node is identified by a hash of its own contents *together with the hashes of its children*:

$$ h(n) = H("tag"(n) ‖ "attrs"(n) ‖ h(c_1) ‖ … ‖ h(c_k)) $$

The consequences follow immediately from the definition: a node's hash transitively commits to its entire reachable subgraph, identical subgraphs are automatically shared (deduplication is free), and any change anywhere propagates to the hashes of all ancestors and *only* to those ancestors.

Git is the best-known instance (blobs, trees and commits form a Merkle DAG), and Nix, IPFS/IPLD, Bitcoin and [[Unison]] are others. [[Hashing and Identity]] applies the construction at the granularity of individual statements rather than files or packages.

The construction has two sharp edges. Concatenation must be **length-prefixed** or unambiguously delimited, or `("ab", "c")` and `("a", "bc")` collide. And it is only well-defined on a **DAG** — cycles have no well-founded hash, which is why mutual recursion needs [[Cycle Hashing]].

## Related

- [[Content-Addressed Code]]
- [[Cycle Hashing]]
- [[Hash Consing]]
- [[BLAKE3]]
- [[Hashing and Identity]]
