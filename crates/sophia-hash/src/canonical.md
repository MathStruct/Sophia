# canonical

Canonicalisation of Sophia Core terms. Source: `canonical.rs` (comments only).

## Why this file matters more than its size suggests

Canonicalisation decides *what counts as the same program*. It is the one component in the [[Trusted Computing Base]] whose failure mode is silent: a bug in the kernel produces a type error, a bug in the store produces a missing node, a bug here produces two genuinely different definitions sharing one hash and therefore one compiled artifact. Nothing downstream can detect it.

This is also the component that has no analogue in Lean or Coq, whose identity is nominal. It is specific to content-addressed code, and [[Unison]] is the only prior system that has had to get it right.

## The pipeline, and why the order is part of the spec

1. **Locally nameless conversion** — [[De Bruijn Index|de Bruijn]] indices for bound variables, hashes for free ones. Buys [[Alpha Equivalence|α-invariance]] by construction rather than by a check.
2. **Let-floating and dead-binding elimination** — only for effect-free bindings. The effect check is not optional; dropping an effectful `let` changes the program.
3. **η-expansion to fixed arity** — so `f` and `λx. f x` agree. Needs types, which is why canonicalisation runs on *elaborated* terms.
4. **Normalisation of type-level subterms** — [[Definitional vs Propositional Equality|definitional equality]] must be reflected in identity, or the kernel's conversion rule and the hash disagree and one term enters the store twice.
5. **Deterministic ordering of unordered structures** — by content hash, which is a fixpoint (the key depends on hashes that depend on the order). Iterate to stability, with a cap.
6. **Primitive attribute normalisation** — overflow discipline, rounding, contraction, alignment made explicit. Julia's wrapping `+` and C++'s `nsw` `+` must *not* converge here; see [[Cross-Language Semantic Hazards]].

## The hard rule

**No algebraic laws.** No commutativity, associativity, distributivity or constant folding. Those are [[E-Graph|e-graph]] equivalences ([[sophia_equiv]]), not identity. The temptation to fold them in is strong — it would make more programs share hashes and improve cache hit rates — and it is precisely the path to needing a decision procedure for program equality, which does not exist ([[Term Rewriting System]], [[Confluence and Termination]]).

The governing principle: **when in doubt, keep it.** Over-fine costs cache hits; over-coarse costs correctness. Those are not symmetric.

## Properties

Idempotence and α-invariance are property-testable. Soundness — `canon(t) == canon(u)` implies `t` and `u` mean the same thing — is not testable, only argued, which is why each step needs a written justification beside it.

## Failure should be loud

Non-convergent ordering, oversized SCCs, exceeded normalisation budgets and unknown attributes all get errors, not best-effort digests. "This term has no identity under schema version V" is an honest answer; a guess is not.

## Related

- [[Hashing and Identity]] · [[sophia_hash]] · [[merkle]]
- [[Alpha Equivalence]] · [[De Bruijn Index]] · [[Normalization by Evaluation]]
- [[Trusted Computing Base]] · [[Cross-Language Semantic Hazards]]
