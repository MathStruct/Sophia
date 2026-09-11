# BLAKE3

BLAKE3 is a cryptographic hash function (2020, successor to BLAKE2) with three properties that make it a good default for content-addressed code storage:

- **Speed.** Several GB/s per core, and much faster than SHA-256 on short inputs — which is the relevant case here, since a code store hashes an enormous number of small nodes.
- **Internal tree structure.** The compression is a Merkle tree over 1 KiB chunks, so hashing parallelises and large blobs support verified streaming. It composes naturally with an outer [[Merkle DAG]].
- **Keyed mode and key derivation.** `derive_key(context)` produces a distinct hash function per context string, which gives *domain separation* for free: hashing a `Term` node under a different key than an `Op` node makes cross-kind collisions structurally impossible rather than merely improbable.

Output is 256 bits by default and is an extendable-output function (XOF), so shorter identifiers can be taken without a separate construction.

Collision probability for `n` distinct objects is approximately $n^2 slash 2^257$. At $n = 2^40$ this is about $2^(-177)$ — not a quantity that requires engineering attention. Truncating to 128 bits for a [[UUID]]-shaped display identifier gives roughly $2^(-49)$ at the same `n`, which is why truncation is for display only and the full digest is what gets stored.

## Related

- [[Merkle DAG]]
- [[Hash Consing]]
- [[UUID]]
- [[Hashing and Identity]]
