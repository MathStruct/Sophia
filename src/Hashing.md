# Hashing

Julia-side hashing and the cross-implementation conformance suite. Source: `Hashing.jl` (comments only).

Full design: [[Hashing and Identity]]. Rust counterpart: [[sophia_hash]], [[canonical]], [[merkle]].

## The contract is equality with Rust

`hash_term`, `hash_type` and `hash_erased` must produce byte-identical digests to the Rust implementation. If the two ever disagree on a single term, one of them is wrong about the *specification* — and the specification is what every node in every store depends on.

This is the payoff of the two-language repository: an independent second implementation of the most safety-critical component, written by someone thinking in a different language, with a test that compares them on a large corpus.

## What the corpus must contain

The interesting entries are the negative ones — pairs that must **not** collide:

- terms differing only in a primitive attribute, especially `Wrap` vs `Poison` overflow and `contract` on/off ([[Cross-Language Semantic Hazards]])
- `0.0` vs `-0.0`, and distinct NaN payloads (float attributes hash as bit patterns)

and the structurally awkward ones:

- alpha-variants, which must collide ([[Alpha Equivalence]])
- mutually recursive groups of 2, 3 and 5 definitions ([[Cycle Hashing]])
- a *symmetric* SCC, where colour refinement cannot separate members and the lexicographic tie-break has to fire — the case most likely to differ between two implementations

## The property test people forget

Determinism and α-invariance get written. **Sensitivity** — perturbing a semantically relevant attribute must change the digest — usually does not, and it is the one that catches an over-eager [[canonical|canonicaliser]]. That failure has no other detector: it produces a wrong binary with no error anywhere ([[Trusted Computing Base]]).

## Cross-machine determinism

Run the corpus on x86-64 and aarch64, Linux and macOS, and compare. Endianness, float printing and hash-table iteration order are the three classic sources of divergence, and the encoding rules in [[sophia_hash]] are written specifically to avoid all three.

## Related

- [[Hashing and Identity]] · [[sophia_hash]] · [[canonical]] · [[merkle]]
- [[BLAKE3]] · [[Hash Consing]] · [[UUID]] · [[Cycle Hashing]]
- [[CoreIR]] · [[Trusted Computing Base]]
