#design

> A compiled sophialang program must be obtainable as a **minimal binary**, or another target artefact, that runs without the Sophia machinery: no store, no hashing, no JIT, no GC, unless the program uses them (SR-60). Like the embedded case. This note says how the store makes that easy, rather than hard, and what has to be true of a program for it to work.
>
> Sources: dead-code elimination and tree shaking (JavaScript bundlers, Rust's `#![no_std]` and LTO); Julia's `juliac` and `--trim` (Julia 1.12) and PackageCompiler; GraalVM native-image (closed-world assumption); Bazel and Nix reproducible builds. code: none yet (`sophia-build`, M5).
>
> Theory: [[Computational Trinitarianism]] §4.3 (the compile DSL), [[SPEC-002 Sophialang Principles]] §3, §7.

## 1. The closure is exact

A content id's Merkle closure is exactly the set of definitions it depends on statically ([[Theory of the v0 Store]] §1). So:

```
binary(main) = compile( closure(main) ∪ dispatch_closure(main) )
```

- **The static closure** is a graph walk over `refs`, with no guessing and no "include the whole package".
- **The dispatch closure** consists of the methods reachable through generic functions, at the method-table versions the build pins. It is exact when every call site from `main` is statically resolved, which is the `compile` DSL's `Compiled(main, S)` proof ([[Computational Trinitarianism]] §4.3). Without that proof the build must include a dispatcher plus every method that *could* be called: a larger binary, but still no store.

This is tree shaking without heuristics. Julia needs `--trim` and a closed-world analysis to get there. Here it is the default representation.

## 2. What the binary contains

| Component | Included when | Size class |
|---|---|---|
| machine code for the closure | always | the program |
| a run-time dispatcher | some call is not statically resolved (the autoresolve level) | small |
| GC runtime | some allocation is not explicitly managed (the memory autoresolve level) | moderate |
| JIT and interpreter | the program asks to compile at run time (the compilation autoresolve level) | large: this is the full machinery |
| scheduler | autoresolved parallelism is used | small |
| proofs, types, specifications, docs, tests | **never**: they are erased (SPEC-002 §3) | 0 |
| the store, names, hashes | **never** at run time | 0 |

A program written entirely at the explicit or safe levels (manual memory, explicit threads, all dispatch proved static) compiles to a binary with **no runtime beyond what C needs**: no GC, no JIT, no dispatcher. That is the embedded case.

## 3. Build artefacts and reproducibility

- **A build is a derived fact**: `compiled(main, target, options, toolchain version) → artefact digest`, with inputs being the closure's content ids plus the pinned method-table versions. The same inputs give the same artefact, and the cache is keyed by that.
- **A manifest** (the list of content ids per symbol) can be emitted beside the binary. It is not inside the binary, and it is optional, for debugging and for mapping crashes back to definitions.
- **Targets:** native code through LLVM (x86-64, AArch64, RISC-V, bare-metal), WebAssembly, and static or shared libraries with a C ABI, so that a sophialang library can be linked into C, Rust or Julia programs without Sophia.

## 4. What could go wrong

- **Dynamic features leak the runtime in.** One unresolved call pulls in the dispatcher; one GC'd allocation pulls in the GC. The build must *report* why each runtime component was included ("included the GC because of `foo` at path 3/2"), so the reason is visible and fixable.
- **Generic code bloat.** Exact closures plus specialisation per concrete type can produce many copies. Shared kernels ([[Compression and Canonical Form]] §3.3) are the countermeasure, measured in the benchmarks.

## Related

[[SPEC-002 Sophialang Principles]] · [[Computational Trinitarianism]] · [[Content-Addressed Precompilation]] · [[Milestones]]
