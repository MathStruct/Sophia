#definition

> Sources: background card; Lattner et al., *MLIR: A Compiler Infrastructure for the End of Moore's Law*, [arXiv:2002.11054](https://arxiv.org/abs/2002.11054)
>
> Theory (CT-ML wiki): [Compiler Correctness](https://mathstruct.org/CategoryTheory-ML-Wiki/Compiler-Correctness) · [Double-Pushout Rewriting](https://mathstruct.org/CategoryTheory-ML-Wiki/Double-Pushout-Rewriting)

Lowering is the process of converting operations from a higher-level [[MLIR Dialect]] into operations from a lower-level one — e.g. `linalg` (linear algebra) → `scf` (loops) → `cf` (branches) → `llvm` dialect → [[LLVM IR]]. Each step is typically implemented as a set of pattern-based rewrite rules run by a [[MLIR Lowering|conversion]] pass, similar in spirit to an [[LLVM Pass]] but operating dialect-to-dialect instead of IR-to-IR.

A single program is usually lowered through several dialects in sequence rather than jumping straight to LLVM, because each intermediate level is still a good place to run optimizations that only make sense at that level (e.g. loop tiling makes sense on `scf`/`linalg`, but not once everything has become flat LLVM instructions).

This progressive, multi-level lowering is a close structural analogue to what [[The Original Idea]] describes wanting: multiple ASTs at different levels (source language, typed IR, LLVM/MLIR) connected by transformations, all persisted as one graph.

## Related

- [[MLIR]]
- [[MLIR Dialect]]
- [[LLVM Pass]]
- [[LLVM IR]]
