# sophia_emit

Lowering core terms into [[MLIR]] and [[LLVM IR]]. Source: `sophia_emit.rs` (comments only).

## A translator, not a compiler

[[Start Here]] is explicit: "the compilation goal should be LLVM/MLIR as I do not want to get into the weeds of writing a compiler backend." That is the right call and this crate honours it — it gets [[Core Calculus|SC]] terms into a form someone else optimises and codegens, and writes the result back into the store.

## Why a `sophia` dialect first

Lowering goes SC → a `sophia` [[MLIR Dialect|dialect]] that renders SC 1:1 → standard dialects (`func`, `arith`, `scf`, `cf`) → `memref`/`llvm` → [[LLVM IR]]. The extra step earns its keep: it makes the SC→MLIR boundary a mechanical, auditable translation and pushes every semantic decision into ordinary MLIR conversion patterns, where existing tooling can inspect them. See [[MLIR Lowering]].

## Recording the pipeline

`LoweringResult` carries the pass pipeline that ran. This is what turns "why did the optimiser do that?" from a rerun with `-print-after-all` into a query over stored `LOWERS_TO` edges — a capability that is useful on day one, independently of whether anything else in the project works out. See [[Compilation as Query]].

## The decision this crate has to make with numbers

[[Graph Schema]] estimates $10^7$–$10^8$ nodes per package if every IR layer is persisted. Three options — persist everything, persist chunked per-function blobs, or regenerate on demand — and [[Roadmap|M2]] should pick between them empirically rather than by preference.

Going-in assumption: chunked blobs for MLIR, regenerate for LLVM IR. LLVM IR is the largest layer and the cheapest to reproduce from MLIR.

## Everything below this crate is untrusted

MLIR and LLVM are enormous and will not be verified. Consequently all guarantees are stated about core terms, not about the binary; per-instance translation validation is the realistic mitigation ([[State of the Art - Program Equivalence Checking]]); and a lowering with no validation [[witness]] must *lower* the artifact's trust label rather than silently inheriting the core term's. See [[Trusted Computing Base]].

Alive2-style validation fits this architecture unusually well: it is per-instance, it produces a citable artifact, and that artifact is exactly the shape of a node.

## Targets are nodes, not flags

Triple, datalayout, CPU features, ABI choices and float environment all hash into `h_target`, which is part of the codegen cache key ([[Content-Addressed Precompilation]]). Two builds differing only in `-mcpu` must not share compiled code; making the target a node is what enforces that. The toolchain version belongs in the pipeline hash for the same reason — otherwise the cache is unsound across an LLVM upgrade.

## Related

- [[MLIR]] · [[MLIR Dialect]] · [[MLIR Lowering]] · [[LLVM IR]] · [[LLVM Bitcode]]
- [[Compilation as Query]] · [[Trusted Computing Base]]
- [[State of the Art - IR Interchange Formats]]
