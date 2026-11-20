# Experiment E-5 (rules vs derived derivatives), Julia's side: for each
# function of a package variant and an argument type, the derived program
# Zygote builds for its gradient, as the text of Zygote's generators for every
# `_pullback` instance the gradient reaches; and, separately, the rule tables
# (methods of `rrule`, `frule`, `no_rrule`, `no_frule`, `ZygoteRules._pullback`,
# `ZygoteRules.adjoint`), read from
# the method table without differentiating anything. See `derivatives.md`.
#
# Usage (from this directory, in the `rules` environment):
#   julia112 --project=rules derivatives.jl rules/variants/<variant> [in-session-file]
# With a second argument, after the variant's rows the file is included into
# the live module and every row is computed again in the same process
# (variant `<variant>+session`).
using Zygote, ChainRulesCore, ChainRules, IRTools, SHA
const ZygoteRules = Zygote.ZygoteRules

const DIR = abspath(ARGS[1])
const VARIANT = basename(DIR)
Base.include(Main, joinpath(DIR, "src", "Rules.jl"))
isfile(joinpath(DIR, "outside.jl")) && Base.include(Main, joinpath(DIR, "outside.jl"))

const PB = ZygoteRules._pullback
const Ctx = Zygote.Context{false}
const TARGETS = [:f1, :f2, :f3, :f4, :g, :sq, :h, :p, :q, :dyn]
const ARGS_ = (0.7, 0.7f0)
const LIMIT = 2000

esc_(s) = replace(s, "\\" => "\\\\", "\n" => "\\n", "\t" => "\\t", "\r" => "\\r")
digest(s) = bytes2hex(sha256(s))
# Closure names are numbered by the session that lowered them
# (`var"#2849#2850"`) and are all replaced; a closure's body is printed where
# it is created (`body`), so two closures differ by what they compute.
clean(s) = replace(s, r"var\"#[^\"]*\"" => "var\"#closure\"")

argtype(ci, a, sig) = a isa Core.SSAValue ? ci.ssavaluetypes[a.id] :
    a isa Core.Argument ? sig[a.n] :
    a isa Core.SlotNumber ? ci.slottypes[a.id] :
    a isa GlobalRef ? Core.Typeof(getglobal(a.mod, a.name)) :
    a isa QuoteNode ? Core.Typeof(a.value) : Core.Typeof(a)

# The `_pullback` instances one instance calls, with inferred argument types
# (untyped calls stay `Any`: runtime dispatch, recorded as such).
function callees(T)
    r = code_typed(PB, Tuple{Ctx, T.parameters...}; optimize = false)
    length(r) == 1 || return Any[]
    ci = r[1][1]
    sig = Any[typeof(PB), Ctx, T.parameters...]
    out = Any[]
    for st in ci.code
        e = st
        e isa Expr && e.head == :(=) && (e = e.args[2])
        (e isa Expr && e.head == :call) || continue
        fn = e.args[1]
        (fn isa GlobalRef && fn.name == :_pullback) || continue
        ts = Any[Core.Compiler.widenconst(argtype(ci, a, sig)) for a in e.args[3:end]]
        push!(out, Tuple{ts...})
    end
    out
end

# A method's lowered body, followed by the bodies of the closures it creates
# (a rule's pullback is a closure: its body is the derivative), recursively.
# Closure names are session-numbered and replaced by `clean`; the bodies stay.
function body(m::Method, seen = Set{Any}())
    m in seen && return ""
    push!(seen, m)
    code = try
        Base.uncompressed_ast(m).code
    catch
        return "generated " * string(m.sig)
    end
    out = join(string.(code), "\n")
    for C in closures(code)
        C in seen && continue
        push!(seen, C)
        sig = Base.rewrap_unionall(Tuple{Base.unwrap_unionall(C), Vararg{Any}}, C)
        ms = Base._methods_by_ftype(sig, -1, Base.get_world_counter())
        ms isa Vector || continue
        for mm in sort!([x.method for x in ms]; by = x -> string(x.sig))
            out *= "\n--- closure\n" * body(mm, seen)
        end
    end
    out
end

isclosure(C) = C isa Union{DataType,UnionAll} && Base.unwrap_unionall(C) <: Function &&
    startswith(string(Base.unwrap_unionall(C).name.name), "#")

# The closure types a lowered body names, in order of appearance.
function closures(code)
    out = Any[]
    visit(x) = if x isa GlobalRef
        isdefined(x.mod, x.name) && (v = getglobal(x.mod, x.name); isclosure(v) && !(v in out) && push!(out, v))
    elseif x isa Union{DataType,UnionAll}
        isclosure(x) && !(x in out) && push!(out, x)
    elseif x isa Expr
        foreach(visit, x.args)
    end
    foreach(visit, code)
    out
end

# The rule method Zygote's `has_chain_rrule` chose for `T`, as Zygote does.
function rule_method(T, world)
    cfg = Zygote.ZygoteRuleConfig{Ctx}
    m = Zygote.meta(Tuple{typeof(rrule), cfg, T.parameters...}; world)
    m === nothing && return "ambiguous"
    if Zygote._is_rrule_redispatcher(m.method)
        m = Zygote.meta(Tuple{typeof(rrule), T.parameters...}; world)
        m === nothing && return "ambiguous"
    end
    string(m.method.module, " ", m.method.sig, "\n", body(m.method))
end

# IR as text: every block with its arguments, statements and branches
# (returns are branches in IRTools).
function ir_text(ir::IRTools.IR)
    out = String[]
    for b in IRTools.blocks(ir)
        push!(out, "block $(b.id) ($(join(string.(IRTools.arguments(b)), ", ")))")
        for (v, st) in b
            push!(out, "$v = $(st.expr)")
        end
        for br in IRTools.branches(b)
            push!(out, "branch " * string(br))
        end
    end
    join(out, "\n")
end
# Two IRs that differ only in what they return must print differently
# (Codex's review of B-026, finding 4).
let a = IRTools.IR(), b = IRTools.IR()
    IRTools.return!(a, 1); IRTools.return!(b, 2)
    @assert ir_text(a) != ir_text(b)
end

# The generic `_pullback` (Zygote's generated method); any other method a
# call dispatches to is a hand-written adjoint (`Zygote.@adjoint`).
generic_probe(x) = x
const GENERIC = which(PB, Tuple{Ctx, typeof(generic_probe), Any})
adjoint_method(T) = (m = which(PB, Tuple{Ctx, T.parameters...}); m === GENERIC ? nothing : m)

hasrule(T, world) = Zygote.has_chain_rrule(Tuple{Zygote.ZygoteRuleConfig{Ctx}, T.parameters...}, world)[1]

# One instance's piece of the derived program, as text, and its kind: the
# hand-written adjoint dispatch selects (`adjoint`), else the rule Zygote's
# generator would call (`rule`), else what its generators make (`ir`: the
# forward pass and the pullback; `expr`: no differentiation, or an error).
function piece(T, world)
    m = adjoint_method(T)
    if m !== nothing
        # `Zygote.@adjoint` makes a `_pullback` method that calls a method of
        # `ZygoteRules.adjoint`, which holds the derivative.
        a = try
            which(ZygoteRules.adjoint, Tuple{Ctx, T.parameters...})
        catch
            nothing
        end
        t = string(m.module, " ", m.sig, "\n", body(m))
        a === nothing || (t *= "\n--- adjoint\n" * string(a.module, " ", a.sig, "\n", body(a)))
        return ("adjoint", t)
    end
    hasrule(T, world) && return ("rule", rule_method(T, world))
    fw = Zygote._generate_pullback(Ctx, world, T.parameters...)
    fw isa Core.CodeInfo || return ("expr", string(fw))
    g = Zygote._generate_pullback_via_decomposition(T, world, :backward)
    bw = g === nothing ? "nothing" : ir_text(g[2])
    ("ir", join(string.(fw.code), "\n") * "\n--- back\n" * bw)
end

# What Zygote does for one instance, as `piece` follows it, without printing:
# dispatch, the rule check, the forward and backward generators.
function transform(T, world)
    adjoint_method(T) === nothing || return true
    hasrule(T, world) && return true
    fw = Zygote._generate_pullback(Ctx, world, T.parameters...)
    fw isa Core.CodeInfo && Zygote._generate_pullback_via_decomposition(T, world, :backward)
    true
end

# The instances reached from `T0`; `dynamic` if one could not be inferred
# (runtime dispatch: what runs is not known before running).
function walk(T0)
    seen = Dict{Any,Nothing}()
    order = Any[]
    todo = Any[T0]
    dynamic = false
    world = Base.get_world_counter()
    while !isempty(todo) && length(order) < LIMIT
        T = popfirst!(todo)
        haskey(seen, T) && continue
        seen[T] = nothing
        push!(order, T)
        if !isconcretetype(T)
            dynamic = true
            continue
        end
        (adjoint_method(T) === nothing && !hasrule(T, world)) || continue
        append!(todo, callees(T))
    end
    order, dynamic ? "dynamic" : length(order) >= LIMIT ? "cut" : "all"
end

const PIECES = Dict{String,String}()

function rows(label)
    world = Base.get_world_counter()
    for x in ARGS_, t in TARGETS
        f = getfield(Rules, t)
        T = Tuple{typeof(f), typeof(x)}
        ns_first = @elapsed (grad = Zygote.gradient(f, x)[1])
        ns_walk = @elapsed ((insts, complete) = walk(T))
        ps = Tuple{String,String,String}[]
        # Zygote's own work, without printing: emptying its decomposition
        # cache first (it is keyed by signature and world, finding 6), then
        # again on the filled cache.
        Zygote.clear_decompositions!()
        ns_gen = @elapsed foreach(I -> isconcretetype(I) && transform(I, world), insts)
        ns_cached = @elapsed foreach(I -> isconcretetype(I) && transform(I, world), insts)
        for I in insts
            k, s = isconcretetype(I) ? piece(I, world) : ("dynamic", "")
            push!(ps, (clean(string(I)), k, clean(s)))
        end
        println(join(["derived", label, t, typeof(x), length(insts), complete,
                      round(Int, 1e9ns_first), round(Int, 1e9ns_walk), round(Int, 1e9ns_gen), round(Int, 1e9ns_cached),
                      repr(grad === nothing ? nothing : Float64(grad))], '\t'))
        for (sig, k, s) in ps
            d = digest(s)
            haskey(PIECES, d) || (PIECES[d] = s)
            println(join(["instance", label, t, typeof(x), esc_(sig), k, d], '\t'))
        end
    end
end

# The rule tables, from the method table alone: per method its module, its
# signature and a digest of its lowered body; `rule` rows for the methods
# whose signature names one of the package's functions.
function tables(label)
    own = Set(typeof(getfield(Rules, n)) for n in names(Rules; all = true)
              if isdefined(Rules, n) && getfield(Rules, n) isa Function && !startswith(string(n), "#"))
    for (name, fn) in (("rrule", ChainRulesCore.rrule), ("frule", ChainRulesCore.frule),
                       ("no_rrule", ChainRulesCore.no_rrule), ("no_frule", ChainRulesCore.no_frule),
                       ("_pullback", ZygoteRules._pullback), ("adjoint", ZygoteRules.adjoint))
        ms = sort!([string(m.module, " ", m.sig, " ", digest(clean(body(m)))) for m in methods(fn)])
        println(join(["table", label, name, length(ms), digest(join(ms, "\n"))], '\t'))
        rs = String[]
        for m in methods(fn)
            ps = Base.unwrap_unionall(m.sig).parameters
            any(p -> p in own, ps) || continue
            push!(rs, join(["rule", label, name, string(m.module), esc_(clean(string(m.sig))), digest(clean(body(m)))], '\t'))
        end
        foreach(println, sort!(rs))
    end
end

# After the variant's rows, the second file is included into the live module
# and every row is computed again in the same process. Called through
# `invokelatest`, so that it runs in the world the include made (`main`'s own
# world is fixed when it is entered).
function session(label)
    tables(label)
    rows(label)
    isdefined(Rules, :G_RULE_CALLS) && println("calls\t", label, "\t", Rules.G_RULE_CALLS[])
    # Whether a gradient called after the rule was added uses it, with and
    # without a function Zygote has not compiled for before.
    Rules.G_RULE_CALLS[] = 0
    Zygote.gradient(Rules.g, 0.7)
    println("calls\t", label, "\tg-again\t", Rules.G_RULE_CALLS[])
    Rules.G_RULE_CALLS[] = 0
    Zygote.gradient(x -> Rules.g(x) * 1.0, 0.7)
    println("calls\t", label, "\tfresh-caller\t", Rules.G_RULE_CALLS[])
end

# Warm Zygote up on a function of the same argument types, as E-AD-0 does.
warm(x) = x + one(x)

# The script: run as a program it prints everything; `layers.jl` includes this
# file for its functions and prints Zygote's rows itself (E-6).
function main()
    println("julia\t", VERSION)
    for p in (Zygote, ChainRulesCore, ChainRules, IRTools)
        println("package\t", p, "\t", pkgversion(p))
    end
    Zygote.gradient(warm, 0.7); Zygote.gradient(warm, 0.7f0)
    tables(VARIANT)
    rows(VARIANT)
    isdefined(Rules, :G_RULE_CALLS) && println("calls\t", VARIANT, "\t", Rules.G_RULE_CALLS[])
    if length(ARGS) >= 2
        Base.include(Rules, abspath(ARGS[2]))
        Base.invokelatest(session, VARIANT * "+session")
    end
    for d in sort!(collect(keys(PIECES)))
        println("piece\t", d, "\t", esc_(PIECES[d]))
    end
end

abspath(PROGRAM_FILE) == (@__FILE__) && main()
