# Frontend

Julia code → Sophia Core terms. Source: `Frontend.jl` (comments only). Milestone: [[Roadmap|M3]].

## Why this must be written in Julia

`Meta.lower`, `code_typed`, `CodeInfo`, `Core.MethodInstance` and the method tables are unreachable from outside a running Julia process, and an external "Julia parser" would be a reimplementation that drifts from the real one. This constraint is the main reason the repository has a Julia half at all ([[Repository Layout]]).

## Ingest lowered IR, and typed IR separately

[[Julia Lowered IR|Lowered IR]] is the primary ingestion point: it is desugared, so the frontend does not reimplement Julia's desugaring, and it is still *generic* rather than specialised to one argument-type tuple.

Typed IR is ingested too, but as a different kind of node — one per specialisation, with its dispatch decisions recorded. It is not the definition; it is a compiled view of the definition under a [[World Age|world age]]. Conflating the two would make a definition's identity depend on which specialisations happened to be compiled.

## What must be made explicit

Everything Julia leaves implicit, because the hash is taken on the result. The full list is in the source; four are worth repeating:

- **Dispatch** is a fact about the method table, not about the syntax. Recorded as `DispatchFact` nodes so invalidation can later be computed exactly ([[Content-Addressed Precompilation]], [[Multiple Dispatch]]).
- **Overflow**: Julia's `+` on `Int64` wraps. Emit `Wrap`, not a generic add. This is the single most likely place for a false Julia/C++ equivalence to enter the store ([[Cross-Language Semantic Hazards]]).
- **`@inbounds`, `@fastmath`, `@simd`** are unchecked promises with no formal statement anywhere. Here each becomes an explicit attribute or an assumption node *that someone signed for* — which is arguably an improvement on the status quo, where the promise is unattributable and buried in a source line.
- **Indexing and layout**: 1-based and column-major, elaborated to explicit linear-index arithmetic, or no comparison with C code means anything.

## The subset boundary is the design

Julia will not be covered. `eval` and generated functions, arbitrary `ccall`, tasks and threads, `unsafe_wrap`/`pointer`/`reinterpret`, finalizers and world-age-crossing `invokelatest` are all out of the first scope.

The boundary must be explicit and ingestion outside it must **fail rather than guess** — a guessed meaning is worse than no ingestion, because it will be hashed, stored, trusted and compiled.

## Validation without a semantics

There is no formal Julia semantics to check the elaboration against ([[State of the Art - Formal Semantics of Real Languages]]). Differential testing — run the function, evaluate the extracted core term, compare on generated inputs — is the only option, and that suite *is* the specification of what the ingested subset means. It should exist before the frontend grows.

## Related

- [[Julia Lowered IR]] · [[Multiple Dispatch]] · [[World Age]]
- [[Core Calculus]] · [[elaborate]] · [[Effects Memory and Resources]]
- [[Cross-Language Semantic Hazards]] · [[Content-Addressed Precompilation]]
