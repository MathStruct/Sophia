#open-problem #julia

> **Status: open (2026-10-20).** Found by [[Julia Conformance]] part 2. Related: [[Julia Name Resolution]], [[SPEC-006 Worlds, Artifacts and Evidence]] §2, [[Julia Multiple Dispatch and World Age]].

**Problem.** Which function a Julia method extends depends on bindings in modules Sophia has not ingested. `Base.Matrix(x::Tridiagonal) = …` adds a constructor to `Core.Array`, because Base's `Matrix` is an alias of Core's `Array{T,2}`. `Base.cbrt(A::AbstractMatrix) = …` extends `Base.Math.cbrt`, which Base re-exports. Reading LinearAlgebra's source cannot tell; the answer is in Base's binding table. On LinearAlgebra this accounts for 86 of the 162 methods whose owner Sophia gets wrong.

The same holds for `using` (a name brought in by `using M` belongs to M's binding, possibly re-exported from elsewhere) and for any package's dependencies.

**Options.**
1. **Ingest Base and Core.** Base is written in Julia, so possible in principle. But Base's bindings are created by bootstrapping (`Core` is built in C), so the source does not give all of them either.
2. **A binding table produced by Julia**, stored as an **environment artifact**: for each module of the environment, each exported or public name and the module that owns the object (`parentmodule`, `Base.binding_module`). Content-addressed, and cited by the facts that used it, like a world. Produced once per (Julia version, Manifest).
3. **Leave it approximate** and flag methods whose owner path is not a module Sophia ingested ("owner not verified").

**Proposal.** Option 2, with option 3 until it exists. It fits SPEC-006: the binding table is part of the world a fact depends on, and it is exactly the kind of fact Sophia should get from Julia rather than infer ([[Semantic Boundary Experiments]] E-1).

**Consequence.** Generic-function entities for external modules (`jl.Base.Matrix`) would be renamed to the real owner (`jl.Core.Array.new`), so all packages extending the same function meet in one entity.
