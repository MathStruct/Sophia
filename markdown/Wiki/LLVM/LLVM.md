# LLVM

LLVM is a collection of modular, reusable compiler and toolchain technologies built around a common intermediate representation, [[LLVM IR]]. A frontend (e.g. Clang for C/C++, or `rustc`, or Julia's compiler) translates source code into LLVM IR; LLVM then runs a pipeline of [[LLVM Pass|passes]] over that IR to optimize it, and finally lowers it to machine code for a target architecture.

Its key design idea is the separation of frontend, optimizer, and backend around a shared IR: any language that can emit LLVM IR gets access to the same optimizations and the same set of target backends, without either side needing to know about the other.

This is directly relevant as a **compilation target**: a graph-database-based compiler does not need to invent its own code generation — it can assemble LLVM IR (or go through [[MLIR]] first) and let LLVM handle optimization and machine code emission.

## Related

- [[LLVM IR]]
- [[LLVM Pass]]
- [[LLVM Bitcode]]
- [[SSA Form]]
- [[MLIR vs LLVM IR]]
