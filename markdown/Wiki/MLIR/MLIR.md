# MLIR

MLIR (Multi-Level Intermediate Representation) is a compiler infrastructure, developed within the [[LLVM]] project, for building reusable IRs at many different levels of abstraction at once. Instead of one fixed IR, MLIR defines a common extensible framework of generic [[MLIR Operation|operations]] grouped into [[MLIR Dialect|dialects]], letting a compiler represent a program as a high-level, domain-specific graph (e.g. tensor operations, SQL relational algebra, hardware circuits) and progressively [[MLIR Lowering|lower]] it, dialect by dialect, down toward LLVM IR and machine code.

It was created to solve the problem of every domain-specific compiler (TensorFlow's XLA, Swift's SIL, etc.) reinventing its own IR, pass infrastructure, and lowering machinery from scratch. Julia has experimented with MLIR-based compilation (see [[State of the Art - Julia Compilation and Precompilation]]) as a possible middle tier between its own IR and LLVM.

MLIR's operations are natively graph-structured (nodes with typed operands/results and nested regions), which makes it arguably an even closer match than raw [[LLVM IR]] for representing code inside a graph database.

## Related

- [[MLIR Dialect]]
- [[MLIR Operation]]
- [[MLIR Lowering]]
- [[MLIR vs LLVM IR]]
- [[LLVM]]
