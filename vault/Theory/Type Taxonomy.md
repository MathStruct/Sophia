#definition #overview

> A systematic overview of the kinds of types a language can have, and the place of each in Sophia (SR-61). Types play **three different roles**, and most confusion about "types" comes from mixing them up:
> - **representation**: how many bits there are, and where they go;
> - **identity and classification**: which values belong together, and which method applies;
> - **specification**: what is true of a value.
>
> Machine types belong to the first role, Julia's abstract types and nominal "types as a UUID" to the second, and the type *systems* (algebraic, dependent, cubical, linear, refinement, …) mostly to the third. Sophialang keeps the three roles apart, so that the specification systems can be exchanged without changing compiled code ([[SPEC-002 Sophialang Principles]] §3).
>
> Sources: Bezanson et al., *Julia: Dynamism and Performance Reconciled by Design*, OOPSLA 2018 (the type lattice, `Union{}`, `UnionAll`); Zappa Nardelli et al., *Julia Subtyping: A Rational Reconstruction*, OOPSLA 2018; Pierce, *Types and Programming Languages*, 2002; Martin-Löf, *Intuitionistic Type Theory*, 1984; Cohen, Coquand, Huber and Mörtberg, *Cubical Type Theory*, TYPES 2015; Atkey, *QTT*, LICS 2018; Rondon, Kawaguchi and Jhala, *Liquid Types*, PLDI 2008; Honda, *Types for Dyadic Interaction*, CONCUR 1993; Leijen, *Koka*, 2014; Siek and Taha, *Gradual Typing*, 2006; Castagna, *Programming with Union, Intersection and Negation Types*, 2023; Rossberg, Russo and Dreyer, *F-ing Modules*, JFP 2014 (generativity). code: `crates/sophialang-core` (planned).
>
> Theory (CT-ML wiki): [Category with Families](https://mathstruct.org/CategoryTheory-ML-Wiki/Category-with-Families) · [Polynomial Functor](https://mathstruct.org/CategoryTheory-ML-Wiki/Polynomial-Functor) · [Linear-Non-Linear Adjunction](https://mathstruct.org/CategoryTheory-ML-Wiki/Linear-Non-Linear-Adjunction) · [Algebraic Effects and Handlers](https://mathstruct.org/CategoryTheory-ML-Wiki/Algebraic-Effects-and-Handlers) · [Curry-Howard-Lambek Correspondence](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence). This vault: [[Computational Trinitarianism]].

## The map

```tikz
\begin{document}
\begin{tikzpicture}[x=1cm,y=1cm, every node/.style={font=\small, align=center}]
\node[draw, rounded corners, minimum width=4cm] (rep) at (0,0) {Representation\\ \footnotesize bits, layout, calling convention};
\node[draw, rounded corners, minimum width=4cm] (cls) at (5,0) {Classification \& identity\\ \footnotesize which values, which method};
\node[draw, rounded corners, minimum width=4cm] (spec) at (10,0) {Specification\\ \footnotesize what is true};
\node (r1) at (0,-1.5) {\footnotesize Int64, Float64, Bool, Ptr\\ \footnotesize structs, tuples, arrays};
\node (c1) at (5,-1.5) {\footnotesize Any, Union\{\}, Nothing, Union\\ \footnotesize abstract types, nominal ids,\\ \footnotesize parameters (UnionAll)};
\node (s1) at (10,-1.5) {\footnotesize algebraic, dependent, cubical,\\ \footnotesize linear/QTT, refinement, session,\\ \footnotesize effect, modal, probabilistic};
\node at (0,-2.8) {\footnotesize exists at run time};
\node at (5,-2.8) {\footnotesize exists at dispatch time};
\node at (10,-2.8) {\footnotesize erased (usage 0)};
\end{tikzpicture}
\end{document}
```

## 1. Representation types: what the processor sees

| Type | What it is | Defined by |
|---|---|---|
| `Int8`–`Int128`, `UInt8`–`UInt128` | two's-complement integers of a fixed width | the processor (registers, ALU) |
| `Float16`, `Float32`, `Float64` (`BFloat16`) | IEEE 754 binary floating point | the processor's FPU or vector unit |
| `Bool`, `Char` | one byte; a Unicode scalar value (Julia uses 32 bits) | convention on top of integers |
| `Ptr{T}` | an address | the processor's address width |
| `Vec{N,T}` (SIMD) | a vector register | SSE, AVX, NEON, SVE |
| **composite concrete types**: structs, tuples, fixed arrays | layouts derived from their fields: size, alignment, padding | the compiler's layout rules (C ABI, or Julia's `isbits`) |

- **In Sophia:** primitive types are built-in **nominal** definitions (`primitive type Int64 64 end`, as in Julia) with a declared bit width. Composite layouts are content-addressed `layout` definitions ([[SPEC-003 Sophialang Core v0]] §3).
- **These are the only types that exist at run time.** The compiler needs exactly these to generate code, which is why every other kind of type can be erased (SPEC-002 §3).

## 2. The classification lattice: Julia's abstract types

Julia organises types in a lattice used for **dispatch**:

| Type | Meaning | Logic reading |
|---|---|---|
| `Any` | the top: every value | ⊤ (true) |
| `Union{}` | the bottom: no value. The type of a computation that never returns (`throw`, an infinite loop) | ⊥ (false) |
| `Nothing` | a type with exactly one value, `nothing` | the unit type, 1 |
| `Union{A, B}` | values of `A` or of `B` (untagged: the value knows its concrete type) | A ∨ B |
| `abstract type Number end` | a named set of concrete types, declared by subtyping (`Int64 <: Signed <: Integer <: Real <: Number`) | a predicate, given extensionally by declaration |
| `UnionAll` (`Vector{T} where T<:Real`) | a family of types over a bounded parameter | bounded ∃ (or ∀, depending on position) |
| `Type{T}` | the singleton type whose only value is the type `T` | types as values |
| `Tuple{A, B}` | covariant tuples. Method signatures are tuple types | A ∧ B |

- **In Sophia:** these are **classifications for dispatch**. A method's signature is a tuple type in this lattice, and dispatch picks the most specific applicable signature (SPEC-002 §4).
- Abstract types are nominal (§3), because declaring `abstract type Shape end` twice should give two different families.
- `Union{}` is also how sophialang types a computation that never returns, which matters to the effect checker.

## 3. Types as identity: a UUID or a hash

A type can be **only an identity**: a fresh name with no structure, or structure that is deliberately ignored. This is **generativity**: each declaration makes a new type, even if it is textually identical to another.

| Form | Example | Use |
|---|---|---|
| **nominal type** | Julia `struct Meters; x::Float64; end` and `struct Seconds; x::Float64; end` are different types with the same layout | units, domain distinctions, dispatch |
| **fresh / generative type** ("draw a random type") | Unison `unique type`, ML's generative datatypes, `gensym`-like `newtype` | guaranteed-distinct tags; capabilities; brands |
| **structural type** | Unison `structural type`, a sophialang `data` | two equal declarations *are* one type: the hash is the identity |
| **phantom / brand types** | `Handle{Owner}` where `Owner` has no values | to prove *who* may use something (capabilities) |

- **In Sophia:** this is exactly the store's distinction between **nominal entities** (a fresh 256-bit id, SPEC-001 §4.3) and **content-addressed definitions**.
- `struct`, `abstract`, `primitive` and generic functions are nominal. `data` is structural.
- "Just draw a random type whenever you need one" is the `nominal` operation of the store.

## 4. Type systems: ways of building more types

Each row below is a *type system*, a discipline for building and checking types. In Sophia each one is a **checker** that produces derived facts of kind `typing` (SPEC-001 §9). Several systems can check the same definition.

| System | Builds | Reads as (logic) | Erased at run time? | Status in Sophia |
|---|---|---|---|---|
| **Algebraic** (sums, products, inductive and coinductive types) | `data Nat`, `Either`, records, recursive types | ∨, ∧, least and greatest fixed points | no: these are data | core v0 (`data`, `struct`) |
| **Parametric polymorphism** (System F) | `∀A. A → A` | second-order ∀ | yes | core v0 (`Pi` over `Type`) |
| **Dependent** (Martin-Löf: Π, Σ, indexed families, universes) | `Vec(A, n)`, `(n : Nat) → Vec(A, n)` | first-order ∀, ∃ over values | the indices are usually erased | core v0 (stored), checker at M5 |
| **Identity and paths**: intensional MLTT; **cubical** (path types, univalence, higher inductive types) | `x = y`; `Path A x y`; quotients | equality as a type; equivalences *are* equalities | yes | later; a cubical checker as a separate system |
| **Linear, affine, quantitative** (QTT: usages 0, 1, ω) | values used exactly once; erasure | linear logic (⊗, ⊸) | the usage-0 parts are | core v0 (usages on binders); checker at M5 |
| **Ownership and regions** (Rust-like, separation logic) | `&'a T`, `&mut T`, `Box<T>` | separation logic (∗, −∗) | yes | the memory DSL (SPEC-002 §5) |
| **Refinement** (liquid types) | `{x : Int64 \| x > 0}` | predicates over values, decided by SMT | yes | later; an SMT-backed checker |
| **Session types** | protocols on channels: `!Int. ?Bool. end` | linear logic (Caires–Pfenning) | yes | the concurrency DSL (SPEC-002 §6) |
| **Effect types and rows** (Koka, algebraic effects) | `Int →{io, exn} Int` | modal / monadic logic | yes | later; required for autoresolved parallelism |
| **Modal types** (staging: □A as "code of A") | compile-time vs run-time values | S4 necessity | yes | the JIT DSL (SPEC-002 §7) |
| **Gradual types** | `?` or `Any` as the dynamic type, with casts | — | the casts remain | already Julia's model: `Any` |
| **Set-theoretic types** (union, intersection, negation) | `Union{A,B}`, `A ∩ B`, `¬A` | Boolean algebra of types | partly | `Union` in v0 (dispatch); the rest later |
| **Probabilistic types** | `Dist(A)`: distributions over A | Markov categories | no: these are data | for weighted rules (Lenticulum LT-24 to LT-29) |

## 5. How they fit together in sophialang

1. **A definition's declared type is a core term** (SPEC-003 §2). Dependent and polymorphic types are just terms. Types from other systems are written in their DSLs and stored as annotations or as typing facts.
2. **Dispatch uses only the classification lattice** (§2 and §3), computed from representation and nominal types. The dependent index `n` in `Vec(A, n)` does not take part in dispatch, unless a method's signature explicitly asks for a value-level singleton type, as Julia's `Val{n}` does.
3. **The compiler uses only representation types** (§1), plus resolved dispatch.
4. **Checkers add facts.** "Under system *S*, definition *d* has type *T*" is a derived fact. A build can *require* facts ("every definition reachable from `main` checks under `sophialang-dependent`"), but never *consumes* them for code generation. That is SR-53's "exchange the type system without performance cost", made precise.

## 6. Open points

- **Subtyping and dependent types interact badly**: decidability and coherence. Julia's subtyping is itself famously intricate (Zappa Nardelli et al. 2018). The dependent checker may have to treat the lattice as opaque, using only nominal subtyping. → to be recorded as a problem when the checker is built.
- **Cubical and erasure:** path types are erased, but transport along univalence is a *computation*. If a program uses univalence to move a function between two representations, the transport is run-time code and has a cost. That has to be visible, never hidden (SR-52).

## Related

[[SPEC-002 Sophialang Principles]] · [[SPEC-003 Sophialang Core v0]] · [[Computational Trinitarianism]] · [[Dependent Types]] · [[Linear and Affine Types]] · [[Effect System]] · [[Multiple Dispatch]]
