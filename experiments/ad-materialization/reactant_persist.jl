# E-AD-1: can a compiled derivative be persisted and run in a *fresh* Julia
# process? And where does the first-call time go? See ad_costs.md and
# [[AD as a Materialization Case]].
#
#   julia112 --project=. reactant_persist.jl save DIR   # compile, time the parts, save
#   julia112 --project=. reactant_persist.jl load DIR   # new process: restore, run, compare
#
# A Reactant thunk is three things, and all three must be restored:
#   - the XLA executable (machine code), serialized through PJRT;
#   - the Julia glue that flattens arguments and rebuilds results (generated
#     at compile time, kept in Reactant's `__thunk_fwd_body_cache`);
#   - the argument types it was compiled for.
# The StableHLO module (the derived program, before XLA) is saved too, as the
# portable fallback.
const t_start = time()
using Reactant, Enzyme, Serialization, Printf
const XLA = Reactant.XLA
const API = Reactant.MLIR.API

const N = 1000
f(x) = sum(abs2, x) * sin(sum(x[1:1]))          # the function differentiated
g(x) = Enzyme.gradient(Enzyme.Reverse, f, x)[1]   # its derivative D(f)
input() = Float64[sin(i) for i in 1:N]           # deterministic, so both processes agree

mode, dir = ARGS[1], ARGS[2]
mkpath(dir)
ms(t) = t < 1e-3 ? @sprintf("%9.1f µs", 1e6t) : @sprintf("%9.1f ms", 1e3t)

if mode == "save"
    xr = Reactant.to_rarray(input())
    # Reactant's own start-up (first use in a session), on an unrelated
    # function, so that "first" below is first for D(f), as in ad_costs.jl.
    warm(x) = Enzyme.gradient(Enzyme.Reverse, x -> sum(x), x)[1]
    t_session = @elapsed (Reactant.@compile warm(xr))(xr)
    println("session start-up (unrelated fn):  ", ms(t_session))
    # Where the first call's time goes.
    Base.cumulative_compile_timing(true)
    c0 = Base.cumulative_compile_time_ns()[1]
    t_hlo1 = @elapsed (hlo = Reactant.@code_hlo g(xr)) # trace + AD (Enzyme-MLIR) + passes, first time
    jl1 = (Base.cumulative_compile_time_ns()[1] - c0) / 1e9
    t_hlo2 = @elapsed Reactant.@code_hlo g(xr)       # the same with Julia's code already compiled
    t_comp = @elapsed (c = Reactant.@compile g(xr))  # trace + passes + XLA compilation
    t_run1 = @elapsed (y = Array(c(xr)))
    t_run = minimum(@elapsed(c(xr)) for _ in 1:50)
    println("derive StableHLO, first:          ", ms(t_hlo1), "  (of which Julia compiling the tracer: ", ms(jl1), ")")
    println("derive StableHLO, Julia warm:     ", ms(t_hlo2))
    println("@compile (derive + XLA):          ", ms(t_comp), "  => XLA compile ≈ ", ms(t_comp - t_hlo2))
    println("first execution:                  ", ms(t_run1))
    println("steady execution:                 ", ms(t_run))

    exec = c.exec
    println("executable type: ", typeof(exec))
    size = Ref{Csize_t}(0)
    p = API.PjRtLoadedExecutableSerialize(exec.exec, size)
    bytes = copy(unsafe_wrap(Vector{UInt8}, Ptr{UInt8}(p), size[]))
    write(joinpath(dir, "executable.bin"), bytes)
    tag = Reactant.Compiler.get_tag(typeof(c))
    body = Reactant.Compiler.__thunk_fwd_body_cache[tag]
    meta = (
        tag = tag,
        body = body,
        argtypes = Reactant.Compiler.get_compiled_argtypes(typeof(c)),
        isclosure = Reactant.Compiler.get_isclosure(typeof(c)),
        num_outputs = exec.num_outputs, num_parameters = exec.num_parameters,
        is_sharded = exec.is_sharded, num_replicas = exec.num_replicas, num_partitions = exec.num_partitions,
        donated = c.donated_args_mask, gdids = c.global_device_ids, sync = c.compiled_with_sync,
        julia = VERSION, reactant = pkgversion(Reactant), reactant_jll = pkgversion(Reactant.Reactant_jll),
    )
    serialize(joinpath(dir, "thunk.jls"), meta)
    write(joinpath(dir, "module.mlir"), string(hlo))
    serialize(joinpath(dir, "expected.jls"), y)
    @printf("saved: executable %d bytes, StableHLO %d bytes, glue %d bytes\n",
        length(bytes), filesize(joinpath(dir, "module.mlir")), filesize(joinpath(dir, "thunk.jls")))
elseif mode == "load"
    t0 = time()
    meta = deserialize(joinpath(dir, "thunk.jls"))
    if (meta.julia, meta.reactant, meta.reactant_jll) != (VERSION, pkgversion(Reactant), pkgversion(Reactant.Reactant_jll))
        error("artifact built with Julia $(meta.julia), Reactant $(meta.reactant)/$(meta.reactant_jll); refusing to load")
    end
    client = XLA.default_backend()
    device = XLA.default_device(client)
    bytes = read(joinpath(dir, "executable.bin"))
    t_restore = @elapsed begin
        p = GC.@preserve bytes API.PjRtClientLoadSerializedExecutable(client.client, pointer(bytes), length(bytes), C_NULL, 0)
        exec = XLA.PJRT.LoadedExecutable(p, meta.num_outputs, meta.num_parameters, meta.is_sharded, meta.num_replicas, meta.num_partitions)
        # The glue: registered under its tag, so the generated call method finds it.
        Reactant.Compiler.__thunk_fwd_body_cache[meta.tag] = meta.body
        Reactant.Compiler.__thunk_rev_body_cache[meta.body] = meta.tag
        thunk = Reactant.Compiler.Thunk{typeof(g), meta.tag, meta.isclosure, meta.argtypes, typeof(exec), typeof(device),
            typeof(client), typeof(meta.gdids), typeof(meta.donated)}(
            g, exec, device, read(joinpath(dir, "module.mlir"), String), client, meta.gdids, meta.donated, meta.sync)
    end
    t_xr = @elapsed (xr = Reactant.to_rarray(input()))
    t_run1 = @elapsed (y = Array(thunk(xr)))
    t_run = minimum(@elapsed(thunk(xr)) for _ in 1:50)
    expected = deserialize(joinpath(dir, "expected.jls"))
    println("Julia + Reactant loaded:          ", ms(t0 - t_start))
    println("restore executable + glue:        ", ms(t_restore))
    println("first device array:               ", ms(t_xr))
    println("first execution after restore:    ", ms(t_run1))
    println("steady execution:                 ", ms(t_run))
    println("same gradient as the saving process: ", y == expected, " (max |Δ| = ", maximum(abs.(y .- expected)), ")")
    # Compare: deriving and compiling again from scratch in this process.
    t_fresh = @elapsed (c = Reactant.@compile g(xr))
    println("for comparison, @compile here:    ", ms(t_fresh))
    println("process total so far:             ", ms(time() - t0))
end
