# State of the Art

A survey of existing systems and research adjacent to the goals in [[Start Here]]: storing code and its intermediate representations in a graph database, addressing it by content hash, and unifying multiple languages through inserted equivalency proofs rather than FFI. Background concepts are in [[Wiki Home]]; the project's own design decisions are in [[Design Overview]].

## Storage and identity

- [[State of the Art - Graph Databases for Code]] — the candidate databases (LadybugDB, HelixDB, TypeDB, FalkorDB, DuckDB, TursoDB) plus existing code-as-graph systems (Glean, Kythe)
- [[State of the Art - Content-Addressable Code Systems]] — Unison, Nix, Git, IPFS/IPLD, and other hash-identified storage systems
- [[State of the Art - Code Indexing and Semantic Search]] — Kythe, Glean, SCIP/LSIF, stack graphs, tree-sitter, CodeQL: the ingestion half of the idea, already solved at scale

## Representation and compilation

- [[State of the Art - Sea of Nodes and Graph IRs]] — graph-structured IRs in production compilers, and V8's retreat from one
- [[State of the Art - IR Interchange Formats]] — LLVM bitcode, Wasm, SPIR-V, CIL, MLIR bytecode, and what .NET's multi-language IR teaches about unification
- [[State of the Art - Julia Compilation and Precompilation]] — Julia's own pipeline, its precompilation caching, and MLIR experiments within Julia
- [[State of the Art - Incremental Computation]] — Salsa, Adapton, rustc's query system, Nix, Bazel, and "Build Systems à la Carte"

## Equivalence and proof

- [[State of the Art - Equality Saturation and E-Graphs]] — e-graphs and equality saturation as a mechanism for storing and reasoning about equivalent code
- [[State of the Art - Program Equivalence Checking]] — Alive2, translation validation, RVT, SymDiff, and cross-language logical relations
- [[State of the Art - Superoptimization and Synthesis]] — STOKE, Souper, Denali, Rosette: automatically *discovering* equivalences
- [[State of the Art - Proof-Carrying Code and Verified Compilation]] — attaching or checking proofs about code, and compiler correctness

## Languages and semantics

- [[State of the Art - Cross-Language Interoperability]] — how multi-language interop is solved today, and how it contrasts with the "no FFI" goal
- [[State of the Art - Language Semantics Frameworks]] — K, PLT Redex, Ott, Spoofax, Truffle, Iris: the tooling for writing a semantics down
- [[State of the Art - Formal Semantics of Real Languages]] — what is actually known formally about C, C++, Rust, Julia, LLVM IR and Wasm
- [[State of the Art - Dependent Types in Practice]] — Lean, Idris, Agda, Coq/Rocq, F*, and how they're used today

## Summary of what exists and what does not

| Capability | Prior art | Maturity |
| --- | --- | --- |
| Parse many languages into one graph | Kythe, Glean, tree-sitter | production, large scale |
| Content-address code by structural hash | Unison, Nix, Git | production |
| Query code as a database | CodeQL, Glean, Kythe | production |
| Compile *from* a graph IR | Sea of Nodes, MLIR, Cranelift | production, but in-memory only |
| Store many equivalent programs compactly | egg, egglog, Cranelift ægraphs | maturing |
| Check equivalence per compilation instance | Alive2, translation validation | production for LLVM |
| Discover equivalences automatically | Souper, STOKE, Denali | research, small scale |
| Multi-language runtime without FFI | GraalVM/Truffle, .NET CIL, Wasm components | production, via convergence not proof |
| **Persist the IR graph across sessions** | — | **none** |
| **Cross-language equivalence with proofs** | theory only (multi-language semantics) | **none** |
| **Attach proofs to an interchange format** | — | **none** |

The last three rows are where this project would be doing something new. Everything above them can and should be borrowed rather than reinvented — which is also the ordering argument in [[Roadmap]].

## Related

- [[Start Here]]
- [[Design Overview]]
- [[Open Problems and Risks]]
