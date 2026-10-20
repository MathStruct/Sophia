#experiment #julia

> E-AD-1: saves a Reactant-compiled derivative (XLA executable, the Julia glue that calls it, its argument types, and the StableHLO module) and restores and runs it in a fresh Julia process. Also splits the first call's time into its parts. Source: `reactant_persist.jl`. Results: `results-persist.txt`, [[AD as a Materialization Case]].
>
> Sources: Reactant.jl source (`compiler/Thunk.jl`, `xla/PJRT/LoadedExecutable.jl`, the PJRT serialize calls in `mlir/libMLIR_h.jl`). code: `reactant_persist.jl`

The restore uses Reactant internals (`Compiler.Thunk`, `__thunk_fwd_body_cache`, `MLIR.API.PjRtClientLoadSerializedExecutable`), not a public API. It refuses an artifact made with another Julia, Reactant or Reactant_jll version. The executable is host-specific machine code; the StableHLO module is the portable form.
