#definition

> Sources: background card; Castellan, Clairambault & Dybjer, *Categories with Families*, [arXiv:1904.00827](https://arxiv.org/abs/1904.00827)
>
> Theory (CT-ML wiki): [Dependent Type](https://mathstruct.org/CategoryTheory-ML-Wiki/Dependent-Type) · [Category with Families](https://mathstruct.org/CategoryTheory-ML-Wiki/Category-with-Families) · [Locally Cartesian Closed Category](https://mathstruct.org/CategoryTheory-ML-Wiki/Locally-Cartesian-Closed-Category)

A dependent type is a type that depends on a *value*, not just on other types. The canonical example is `Vector(n, T)`, the type of a length-`n` vector of `T`s, where `n` is an ordinary runtime value appearing inside a type. This lets a type system express properties that ordinary (non-dependent) type systems can't, such as "these two matrices have compatible dimensions for multiplication" or, taken further, arbitrary logical propositions about a program's behavior (via [Curry-Howard Correspondence](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence)).

Languages built around dependent types (Lean, Idris, Agda, Coq/Rocq — see [[Proof Assistant]]) are used both as programming languages and as proof assistants, because a sufficiently rich dependent type *is* a formally verifiable specification.

[[The Original Idea]] mentions wanting "Julia + Lean combined, i.e. a Julia with dependent types" as one of the original motivations for this project — using a dependently-typed layer to attach and check correctness properties (and equivalency proofs) on ordinary, dynamically-typed Julia code.

## Related

- [[Type Theory]]
- [Curry-Howard Correspondence](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence)
- [[Proof Assistant]]
- [[State of the Art - Dependent Types in Practice]]
