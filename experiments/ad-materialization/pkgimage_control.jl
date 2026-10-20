# Control for pkgimage_reuse.jl: the function *not* precompiled, in a fresh
# process. If this works, the crash is in the restored (precompiled) code.
using ADCache
x = Float64[sin(i) for i in 1:1000]
b = @elapsed (g = ADCache.grad(ADCache.f_plain, x))
println("first gradient, not precompiled: ", round(1e3b; digits=1), " ms; finite: ", all(isfinite, g))
