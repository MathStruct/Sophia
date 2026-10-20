#experiment #julia

> E-AD-0: the cost of producing a derivative program (transformation + compilation of D(f)) against the cost of running it, per Julia AD system. Source: `ad_costs.jl`. Results and discussion: [[AD as a Materialization Case]].
>
> Sources: [[AD as a Materialization Case]]; [[Storage Measurements]] M-6. code: `ad_costs.jl`, `results.txt`

See `vault/Storage and Materialization/AD as a Materialization Case.md`.

```sh
J=julia   # 1.12: the AD packages' binaries installed here resolve offline for 1.12
$J --project=. -e 'using Pkg; Pkg.offline(true); Pkg.add(["ForwardDiff","ReverseDiff","Zygote","Enzyme","Mooncake"])'
$J --project=. ad_costs.jl ForwardDiff ReverseDiff Zygote Enzyme Mooncake > results.txt
```

`results.txt` is the run of 2026-10-20 (Julia 1.12.7, Linux x86-64), all six systems.

Reactant: `julia112 --project=. -e 'using Pkg; Pkg.add(["Reactant", "Reactant_jll"])'`, then
`ad_costs.jl Reactant`. `Reactant_jll` is a direct dependency only so that
`LocalPreferences.toml` (`gpu = "none"`, the CPU build) applies to it: Julia reads preferences
only for packages the project lists. Without it, Reactant picks its ROCm build for the
integrated AMD GPU and fails to load.

## E-AD-1: restoring a compiled derivative in a fresh process

`reactant_persist.jl save DIR` compiles D(f) with Reactant, times the parts of the first call, and saves the three things a Reactant thunk is: the XLA executable (PJRT `PjRtLoadedExecutableSerialize`), the generated Julia glue that calls it (from Reactant's `__thunk_fwd_body_cache`), and its argument types; plus the StableHLO module as the portable form. `reactant_persist.jl load DIR` runs in a new process, refuses an artifact from another Julia or Reactant version, rebuilds the thunk from the files, runs it and compares the gradient. `ADCache/` and `pkgimage_reuse.jl` try the Julia-native route: an Enzyme gradient compiled into a package image by a PrecompileTools workload; `pkgimage_control.jl` is the control. Results: `results-persist.txt`; discussion in [[AD as a Materialization Case]].
