#experiment #julia

> Control for [[pkgimage_reuse]]. E-AD-1, Julia-native half: an Enzyme gradient compiled into a package image (`ADCache`, PrecompileTools workload) is called in a fresh process. Observed 2026-10-20: a segmentation fault at the first call, reproducibly; the control (`pkgimage_control.jl`, same package, function not precompiled) works. Sources: `pkgimage_reuse.jl`, `pkgimage_control.jl`, `ADCache/src/ADCache.jl`. Results: `results-persist.txt`.
