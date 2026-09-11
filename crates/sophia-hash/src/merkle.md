# merkle

Merkle hashing over canonical terms. Source: `merkle.rs` (comments only).

## The easy half

For a DAG, the [[Merkle DAG]] construction is three lines: hash the tag, the attributes and the children's hashes, with length-prefixed concatenation and a per-tag [[BLAKE3]] key. Memoising on the interned pointer ([[Hash Consing]]) makes a term that occurs `n` times cost one hash.

## The hard half: mutual recursion

Merkle hashing is defined by recursion on children, so it is well-founded only on acyclic structure. `isEven` calling `isOdd` calling `isEven` has no bottom. Every content-addressed code system meets this; the construction is in [[Cycle Hashing]] and is implemented here:

Tarjan SCCs → hash the condensation (which *is* acyclic) → inside each SCC, order members by iterative colour refinement seeded from external dependency hashes → break residual ties lexicographically and record the choice → derive each member's hash from the SCC hash plus its canonical index.

The uncomfortable part is the tie-break: canonical labelling of a general graph is not known to be polynomial. In practice SCCs in real code are 2–5 definitions and the first refinement round separates them. The engineering response is a **hard cap on SCC size and on refinement rounds, with a loud error above it** — an adversarial input should fail, not hang.

## Erasure and `h_run`

`hash_erased` reruns the construction on a term stripped of runtime-irrelevant structure: types in erasable (0-quantity, cf. [[Linear and Affine Types|QTT]]) positions, proof terms from the `Prf` fragment, phantom parameters. Definitions differing only in erased content then share compiled code, which is the mechanism behind [[Content-Addressed Precompilation]].

The erasure function carries the same risk as [[canonical]]: erasing something that *is* runtime-relevant silently unifies two different programs.

## Edges get hashed too

`hash_edge` exists so a [[Equivalence and Witnesses|witness]] can cite the specific `EQUIV` edge it justifies. An unidentified edge cannot be referred to, and a claim that cannot be referred to cannot be checked. See [[Graph Schema]].

## Related

- [[Merkle DAG]] · [[Cycle Hashing]] · [[BLAKE3]] · [[Hash Consing]]
- [[sophia_hash]] · [[canonical]]
- [[Hashing and Identity]] · [[Content-Addressed Precompilation]]
