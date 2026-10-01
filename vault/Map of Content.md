#overview

> Every note of the Sophia vault, in reading order. How the vault is organised: [[Start Here]]. The mathematics behind the design is in the [CT-ML wiki](https://mathstruct.org/CategoryTheory-ML-Wiki/), Track F; links to it below are ordinary web links.

## 0. The idea

- [[The Original Idea]] — the project as it was first written down: one graph database for code in every language and all its IRs, equivalence proofs instead of an FFI, hashes instead of names
- [[Design Overview]] — what that becomes once made precise, and the three claims the project makes
- [[Glossary]] — the vocabulary, including where it departs from the original text

## 1. Identity: what a node *is*

1. [[Core Calculus]] — the one language every frontend elaborates into; definitional versus propositional equality; the `Prf`/`Cmp` split
2. [[Hashing and Identity]] — Merkle hashing of canonical forms; binders, cycles, three hashes per definition
3. [[Multi-AST Layering]] — terms, types and proofs as one syntax; source, core, MLIR and LLVM as layers connected by functors
4. [[Graph Schema]] — node and edge kinds, the `EQUIV` edge, three storage backends
5. [[Naming and Change Propagation]] — names as mutable labels over immutable hashes

## 2. Equivalence: when two nodes are *the same*

6. [[Equivalence and Witnesses]] — the ladder from `alpha` to `asserted`; congruence; the cross-language relation; refinement
7. [[Cross-Language Semantic Hazards]] — the concrete ways "the same" turns out to be false
8. [[Effects Memory and Resources]] — effect rows, regions and borrows; four memory models under one roof
9. [[Trusted Computing Base]] — what must be correct for any of this to mean anything
10. [[Tests and Documentation as Nodes]] — evidence that is not proof, and prose that is not code

## 3. Compilation: a compiler as queries

11. [[Compilation as Query]] — passes as rules, saturate then extract, incrementality for free
12. [[Query Cookbook]] — concrete queries in SQL, Cypher and Datalog
13. [[Content-Addressed Precompilation]] — the original Julia motivation, made precise

## 4. Plans and risks

14. [[Roadmap]] — milestones M0–M7, and why M3 is where the project justifies itself
15. [[Open Problems and Risks]] — what may sink it
16. [[Repository Layout]] — where code and notes live; [[Code Map]] — the per-file design notes

## 5. State of the art

[[State of the Art]] is the index. By topic: [[State of the Art - Content-Addressable Code Systems|content-addressed code]] · [[State of the Art - Equality Saturation and E-Graphs|e-graphs]] · [[State of the Art - Incremental Computation|incremental computation]] · [[State of the Art - Graph Databases for Code|graph databases for code]] · [[State of the Art - Code Indexing and Semantic Search|code indexing]] · [[State of the Art - IR Interchange Formats|IR formats]] · [[State of the Art - Sea of Nodes and Graph IRs|graph IRs]] · [[State of the Art - Program Equivalence Checking|equivalence checking]] · [[State of the Art - Proof-Carrying Code and Verified Compilation|verified compilation]] · [[State of the Art - Superoptimization and Synthesis|superoptimisation]] · [[State of the Art - Cross-Language Interoperability|interoperability]] · [[State of the Art - Formal Semantics of Real Languages|semantics of real languages]] · [[State of the Art - Language Semantics Frameworks|semantics frameworks]] · [[State of the Art - Dependent Types in Practice|dependent types]] · [[State of the Art - Julia Compilation and Precompilation|Julia compilation]]

## 6. Background cards

[[Background Concepts]] is the index: LLVM and MLIR, Unison, hashing, rewriting, operational semantics and effect systems, type theory, Julia internals, databases.

## 7. Theory: where each design note's mathematics lives

| design note | CT-ML wiki |
|---|---|
| [[Core Calculus]] | [Category with Families](https://mathstruct.org/CategoryTheory-ML-Wiki/Category-with-Families) · [Curry–Howard–Lambek](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence) · [Call-by-Push-Value](https://mathstruct.org/CategoryTheory-ML-Wiki/Call-by-Push-Value) |
| [[Hashing and Identity]] | [Abstract Syntax with Binding](https://mathstruct.org/CategoryTheory-ML-Wiki/Abstract-Syntax-with-Binding) · [Polynomial Functor](https://mathstruct.org/CategoryTheory-ML-Wiki/Polynomial-Functor) · [Bisimulation](https://mathstruct.org/CategoryTheory-ML-Wiki/Bisimulation) |
| [[Multi-AST Layering]] | [Polynomial Functor](https://mathstruct.org/CategoryTheory-ML-Wiki/Polynomial-Functor) · [Institution](https://mathstruct.org/CategoryTheory-ML-Wiki/Institution) · [Natural Transformation](https://mathstruct.org/CategoryTheory-ML-Wiki/Natural-Transformation) |
| [[Graph Schema]] | [Attributed C-Set](https://mathstruct.org/CategoryTheory-ML-Wiki/Attributed-C-Set) · [Algebraic Database](https://mathstruct.org/CategoryTheory-ML-Wiki/Algebraic-Database) |
| [[Equivalence and Witnesses]] | [Congruence](https://mathstruct.org/CategoryTheory-ML-Wiki/Congruence) · [Contextual Equivalence](https://mathstruct.org/CategoryTheory-ML-Wiki/Contextual-Equivalence) · [Logical Relations](https://mathstruct.org/CategoryTheory-ML-Wiki/Logical-Relations) · [Cartesian Bicategory](https://mathstruct.org/CategoryTheory-ML-Wiki/Cartesian-Bicategory) |
| [[Effects Memory and Resources]] | [Algebraic Effects and Handlers](https://mathstruct.org/CategoryTheory-ML-Wiki/Algebraic-Effects-and-Handlers) · [Freyd Category](https://mathstruct.org/CategoryTheory-ML-Wiki/Freyd-Category) · [Linear-Non-Linear Adjunction](https://mathstruct.org/CategoryTheory-ML-Wiki/Linear-Non-Linear-Adjunction) |
| [[Compilation as Query]] | [E-Graph](https://mathstruct.org/CategoryTheory-ML-Wiki/E-Graph) · [Least Fixed Point](https://mathstruct.org/CategoryTheory-ML-Wiki/Least-Fixed-Point) · [Change Action](https://mathstruct.org/CategoryTheory-ML-Wiki/Change-Action) · [Double-Pushout Rewriting](https://mathstruct.org/CategoryTheory-ML-Wiki/Double-Pushout-Rewriting) |
| [[Query Cookbook]] | [Conjunctive Query](https://mathstruct.org/CategoryTheory-ML-Wiki/Conjunctive-Query) · [Provenance Semiring](https://mathstruct.org/CategoryTheory-ML-Wiki/Provenance-Semiring) · [Monad Comprehension](https://mathstruct.org/CategoryTheory-ML-Wiki/Monad-Comprehension) |
| [[Trusted Computing Base]] | [Compiler Correctness](https://mathstruct.org/CategoryTheory-ML-Wiki/Compiler-Correctness) · [Provenance Semiring](https://mathstruct.org/CategoryTheory-ML-Wiki/Provenance-Semiring) |
| [[Naming and Change Propagation]] | [Relational Lens](https://mathstruct.org/CategoryTheory-ML-Wiki/Relational-Lens) |
