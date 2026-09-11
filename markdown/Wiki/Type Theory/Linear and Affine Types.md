# Linear and Affine Types

Substructural type systems restrict the structural rules that ordinary type systems take for granted:

- dropping **weakening** (a value may not be discarded) gives **relevant** types,
- dropping **contraction** (a value may not be duplicated) gives **affine** types,
- dropping both gives **linear** types: every value is used *exactly once*.

The practical payoff is that a linear or affine value can be safely mutated in place, freed deterministically, or handed to exactly one owner — because no one else holds a reference. Rust's ownership system is affine types with borrowing bolted on; `Vec<T>` is affine, `&mut T` is a borrow tracked by a lifetime, and `Copy` marks the types that opt back into contraction.

**Quantitative type theory** (Idris 2, and Linear Haskell's multiplicities) generalises this to a usage annotation on each binder — `0` for erased-at-runtime, `1` for linear, `ω` for unrestricted — which unifies erasure, linearity and ordinary types in one system. That is an attractive shape for [[Core Calculus|SC]], because it makes the `h_run` / `h_def` distinction in [[Hashing and Identity]] fall out of the calculus: the `0`-quantified parts are exactly what erasure removes.

For this project's purposes, the key observation is in [[Effects Memory and Resources]]: **Julia's model is unrestricted, Rust's is affine, so the translation Julia → Rust is not an equivalence.** Rust forbids programs Julia permits. The correct relation is `REFINES`, and treating it as `EQUIV` is a way to generate genuine miscompilations around aliasing.

## Related

- [[Effect System]]
- [[Effects Memory and Resources]]
- [[Dependent Types]]
- [[Equivalence and Witnesses]]
