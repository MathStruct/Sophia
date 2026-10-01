#overview

> Sources: index of the background cards in this folder
>
> Theory (CT-ML wiki): [Start Here, Track F](https://mathstruct.org/CategoryTheory-ML-Wiki/Start-Here) · [Map of Content](https://mathstruct.org/CategoryTheory-ML-Wiki/Map-of-Content)

Linked reference cards for the *engineering* background behind [[The Original Idea]]: compilers and IRs, content-addressed code, hashing, rewriting, Julia internals, databases. Each card covers one concept and links to related cards. For the project's own design decisions see [[Design Overview]].

The *mathematical* background — category theory, categorical semantics, logical relations, e-graphs as algebra, conjunctive queries, fixed points — lives in the [CT-ML wiki](https://mathstruct.org/CategoryTheory-ML-Wiki/), the root vault this one builds on; its **Track F** leads up to every concept Sophia uses. Cards that used to duplicate it here have been replaced by links to it.

## Cards

- **Compilers & IRs**
  - [[LLVM]] — the optimizing compiler infrastructure and its IR
    - [[LLVM IR]] · [[LLVM Pass]] · [[LLVM Bitcode]] · [[SSA Form]] · [[Sea of Nodes]]
  - [[MLIR]] — multi-level IR framework built around extensible dialects
    - [[MLIR Dialect]] · [[MLIR Operation]] · [[MLIR Lowering]] · [[MLIR vs LLVM IR]]
- **Content-addressed code**
  - [[Unison]] — a language whose code is identified by hashes of its syntax
    - [[Content-Addressed Code]] · [[Unison Codebase]] · [[Unison Abilities]]
- **Hashing & identity** — how a piece of code gets a name that is not a name
  - [[Merkle DAG]] · [[Alpha Equivalence]] · [[De Bruijn Index]] · [[Hash Consing]] · [[BLAKE3]] · [[Cycle Hashing]] · [[UUID]]
- **Rewriting & equivalence**
  - [[Term Rewriting System]] · [[Confluence and Termination]]
- **Semantics** — what a program does, and what it means for two programs to be "the same"
  - [[Operational Semantics]] · [[Effect System]]
- **Type theory**
  - [[Type Theory]] · [[Dependent Types]] · [[Proof Assistant]] · [[Normalization by Evaluation]] · [[Definitional vs Propositional Equality]] · [[Linear and Affine Types]]
- **Julia specifics** — the language this project started from
  - [[Multiple Dispatch]] · [[World Age]] · [[Julia Lowered IR]] · [[Pkgimage]]
- **Databases**
  - [[Property Graph]] · [[Datalog]]

## In the CT-ML wiki

| topic | wiki notes |
|---|---|
| syntax and identity | [Polynomial Functor](https://mathstruct.org/CategoryTheory-ML-Wiki/Polynomial-Functor) · [Abstract Syntax with Binding](https://mathstruct.org/CategoryTheory-ML-Wiki/Abstract-Syntax-with-Binding) · [Initial Algebra](https://mathstruct.org/CategoryTheory-ML-Wiki/Initial-Algebra) · [Bisimulation](https://mathstruct.org/CategoryTheory-ML-Wiki/Bisimulation) |
| semantics | [Curry–Howard–Lambek](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence) · [Category with Families](https://mathstruct.org/CategoryTheory-ML-Wiki/Category-with-Families) · [Algebraic Effects and Handlers](https://mathstruct.org/CategoryTheory-ML-Wiki/Algebraic-Effects-and-Handlers) · [Freyd Category](https://mathstruct.org/CategoryTheory-ML-Wiki/Freyd-Category) · [Linear-Non-Linear Adjunction](https://mathstruct.org/CategoryTheory-ML-Wiki/Linear-Non-Linear-Adjunction) |
| equivalence | [Congruence](https://mathstruct.org/CategoryTheory-ML-Wiki/Congruence) · [Contextual Equivalence](https://mathstruct.org/CategoryTheory-ML-Wiki/Contextual-Equivalence) · [Logical Relations](https://mathstruct.org/CategoryTheory-ML-Wiki/Logical-Relations) · [Institution](https://mathstruct.org/CategoryTheory-ML-Wiki/Institution) · [Compiler Correctness](https://mathstruct.org/CategoryTheory-ML-Wiki/Compiler-Correctness) |
| rewriting | [E-Graph](https://mathstruct.org/CategoryTheory-ML-Wiki/E-Graph) (equality saturation) · [Double-Pushout Rewriting](https://mathstruct.org/CategoryTheory-ML-Wiki/Double-Pushout-Rewriting) |
| databases | [Attributed C-Set](https://mathstruct.org/CategoryTheory-ML-Wiki/Attributed-C-Set) · [Conjunctive Query](https://mathstruct.org/CategoryTheory-ML-Wiki/Conjunctive-Query) · [Least Fixed Point](https://mathstruct.org/CategoryTheory-ML-Wiki/Least-Fixed-Point) · [Provenance Semiring](https://mathstruct.org/CategoryTheory-ML-Wiki/Provenance-Semiring) · [Change Action](https://mathstruct.org/CategoryTheory-ML-Wiki/Change-Action) |

## Suggested reading orders

**"I want to understand the identity scheme."**
[[Content-Addressed Code]] → [[Merkle DAG]] → [[Alpha Equivalence]] → [[De Bruijn Index]] → [[Cycle Hashing]] → [[Hashing and Identity]]

**"I want to understand the equivalence claim."**
[Contextual Equivalence](https://mathstruct.org/CategoryTheory-ML-Wiki/Contextual-Equivalence) → [Logical Relations](https://mathstruct.org/CategoryTheory-ML-Wiki/Logical-Relations) → [[Definitional vs Propositional Equality]] → [E-Graph](https://mathstruct.org/CategoryTheory-ML-Wiki/E-Graph) → [[Equivalence and Witnesses]] → [[Cross-Language Semantic Hazards]]

**"I want to understand why category theory keeps coming up."**
[Track F of the CT-ML wiki](https://mathstruct.org/CategoryTheory-ML-Wiki/Start-Here) → [[Multi-AST Layering]]

**"I care about the Julia performance story."**
[[Multiple Dispatch]] → [[World Age]] → [[Pkgimage]] → [[Content-Addressed Precompilation]]

## See also

- [[Design Overview]] — the project's own design decisions
- [[State of the Art]] — survey of existing systems and research adjacent to this project's goals
- [[Glossary]] — project vocabulary
