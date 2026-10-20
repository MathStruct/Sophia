#design #julia

> Julia's type system as far as identity, dependencies and dispatch need it. What Sophia models (very little, on purpose) and where types will matter. 2026-10-20.
>
> Sources: the Julia manual, *Types* and *Methods*; Zappa Nardelli et al., *Julia Subtyping: A Rational Reconstruction*, OOPSLA 2018; Belyakova et al., *Decidable Subtyping of Existential Types for Julia*, PLDI 2024; [[Type Taxonomy]] (how Julia's types sit among sophialang's); [[Julia Multiple Dispatch and World Age]].

## What Julia has

- **Nominal types**: `struct`, `mutable struct`, `abstract type`, `primitive type`, with single inheritance of abstract types.
- **Parametric types** with `where` bounds; **union** types; **tuple** types (covariant); `Vararg`; `Type{T}` singletons.
- Types are **values** at run time; type annotations on fields and arguments are checked at run time and drive dispatch.
- **Subtyping** decides dispatch and is the hardest algorithm in Julia (undecidable in general; Julia's implementation is a heuristic search).

## What Sophia models

| Aspect | Model | Level |
|---|---|---|
| a type definition | a unit with a content id; fields and supertypes are part of the content | L2 |
| a reference to a type defined in the same module | a content reference (the type's id) | L3 |
| a reference to a type elsewhere (`AbstractMatrix`, `Float64`) | its spelling | L2 |
| `where` type parameters | positional locals since v6 (renaming `T` does not change the id) | L2 |
| a type's constructor methods | the generic function `jl.M.T.new` | L3 |
| subtyping, type inference, specialisation | **not modelled** | — |

## Where types will matter

- **Nominal identity.** Two structs with identical fields in different modules are different types in Julia. With content addressing they have one content id. Sophia's answer, as for generic functions, is a **nominal entity** per type (a fresh id) whose definition is the content-addressed record; the review raised the same point for sophialang relations ([[sophialang_core]]).
- **Dispatch facts** need argument types; they come from Julia ([[Julia Multiple Dispatch and World Age]]).
- **Cross-language equivalence** (Julia's `Matrix{Float64}` vs a Rust `&[f64]` with dimensions) is where types meet [[Cross-Language Semantic Hazards]]: layout, aliasing and mutability differ.
- **AD** differentiates per signature; types decide which derivative program exists ([[AD as a Materialization Case]]).
