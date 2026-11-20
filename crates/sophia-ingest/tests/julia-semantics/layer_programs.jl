# Experiment E-6 (one derivative, two layers), Julia's side: for each function
# of the E-5 fixture package and an argument type, the derived program of its
# gradient at two layers, and whether the two compute the same function.
#   - Julia IR: Zygote's program, printed by `derivatives.jl`'s functions in
#     this process (its `derived`, `instance`, `table`, `rule`, `piece` lines);
#   - LLVM IR: the module Enzyme builds for reverse mode
#     (`Enzyme.Compiler.enzyme_code_llvm`), with the addresses of Julia objects
#     it embeds replaced by what they point to and session-numbered names
#     renumbered (`enzyme`, `module` lines);
#   - agreement: both gradients at fixed points against a central difference
#     of f in 512-bit BigFloat (`point` lines).
# See `layer_programs.md`.
#
# Usage (from this directory, in the `layers` environment):
#   julia112 --project=layers layer_programs.jl rules/variants/base [reverse]
include(joinpath(@__DIR__, "derivatives.jl"))   # loads the variant (ARGS[1]) and Zygote
using Enzyme

const POINTS = (0.1, 0.35, 0.7, 1.3, 1.9, 2.6, 3.4, -0.45, -1.7, 5.2)

addr(x) = UInt(ccall(:jl_value_ptr, Ptr{Cvoid}, (Any,), x))

# What an embedded address points to. Only a literal in a position that holds
# a pointer is read (an integer constant in the same range is data; Codex's
# review, finding 1):
#   - a call through `inttoptr (i64 N to ptr)(…)` jumps to native code: named by
#     Julia's symbol lookup, or left as it is when the lookup has no name
#     (finding 2);
#   - an `inttoptr (i64 N to ptr)` value, a type tag (`store atomic i64 N, ptr
#     %"….tag_addr"`, the last argument of `@ijl_gc_…alloc…(…, i64 N)`), an
#     `enzymejl_mi`, `enzymejl_rt` or `enzymejl_parmtype` attribute: a
#     `jl_value_t` pointer.
function describe_code(p::UInt)
    fr = Base.StackTraces.lookup(Ptr{Cvoid}(p))
    isempty(fr) && return nothing
    name = string(fr[1].func)
    isempty(name) || name == "???" ? nothing : "code " * name
end

# A type, a method instance, a function without state and a string, symbol or
# number are named by what they are. Anything else (a closure with captures, a
# struct) is `obj`, its type and its `repr`, which shows its fields (finding
# 3): two such objects are told apart, though the text may then depend on the
# session; `obj` is not counted as named.
function describe(p::UInt)
    o = unsafe_pointer_to_objref(Ptr{Cvoid}(p))
    o isa Type && return "type " * string(o)
    o isa Core.MethodInstance && return "mi " * string(o.specTypes)
    o isa Core.CodeInstance && return "ci " * string(o.def isa Core.MethodInstance ? o.def.specTypes : o.def)
    o isa Function && Base.issingletontype(typeof(o)) && return "fn " * string(typeof(o))
    o isa Union{String,Symbol,Number} && return string(typeof(o), " ", repr(o))
    "obj " * string(typeof(o)) * " " * repr(o; context = :limit => false)
end

# (pattern, how to name its address); each pattern has the groups `pre`, `n`,
# `post`. A call is tried before a value.
const POINTERS = [
    (r"(?<pre>inttoptr \(i64 )(?<n>[0-9]+)(?<post> to ptr\)\()", describe_code),
    (r"(?<pre>inttoptr \(i64 )(?<n>[0-9]+)(?<post> to ptr\))(?!\()", describe),
    (r"(?<pre>\"enzymejl_(?:mi|rt|parmtype)\"=\")(?<n>[0-9]+)(?<post>\")", describe),
    (r"(?<pre>store atomic i64 )(?<n>[0-9]+)(?<post>, ptr %\"[^\"]*tag_addr\")", describe),
    (r"(?<pre>@ijl_gc_[a-z_]*alloc[a-z_]*\([^()]*, i64 )(?<n>[0-9]+)(?<post>\))", describe),
]
# Names the session numbers, in names only: global (`@julia_sin_12951`,
# `@diffejulia_f1_10492wrap`, `@"preprocess_julia_g_123"`), local and block
# labels named after an inlined function (`%diffejulia_p_25490.exit`, and the
# label's definition at the start of a line). A string constant that spells
# one is data (finding 4).
const NUMBERED = r"(?<q>(?:@|%|^)\"?)(?<name>(?:julia|jlcapi|j|tojlinvoke|jfptr|diffe|augmented|preprocess)[A-Za-z0-9_.!#]*?)_(?<n>[0-9]+)(?=wrap|\b|\"|\()"m

# The module text with addresses and session numbers replaced; returns the
# text, how many distinct addresses it had and how many were named.
function normalise(raw)
    names = Dict{Tuple{Function,UInt},Union{Nothing,String}}()
    text = raw
    for (re, name) in POINTERS
        text = replace(text, re => function (m)
            c = match(re, m)
            n = parse(UInt, c[:n])
            n < 4096 && return m                  # null and small constants
            d = get!(() -> name(n), names, (name, n))
            d === nothing ? m : c[:pre] * "{" * d * "}" * c[:post]
        end)
    end
    nums = Dict{String,Int}()
    text = replace(text, NUMBERED => function (m)
        c = match(NUMBERED, m)
        k = get!(nums, c[:n], length(nums) + 1)
        c[:q] * c[:name] * "_#" * string(k)
    end)
    addresses = Set(n for (_, n) in keys(names))
    named = Set(n for ((_, n), d) in names if d !== nothing && !startswith(d, "obj"))
    text, length(addresses), length(named)
end

# Codex's review findings 1 to 4 and 8, on synthetic text.
let mk(a) = x -> a * x
    c2, c3 = mk(2), mk(3)
    @assert normalise("  ret i64 17592186044416") == ("  ret i64 17592186044416", 0, 0)
    @assert normalise("  call void inttoptr (i64 8192 to ptr)()") == ("  call void inttoptr (i64 8192 to ptr)()", 1, 0)
    @assert describe(addr(c2)) != describe(addr(c3))
    @assert normalise("@s = constant [12 x i8] c\"julia_f_1000\"")[1] == "@s = constant [12 x i8] c\"julia_f_1000\""
    @assert normalise("call void @julia_f_99()")[1] == "call void @julia_f_#1()"
    @assert normalise("br label %diffejulia_p_25490.exit\ndiffejulia_p_25490.exit:")[1] ==
            "br label %diffejulia_p_#1.exit\ndiffejulia_p_#1.exit:"
    @assert normalise("@_j_const_2 = constant [1 x ptr] [ptr inttoptr (i64 $(addr(Float64)) to ptr)]")[1] ==
            "@_j_const_2 = constant [1 x ptr] [ptr inttoptr (i64 {type Float64} to ptr)]"
end

const MODULES = Dict{String,String}()

# `reverse` as the second argument runs the rows in the opposite order: another
# session history (allocation order, numbering) for the same inputs.
const ORDER = get(ARGS, 2, "") == "reverse" ? reverse : identity

function enzyme_rows(label)
    for (x0, t) in ORDER([(x0, t) for x0 in ARGS_ for t in TARGETS])
        f = getfield(Rules, t)
        T = typeof(x0)
        status, ns_export, ns_first, raw, text, addrs, described = "ok", 0.0, 0.0, "", "", 0, 0
        try
            ns_first = @elapsed Enzyme.autodiff(Reverse, f, Active, Active(x0))
            io = IOBuffer()
            ns_export = @elapsed Enzyme.Compiler.enzyme_code_llvm(io, f, Active, Tuple{Active{T}};
                                                                   debuginfo = :none, dump_module = true)
            raw = String(take!(io))
            text, addrs, described = normalise(raw)
        catch e
            status = "refused:" * string(nameof(typeof(e)))
        end
        d = isempty(text) ? "-" : digest(text)
        isempty(text) || (MODULES[d] = text)
        println(join(["enzyme", label, t, T, status, round(Int, 1e9ns_first), round(Int, 1e9ns_export),
                      sizeof(raw), isempty(raw) ? "-" : digest(raw), addrs, described, sizeof(text), d], '\t'))
        for p in POINTS
            x = T(p)
            zy = try
                g = Zygote.gradient(f, x)[1]; g === nothing ? "nothing" : repr(g)
            catch e
                "error:" * string(nameof(typeof(e)))
            end
            en = status == "ok" ? repr(Enzyme.autodiff(Reverse, f, Active, Active(x))[1][1]) : "-"
            # The reference: f at the same input, in 512-bit arithmetic.
            ref = setprecision(BigFloat, 512) do
                b, h = big(x), big(2)^-120
                (f(b + h) - f(b - h)) / 2h
            end
            println(join(["point", label, t, T, repr(x), zy, en, string(Float64(ref))], '\t'))
        end
    end
end

println("julia\t", VERSION)
for p in (Zygote, ChainRulesCore, ChainRules, IRTools, Enzyme, Enzyme.EnzymeCore)
    println("package\t", p, "\t", pkgversion(p))
end
println("package\tLLVM\t", Base.libllvm_version)
Zygote.gradient(warm, 0.7); Zygote.gradient(warm, 0.7f0)
Enzyme.autodiff(Reverse, warm, Active, Active(0.7)); Enzyme.autodiff(Reverse, warm, Active, Active(0.7f0))
tables(VARIANT)
rows(VARIANT)
enzyme_rows(VARIANT)
for d in sort!(collect(keys(PIECES)))
    println("piece\t", d, "\t", esc_(PIECES[d]))
end
for d in sort!(collect(keys(MODULES)))
    println("module\t", d, "\t", esc_(MODULES[d]))
end
