# sophia_equiv

Equivalence claims, their strength, and their checking. Source: `sophia_equiv.rs` (comments only).

Full design: [[Equivalence and Witnesses]].

## What this crate is

The home of [[Start Here]]'s central claim — that code from different languages can interoperate through inserted equivalence proofs rather than an FFI. It is also the crate most likely to produce a silently wrong program, so most of its design is about *refusing* things rather than enabling them.

## The free case comes first

If two definitions elaborate to the same core term they have the same hash and are the same node. No edge, no witness, no check. A good deal of cross-language sameness is of this kind. This crate only engages where the core terms genuinely differ, which is a much smaller problem than "prove Julia equals C++".

## Defaults are the design

`EquivPolicy` defaults to `min_level = Rewrite`, empty `allowed_modulo`, `max_chain = 3`. Everything weaker requires an explicit opt-in that taints the artifact's provenance ([[Trusted Computing Base]]). The reason is the **congruence rule**: substitution is only licensed when `t ≈ u ⟹ C[t] ≈ C[u]` for every context, and `Tested`/`Asserted` claims are not congruences ([[Contextual Equivalence]]). Two sort functions agreeing on every test may differ on stability.

## Composition degrades monotonically

Chaining claims takes the minimum level and the **union** of moduli. Chain a few and the claim says nothing. Three implementation consequences, all in the source: never materialise the transitive closure, keep shortest witness chains rather than mere reachability, and bound chain length by policy.

This degradation is not a flaw to engineer around — it is an accurate account of what chained approximate claims are worth.

## Cross-language claims need `R`

"`t₁` does the same as `t₂`" is not well-formed until someone supplies a relation between the two observation universes. Elaborating both sides into [[Core Calculus|SC]] makes this tractable by reducing it to one semantics, but the residual relation — Julia `Int64` ↔ C++ `int64_t` *only where no overflow occurs*, `String` ↔ `std::string` only under stated invariants, `Array` ↔ `vector` only with an ownership story — is a reviewed artifact and part of the [[Trusted Computing Base]]. It is a [[Logical Relations|cross-language logical relation]], which is at least a well-studied kind of object.

## Refinement is the more useful primitive

`a ⊑ b` — "a may replace b" — covers most real cases, which are asymmetric: fewer behaviours, total where the other was partial, checked where the other was unchecked. Alive2 checks refinement rather than equality for exactly this reason ([[State of the Art - Program Equivalence Checking]]), and Julia → Rust is refinement rather than equivalence ([[effects]]). Conflating the directions is a classic unsoundness.

## Submodules

- [[egraph]] — saturation and extraction
- [[witness]] — witness formats and their checkers

## Related

- [[Equivalence and Witnesses]] · [[Cross-Language Semantic Hazards]]
- [[Contextual Equivalence]] · [[Logical Relations]] · [[Bisimulation]]
- [[E-Graph]] · [[Equality Saturation]]
