# Run in a fresh process after ADCache is precompiled:
#   julia112 --project=. pkgimage_reuse.jl
t0 = time()
using ADCache
t_load = time() - t0
x = Float64[sin(i) for i in 1:1000]
a = @elapsed ADCache.grad(ADCache.f_cached, x)
b = @elapsed ADCache.grad(ADCache.f_plain, x)
s = minimum(@elapsed(ADCache.grad(ADCache.f_cached, x)) for _ in 1:50)
println("load ADCache (with Enzyme):                 ", round(1e3t_load; digits=1), " ms")
println("first gradient, precompiled in pkgimage:   ", round(1e3a; digits=1), " ms")
println("first gradient, not precompiled:           ", round(1e3b; digits=1), " ms")
println("steady gradient:                           ", round(1e6s; digits=1), " µs")
