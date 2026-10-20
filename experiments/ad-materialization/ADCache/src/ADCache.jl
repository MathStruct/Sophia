# E-AD-1, Julia-native half: does a derivative compiled during package
# precompilation survive into a fresh process (Julia's pkgimage), so the
# first gradient call there is fast? `f_cached` is differentiated in the
# precompile workload, `f_plain` is not. See ad_costs.md.
module ADCache
using Enzyme, PrecompileTools
f_cached(x) = sum(abs2, x) * sin(sum(x[1:1]))
f_plain(x) = sum(abs2, x) * cos(sum(x[1:1]))
grad(f, x) = Enzyme.gradient(Enzyme.Reverse, f, x)[1]
@setup_workload begin
    x = Float64[sin(i) for i in 1:1000]
    @compile_workload begin
        grad(f_cached, x)
    end
end
end
