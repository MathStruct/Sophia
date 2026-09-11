# MLIR vs LLVM IR

[[LLVM IR]] is a single, fixed instruction set designed to sit close to machine code — it has one level of abstraction. [[MLIR]] is a framework for defining *many* IRs (dialects) at *many* levels of abstraction, plus the shared infrastructure (verifier, pass manager, rewrite engine, printer/parser) to manipulate all of them uniformly.

In practice they are complementary rather than competing: a compiler built on MLIR usually still ends by lowering into MLIR's `llvm` dialect and handing off to real LLVM IR and the existing [[LLVM]] backend, to reuse LLVM's mature code generation and target support. MLIR's contribution is the earlier stages — representing a program at higher levels (source-language-ish, or domain-specific) before that final drop to LLVM.

For a project storing code and its IRs in a graph database, this suggests a natural layering: source-language ASTs at the top, one or more MLIR-dialect layers in the middle for language-agnostic and domain-specific transformations, and LLVM IR as the final common layer before machine code.

## Related

- [[MLIR]]
- [[LLVM IR]]
- [[MLIR Lowering]]
