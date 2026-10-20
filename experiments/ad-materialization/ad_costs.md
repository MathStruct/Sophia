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
