# Wiki Home

A set of linked reference cards for the background concepts behind [[Start Here]]. Each card covers one concept and links to related cards. This is not exhaustive — it's a map to get oriented. For the project's own design decisions rather than background, see [[Design Overview]].

## Sub-wikis

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
  - [[E-Graph]] · [[Equality Saturation]] · [[Term Rewriting System]] · [[Confluence and Termination]]
- **Semantics** — what it means for two programs to be "the same"
  - [[Operational Semantics]] · [[Contextual Equivalence]] · [[Logical Relations]] · [[Bisimulation]] · [[Effect System]]
- **Foundations**
  - [[Type Theory]] — types, proofs, and dependent types
    - [[Dependent Types]] · [[Curry-Howard Correspondence]] · [[Curry-Howard-Lambeck Correspondence]] · [[Proof Assistant]] · [[Normalization by Evaluation]] · [[Definitional vs Propositional Equality]] · [[Linear and Affine Types]]
  - [[Category Theory]] — objects, morphisms, and the structures that connect to type theory
    - [[Category]] · [[Functor]] · [[Natural Transformation]] · [[Initial Algebra]] · [[Cartesian Closed Category]] · [[Institution]] · [[Category Theory and Programming Languages]]
- **Julia specifics** — the language this project started from
  - [[Multiple Dispatch]] · [[World Age]] · [[Julia Lowered IR]] · [[Pkgimage]]
- **Databases**
  - [[Property Graph]] · [[Datalog]]

## Suggested reading orders

**"I want to understand the identity scheme."**
[[Content-Addressed Code]] → [[Merkle DAG]] → [[Alpha Equivalence]] → [[De Bruijn Index]] → [[Cycle Hashing]] → [[Hashing and Identity]]

**"I want to understand the equivalence claim."**
[[Contextual Equivalence]] → [[Logical Relations]] → [[Definitional vs Propositional Equality]] → [[E-Graph]] → [[Equivalence and Witnesses]] → [[Cross-Language Semantic Hazards]]

**"I want to understand why category theory keeps coming up."**
[[Category]] → [[Functor]] → [[Natural Transformation]] → [[Initial Algebra]] → [[Curry-Howard-Lambeck Correspondence]] → [[Multi-AST Layering]]

**"I care about the Julia performance story."**
[[Multiple Dispatch]] → [[World Age]] → [[Pkgimage]] → [[Content-Addressed Precompilation]]

## See also

- [[Design Overview]] — the project's own design decisions
- [[State of the Art]] — survey of existing systems and research adjacent to this project's goals
- [[Glossary]] — project vocabulary
