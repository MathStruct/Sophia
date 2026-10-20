# E-AD-0: what does it cost to *produce* a derivative program, compared
# with *running* it? The number that decides whether D(f) is worth keeping
# ([[AD as a Materialization Case]]). For each AD system: warm the package
# on an unrelated function of the same argument type, then time the first
# call of the gradient of a fresh function (transformation + compilation of
# D(f)) and the second call (running the compiled D(f)).
#
# Usage: julia --project=@. ad_costs.jl [systems...]   (default: all)
using Printf
const N = 1000
x = rand(N)
warm(x) = sum(x)
fresh = [
    x -> sum(abs2, x) * sin(x[1]),
    x -> sum(exp.(x) .* cos.(x)) / length(x),
    x -> sum(tanh.(x .* x[end])),
]
results = Tuple{String,String,Float64,Float64}[]
function measure(name, grad)
    grad(warm, x); grad(warm, x)
    for (i, f) in enumerate(fresh)
        t1 = @elapsed grad(f, x)
        t2 = minimum(@elapsed(grad(f, x)) for _ in 1:20)
        push!(results, (name, "f$i", t1, t2))
        @printf("%-12s f%d  first %10.3f ms   steady %10.4f ms   ratio %8.0f\n", name, i, 1e3t1, 1e3t2, t1 / t2)
        flush(stdout)
    end
end
want = isempty(ARGS) ? ["ForwardDiff", "ReverseDiff", "Zygote", "Enzyme", "Mooncake", "Reactant"] : ARGS
t0 = time()
if "ForwardDiff" in want
    using ForwardDiff
    measure("ForwardDiff", (f, x) -> ForwardDiff.gradient(f, x))
end
if "ReverseDiff" in want
    using ReverseDiff
    measure("ReverseDiff", (f, x) -> ReverseDiff.gradient(f, x))
    # A compiled tape: the promoted intermediate.
    tape = ReverseDiff.compile(ReverseDiff.GradientTape(fresh[1], x))
    out = similar(x)
    t1 = @elapsed ReverseDiff.compile(ReverseDiff.GradientTape(fresh[2], x))
    t2 = minimum(@elapsed(ReverseDiff.gradient!(out, tape, x)) for _ in 1:20)
    @printf("%-12s compiled tape: build %10.3f ms   steady %10.4f ms\n", "ReverseDiff", 1e3t1, 1e3t2)
end
if "Zygote" in want
    using Zygote
    measure("Zygote", (f, x) -> Zygote.gradient(f, x)[1])
end
if "Enzyme" in want
    using Enzyme
    measure("Enzyme", (f, x) -> Enzyme.gradient(Enzyme.Reverse, f, x)[1])
end
if "Mooncake" in want
    using Mooncake
    measure("Mooncake", (f, x) -> Mooncake.value_and_gradient!!(Mooncake.build_rrule(f, x), f, x)[2][2])
end
# `@eval`: `Reactant.@compile` must not be expanded before Reactant is loaded.
if "Reactant" in want
    @eval using Reactant, Enzyme
    @eval begin
    xr = Reactant.to_rarray(x)
    g(f, x) = Enzyme.gradient(Enzyme.Reverse, f, x)[1]
    for (i, f) in enumerate(fresh)
        tc = @elapsed (c = Reactant.@compile g(f, xr))
        c(f, xr)
        t2 = minimum(@elapsed(c(f, xr)) for _ in 1:20)
        @printf("%-12s f%d  compile %10.3f ms   steady %10.4f ms   ratio %8.0f\n", "Reactant", i, 1e3tc, 1e3t2, tc / t2)
        flush(stdout)
    end
    end
end
@printf("total %.1f s, Julia %s\n", time() - t0, VERSION)
