# Effect System

A type-and-effect system augments a typing judgement with a description of what a computation *does* in addition to what it *returns*:

$$ Gamma ⊢ t : A ! epsilon $$

where `ε` is a set (a **row**) of effect labels — `io`, `throw(E)`, `alloc(r)`, `div`, `nondet`. Row polymorphism lets generic code be effect-generic: `map : (A -!ρ-> B) -> List A -!ρ-> List B` performs exactly the effects its argument does.

Three related traditions:

- **Monads** (Haskell): effects are encoded in the *return type*, `IO a`. Composes awkwardly when several effects interact (monad transformers).
- **Algebraic effects and handlers** (Eff, Koka, OCaml 5, [[Unison Abilities]]): an effect is an *interface* of operations, and a handler gives an interpretation. Effects compose without transformer towers, and handlers subsume exceptions, generators, async and backtracking with one mechanism.
- **Region and ownership systems** (Cyclone, Rust, MLKit): effects indexed by *where* in memory they happen, so `read(r)`/`write(r)` can be checked for non-interference.

Why this is central to [[Start Here]]'s ambitions rather than a refinement:

1. **Purity licenses rewriting.** CSE, reordering, memoisation and parallelisation are all sound only for effect-free code. An effect row is the certificate.
2. **Effects are the observations.** [[Contextual Equivalence]] is defined against an observation; effects are what is observed. Two functions with different effect rows are not equivalent, full stop.
3. **Effects are where languages differ most.** A Julia mutation, a Rust `&mut` write and a C++ pointer store are three mechanisms; whether they are the "same effect" is a question about handlers and regions, not about syntax. See [[Effects Memory and Resources]].

## Related

- [[Unison Abilities]]
- [[Effects Memory and Resources]]
- [[Linear and Affine Types]]
- [[Contextual Equivalence]]
- [[Core Calculus]]
