# State of the Art - Julia Compilation and Precompilation

[[Start Here]] and the README both mention wanting to improve Julia precompilation as an original motivation. Current state of Julia's own compilation pipeline:

- **Pipeline stages**: Julia source → surface AST → "lowered" IR (desugared, still fairly high-level) → type inference produces typed IR → [[LLVM IR]] codegen → machine code via [[LLVM]], JIT-compiled per call-site specialization (per concrete argument types) rather than once per function. This per-specialization compilation is a major source of the "time to first X" latency the ecosystem is known for.
- **Precompilation caching**: historically Julia could cache lowered/type-inferred code across a session (`.ji` files) but still had to regenerate native machine code on every process start. Since Julia 1.9, native code caching ("pkgimages") lets packages cache actual compiled machine code to disk, substantially reducing startup/first-call latency — a big step in the direction [[Start Here]] wanted, though still organized around Julia's own package/module system rather than a content-addressed, cross-language graph store.
- **PackageCompiler.jl** — builds standalone system images or apps with a chosen set of packages' code precompiled and baked in, trading flexibility for startup speed; an existing (coarser-grained, whole-system-image) alternative to fine-grained, hash-addressed incremental precompilation.
- **MLIR experiments** — there has been exploratory work on compiling Julia through [[MLIR]] (rather than straight to [[LLVM IR]]) to get access to higher-level, domain-specific dialects and their optimizations (e.g. for array/tensor code) before lowering to LLVM — directly relevant prior art for the "compilation goal should be LLVM/MLIR" line in the README, since it establishes that Julia's typed IR *can* be lowered into MLIR dialects rather than directly to LLVM IR.

## Relevance

Julia's move toward hash-addressable, cacheable compiled artifacts (pkgimages) is the closest thing already in production to the "unique reproducible UUID from the tree of dependencies" idea in [[Start Here]], just scoped to whole precompiled modules rather than individual statements — worth studying as a precedent for cache-invalidation semantics (what counts as "the same" input for cache-hit purposes) before designing the finer-grained, per-statement hashing this project wants.

## Related

- [[MLIR]]
- [[LLVM IR]]
- [[Content-Addressed Code]]
- [[State of the Art - Content-Addressable Code Systems]]
