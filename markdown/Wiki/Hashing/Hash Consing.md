# Hash Consing

Hash consing is the technique of maintaining a table of every structure already allocated, so that constructing a value that already exists returns a pointer to the existing one instead of a new copy. The result is **maximal sharing**: structurally identical subterms are physically identical.

Two properties follow, and both matter for a compiler-scale term representation:

- **Structural equality becomes pointer equality** — an $O(1)$ test instead of an $O(n)$ traversal. For a system whose conversion checker compares terms constantly, this is not a micro-optimisation.
- **Memory is proportional to the number of *distinct* subterms**, which for real programs is far smaller than the number of subterm *occurrences*.

Hash consing is the in-memory analogue of [[Content-Addressed Code|content addressing]], and the two compose naturally: the persistent store is keyed by a cryptographic hash ([[BLAKE3]]), while the in-process intern table is keyed by a fast non-cryptographic hash (`xxhash`) plus a structural equality check on collision. The intern table is never persisted and never trusted; it only has to be fast.

The costs are a global table (which interacts badly with concurrency and with garbage collection) and the hashing itself on every construction.

## Related

- [[Merkle DAG]]
- [[Content-Addressed Code]]
- [[BLAKE3]]
- [[E-Graph]]
