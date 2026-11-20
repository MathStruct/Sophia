# E-7 (B-028): staged compilation of a small Lux training step with Reactant.
# See lux_train_step.md and [[Semantic Boundary Experiments]] §E-7.
#
#   julia112 --project=. lux_train_step.jl run DIR [perturb]   # compile, time, train, save
#   julia112 --project=. lux_train_step.jl load DIR            # fresh process: restore, train, compare
#   julia112 --project=. lux_train_step.jl plain               # the same step without Reactant
#   julia112 --project=. lux_train_step.jl compare DIR...      # which layers are bit-identical
#
# One training step is: loss of a 3-layer MLP on a fixed batch, its gradient
# (Enzyme reverse mode, inside Reactant's trace), and one Adam update. Each
# `run` saves the three layers of the compiled step:
#   stablehlo.mlir  the module Reactant hands to XLA (`serializable=true`), with locations
#   hlo.txt         XLA's optimised HLO module, read back from the executable
#   executable.bin  the PJRT-serialised executable (CPU machine code)
# plus the Julia glue and argument types (thunk.jls, as in E-AD-1's
# reactant_persist.jl) and the training trajectory (losses and final parameters).
# `perturb` compiles an unrelated model first, so the session history differs.
const t_start = time()
using Lux, Reactant, Enzyme, Optimisers, Random, Serialization, Printf, SHA
const XLA = Reactant.XLA
const API = Reactant.MLIR.API

const BATCH = 64
const STEPS = 200
const model = Chain(Dense(4 => 32, tanh), Dense(32 => 32, tanh), Dense(32 => 1))

function data(batch)
    x = randn(Xoshiro(1), Float32, 4, batch)
    y = sum(sin, x; dims = 1) .+ 0.1f0 .* x[1:1, :] .^ 2
    return x, y
end
init() = Lux.setup(Xoshiro(0), model)       # deterministic, so every process starts the same

function loss(model, ps, st, x, y)
    ŷ, _ = model(x, ps, st)
    return sum(abs2, ŷ .- y) / size(y, 2)
end

function train_step(model, ps, st, opt_state, x, y)
    dps = Enzyme.make_zero(ps)
    _, l = Enzyme.autodiff(Enzyme.ReverseWithPrimal, loss, Active, Const(model), Duplicated(ps, dps), Const(st), Const(x), Const(y))
    opt_state, ps = Optimisers.update(opt_state, ps, dps)
    return ps, opt_state, l
end

ms(t) = t < 1e-3 ? @sprintf("%9.1f µs", 1e6t) : @sprintf("%9.1f ms", 1e3t)
hex(b) = bytes2hex(sha256(b))[1:16]
flat(ps) = Optimisers.destructure(ps |> cpu_device())[1]

# Trains STEPS steps with a compiled step `c`; returns the losses and the final parameters.
function train(c, ps, st, os, x, y)
    losses = Float32[]
    times = Float64[]
    for _ in 1:STEPS
        t = @elapsed begin
            ps, os, l = c(model, ps, st, os, x, y)
            l = Float32(l)                     # waits for the device
        end
        push!(losses, l)
        push!(times, t)
    end
    return losses, flat(ps), times
end

function serialize_exec(exec)
    size = Ref{Csize_t}(0)
    p = API.PjRtLoadedExecutableSerialize(exec.exec, size)
    return copy(unsafe_wrap(Vector{UInt8}, Ptr{UInt8}(p), size[]))
end
hlo_text(exec) = string(XLA.get_hlo_modules(exec)[1])

# The same step on Julia arrays (Enzyme on Julia's own code, no Reactant).
# Timed from the top level: calling plain() makes Julia infer train_step, and with
# it Enzyme's derivative, before the first line of plain() runs.
function plain(t_loaded, t_enter)
    x, y = data(BATCH)
    ps, st = init()
    os = Optimisers.setup(Adam(0.01f0), ps)
    out = train_step(model, ps, st, os, x, y)
    t_first = time() - t_enter
    times = Float64[]
    losses = Float32[]
    for _ in 1:STEPS
        t = @elapsed ((ps, os, l) = train_step(model, ps, st, os, x, y))
        push!(times, t); push!(losses, l)
    end
    println("E-7 plain (no Reactant): Julia + packages loaded ", ms(t_loaded))
    println("first step, from the call of plain() (Julia compiles the step and Enzyme's derivative): ", ms(t_first))
    println("step, min / median of ", STEPS, ": ", ms(minimum(times)), " / ", ms(sort(times)[div(end, 2)]))
    println("loss: first ", out[3], ", last ", losses[end])
end

mode = ARGS[1]
if mode == "run"
    dir = ARGS[2]
    perturb = length(ARGS) >= 3 && ARGS[3] == "perturb"
    mkpath(dir)
    t_loaded = time() - t_start
    dev = reactant_device()
    x, y = data(BATCH) |> dev
    ps0, st0 = init()
    ps, st = (ps0, st0) |> dev
    os = Optimisers.setup(Adam(0.01f0), ps)

    # Reactant's start-up on an unrelated function, as in E-AD-1.
    warm(v) = sum(abs2, v)
    t_session = @elapsed (Reactant.@compile warm(x))(x)
    if perturb                                   # another model's step, first
        m2 = Chain(Dense(4 => 8, relu), Dense(8 => 1))
        p2, s2 = Lux.setup(Xoshiro(5), m2) |> dev
        o2 = Optimisers.setup(Descent(0.1f0), p2)
        step2(m, p, s, o, x, y) = (d = Enzyme.make_zero(p);
            Enzyme.autodiff(Enzyme.Reverse, loss, Active, Const(m), Duplicated(p, d), Const(s), Const(x), Const(y));
            Optimisers.update(o, p, d))
        Reactant.@compile step2(m2, p2, s2, o2, x, y)
    end

    Base.cumulative_compile_timing(true)
    c0 = Base.cumulative_compile_time_ns()[1]
    t_hlo1 = @elapsed Reactant.@code_hlo train_step(model, ps, st, os, x, y)
    jl1 = (Base.cumulative_compile_time_ns()[1] - c0) / 1e9
    t_hlo2 = @elapsed Reactant.@code_hlo train_step(model, ps, st, os, x, y)
    t_comp = @elapsed (c = Reactant.@compile serializable = true train_step(model, ps, st, os, x, y))
    t_run1 = @elapsed (Float32(c(model, ps, st, os, x, y)[3]))
    losses, final, times = train(c, ps, st, os, x, y)
    t_comp2 = @elapsed (c2 = Reactant.@compile serializable = true train_step(model, ps, st, os, x, y))

    exe = serialize_exec(c.exec)
    exe2 = serialize_exec(c2.exec)
    shlo = c.module_string
    hlo = hlo_text(c.exec)
    # Another batch size: a separate artifact (shapes are part of the key).
    xb, yb = data(2BATCH) |> dev
    t_compb = @elapsed (cb = Reactant.@compile serializable = true train_step(model, ps, st, os, xb, yb))
    exeb = serialize_exec(cb.exec)

    tag = Reactant.Compiler.get_tag(typeof(c))
    body = Reactant.Compiler.__thunk_fwd_body_cache[tag]
    exec = c.exec
    meta = (
        tag = tag, body = body,
        argtypes = Reactant.Compiler.get_compiled_argtypes(typeof(c)),
        isclosure = Reactant.Compiler.get_isclosure(typeof(c)),
        num_outputs = exec.num_outputs, num_parameters = exec.num_parameters,
        is_sharded = exec.is_sharded, num_replicas = exec.num_replicas, num_partitions = exec.num_partitions,
        donated = c.donated_args_mask, gdids = c.global_device_ids, sync = c.compiled_with_sync,
        julia = VERSION, reactant = pkgversion(Reactant), reactant_jll = pkgversion(Reactant.Reactant_jll),
    )
    serialize(joinpath(dir, "thunk.jls"), meta)
    write(joinpath(dir, "executable.bin"), exe)
    write(joinpath(dir, "executable-again.bin"), exe2)
    write(joinpath(dir, "stablehlo.mlir"), shlo)
    write(joinpath(dir, "hlo.txt"), hlo)
    serialize(joinpath(dir, "trajectory.jls"), (losses = losses, final = final))
    glue = let io = IOBuffer(); serialize(io, body); take!(io) end

    st_sorted = sort(times)
    println("E-7 run, ", perturb ? "perturbed session" : "plain session", "; Julia ", VERSION, ", Lux ", pkgversion(Lux),
        ", Reactant ", pkgversion(Reactant), ", Enzyme ", pkgversion(Enzyme))
    println("parameters: ", length(final), " Float32; batch ", BATCH, "; ", STEPS, " steps")
    println("Julia + packages loaded:          ", ms(t_loaded))
    println("Reactant session start-up:        ", ms(t_session))
    println("derive StableHLO, first:          ", ms(t_hlo1), "  (Julia compiling: ", ms(jl1), ")")
    println("derive StableHLO, Julia warm:     ", ms(t_hlo2))
    println("@compile (derive + XLA):          ", ms(t_comp), "  => XLA compile ≈ ", ms(t_comp - t_hlo2))
    println("@compile again, same process:     ", ms(t_comp2))
    println("@compile, batch ", 2BATCH, ":              ", ms(t_compb))
    println("first execution:                  ", ms(t_run1))
    println("step, min / median of ", STEPS, ":       ", ms(st_sorted[1]), " / ", ms(st_sorted[div(end, 2)]))
    println("loss: first ", losses[1], ", last ", losses[end])
    @printf("sizes: StableHLO %d B, HLO %d B, executable %d B (batch %d: %d B), glue %d B\n",
        sizeof(shlo), sizeof(hlo), length(exe), 2BATCH, length(exeb), length(glue))
    println("same process, compiled twice: executable identical: ", exe == exe2,
        ", StableHLO identical: ", shlo == c2.module_string, ", HLO identical: ", hlo == hlo_text(c2.exec))
    println("sha256/16: stablehlo ", hex(shlo), "  hlo ", hex(hlo), "  executable ", hex(exe), "  glue ", hex(glue),
        "  final params ", hex(reinterpret(UInt8, final)))
elseif mode == "load"
    dir = ARGS[2]
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
        Reactant.Compiler.__thunk_fwd_body_cache[meta.tag] = meta.body
        Reactant.Compiler.__thunk_rev_body_cache[meta.body] = meta.tag
        thunk = Reactant.Compiler.Thunk{typeof(train_step), meta.tag, meta.isclosure, meta.argtypes, typeof(exec), typeof(device),
            typeof(client), typeof(meta.gdids), typeof(meta.donated)}(
            train_step, exec, device, read(joinpath(dir, "stablehlo.mlir"), String), client, meta.gdids, meta.donated, meta.sync)
    end
    dev = reactant_device()
    t_data = @elapsed begin
        x, y = data(BATCH) |> dev
        ps, st = init() |> dev
        os = Optimisers.setup(Adam(0.01f0), ps)
    end
    t_run1 = @elapsed (Float32(thunk(model, ps, st, os, x, y)[3]))
    losses, final, times = train(thunk, ps, st, os, x, y)
    saved = deserialize(joinpath(dir, "trajectory.jls"))
    println("E-7 load (fresh process) of ", dir)
    println("Julia + packages loaded:          ", ms(t0 - t_start))
    println("restore executable + glue:        ", ms(t_restore))
    println("parameters to device:             ", ms(t_data))
    println("first execution after restore:    ", ms(t_run1))
    println("step, min / median of ", STEPS, ":       ", ms(minimum(times)), " / ", ms(sort(times)[div(end, 2)]))
    bits(v) = reinterpret(UInt8, v)              # `==` would accept -0.0 for 0.0
    println("same losses as the saving process, bitwise:  ", bits(losses) == bits(saved.losses))
    println("same final parameters, bitwise:              ", bits(final) == bits(saved.final), " (max |Δ| = ", maximum(abs.(final .- saved.final)), ")")
    t_fresh = @elapsed (c = Reactant.@compile train_step(model, ps, st, os, x, y))
    println("for comparison, @compile here:    ", ms(t_fresh))
elseif mode == "plain"
    t_enter = time()
    plain(t_enter - t_start, t_enter)
elseif mode == "compare"
    dirs = ARGS[2:end]
    length(unique(realpath.(dirs))) >= 2 || error("compare needs at least two distinct run directories")
    files = ["stablehlo.mlir", "hlo.txt", "executable.bin"]
    @printf("%-40s %-18s %-18s %-18s %s\n", "run", files..., "trajectory")
    for d in dirs
        tr = deserialize(joinpath(d, "trajectory.jls"))
        @printf("%-40s %-18s %-18s %-18s %s\n", d, (hex(read(joinpath(d, f))) for f in files)...,
            hex(vcat(reinterpret(UInt8, tr.losses), reinterpret(UInt8, tr.final))))
    end
end
