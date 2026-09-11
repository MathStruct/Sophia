# Dependent Types

A dependent type is a type that depends on a *value*, not just on other types. The canonical example is `Vector(n, T)`, the type of a length-`n` vector of `T`s, where `n` is an ordinary runtime value appearing inside a type. This lets a type system express properties that ordinary (non-dependent) type systems can't, such as "these two matrices have compatible dimensions for multiplication" or, taken further, arbitrary logical propositions about a program's behavior (via [[Curry-Howard Correspondence]]).

Languages built around dependent types (Lean, Idris, Agda, Coq/Rocq — see [[Proof Assistant]]) are used both as programming languages and as proof assistants, because a sufficiently rich dependent type *is* a formally verifiable specification.

[[Start Here]] mentions wanting "Julia + Lean combined, i.e. a Julia with dependent types" as one of the original motivations for this project — using a dependently-typed layer to attach and check correctness properties (and equivalency proofs) on ordinary, dynamically-typed Julia code.

## Related

- [[Type Theory]]
- [[Curry-Howard Correspondence]]
- [[Proof Assistant]]
- [[State of the Art - Dependent Types in Practice]]
