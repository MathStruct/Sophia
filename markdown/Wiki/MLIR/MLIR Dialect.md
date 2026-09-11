# MLIR Dialect

A dialect is a namespace within [[MLIR]] that groups a related set of [[MLIR Operation|operations]], types, and attributes under one prefix (e.g. `arith.addi`, `scf.for`, `llvm.getelementptr`). Dialects are how MLIR stays generic: the core infrastructure (the IR data structures, the pass manager, the pattern-rewrite engine) knows nothing about any specific dialect, and new dialects can be added without modifying MLIR itself.

Common built-in dialects include `func` (functions), `arith`/`math` (arithmetic), `scf` (structured control flow — loops, ifs), `cf` (unstructured branches), `memref` (memory buffers), `tensor`/`linalg` (array and linear-algebra computation), and `llvm` (a dialect that mirrors [[LLVM IR]] itself, used as the final stop before leaving MLIR).

A single module can freely mix operations from multiple dialects, which is exactly the kind of multi-language, multi-abstraction-level mixing described in [[Start Here]].

## Related

- [[MLIR]]
- [[MLIR Operation]]
- [[MLIR Lowering]]
