# Ground truth from Julia itself for part 6 of the conformance suite
# (closures and captured variables): for every closure a package's methods
# create, what Julia's lowering made of each name in it. Compared with
# Sophia's encoding by `sophia_ingest::conformance::closure_ledger`.
#
# Prints one line per closure method:
#   file  line  closure  placed  args  captured  locals  globals
# - file, line: the closure method's (file relative to the package's
#   `src/`); for a closure Julia files at no source line (a generator's or
#   comprehension's body, `none:0`), the method that creates it, and
#   placed = `creator` (else `own`);
# - closure: the closure type's name (`#adder##0#adder##1`);
# - args: the closure's argument names (`#self#` and unnamed ones left out);
# - captured: the closure type's fields, `fieldnames(typeof(closure))`, each
#   `name` or `name:box` when Julia boxes it (`Core.Box`: the variable is
#   assigned after it is captured); a static parameter of the enclosing
#   method that the closure uses is a type parameter of the closure type,
#   not a field: `name:sparam`;
# - locals: the closure's other local variables (slot names);
# - globals: each global binding its lowered body uses, `name=Module.name`
#   (compiler-made `#…` names left out);
# lists are joined by `,`, `-` when empty.
#
# A closure type is a type named `#…` whose name is bound to the type
# itself and which is not a named function's type (`typeof(solo)` is bound
# as `#solo` too, and `solo` is bound to its instance). A closure with
# keyword arguments has several closure types at its line (the positional
# one, its keyword body); fields Julia makes for itself (`#20#21`, the
# keyword body a closure holds) are left out.
#
# Usage: julia closures.jl PKG [SRCDIR]   (as calls.jl)
pkg = ARGS[1]
if length(ARGS) >= 2
    include(joinpath(abspath(ARGS[2]), pkg * ".jl"))
else
    @eval using $(Symbol(pkg))
end
mod = getfield(Main, Symbol(pkg))
marker = length(ARGS) >= 2 ? rstrip(abspath(ARGS[2]), '/') * "/" : "/" * pkg * "/src/"

function relfile(file)
    i = findlast(marker, file)
    i === nothing ? nothing : file[last(i)+1:end]
end
name_of(m::Module) = join(string.(filter(!=(:Main), collect(fullname(m)))), ".")

lowered(m) = try
    ci = Base.uncompressed_ast(m)
    ci isa Core.CodeInfo ? ci : nothing
catch
    nothing
end

function closure_type(m::Method)
    sig = Base.unwrap_unionall(m.sig)
    sig isa DataType && !isempty(sig.parameters) || return nothing
    T = Base.unwrap_unionall(sig.parameters[1])
    T isa DataType || return nothing
    n, M = T.name.name, T.name.module
    startswith(string(n), "#") && isdefined(M, n) && getglobal(M, n) isa Type || return nothing
    # A named function's type (`typeof(solo)`, bound as `#solo` too): its
    # instance is bound under the function's name.
    if isdefined(T, :instance)
        f = T.instance
        isdefined(parentmodule(f), nameof(f)) && getglobal(parentmodule(f), nameof(f)) === f && return nothing
    end
    T.name.wrapper
end

# Closure methods by closure type.
closures = Dict{Any,Vector{Method}}()
Base.visit(Core.methodtable) do m
    W = closure_type(m)
    W === nothing || push!(get!(closures, W, Method[]), m)
end

joined(xs) = isempty(xs) ? "-" : join(xs, ",")
useful(s::Symbol) = (t = string(s); !isempty(t) && !startswith(t, "#"))

function globals(ci)
    out = String[]
    walk(e) = if e isa GlobalRef
        useful(e.name) && push!(out, string(e.name) * "=" * name_of(e.mod) * "." * string(e.name))
    elseif e isa Expr
        foreach(walk, e.args)
    end
    foreach(walk, ci.code)
    unique!(sort!(out))
end

function row(file, line, W, placed, m)
    ci = lowered(m)
    ci === nothing && return
    names = ci.slotnames
    args = [string(s) for s in names[2:m.nargs] if useful(s)]
    locals = unique!(sort!([string(s) for s in names[m.nargs+1:end] if useful(s) && !(string(s) in args)]))
    U = Base.unwrap_unionall(W)
    fields = [string(f) * (t === Core.Box ? ":box" : "") for (f, t) in zip(fieldnames(U), U.types) if useful(f)]
    vars = string.(U.parameters)
    fieldtypes_named = Set(string(t) for t in U.types if t isa TypeVar)
    sparams = [v * ":sparam" for v in vars if !(v in fieldtypes_named) && !(v in string.(fieldnames(U)))]
    println(join((file, line, string(nameof(U)), placed, joined(args), joined(vcat(fields, sparams)), joined(locals), joined(globals(ci))), '\t'))
end

# Closure types a method's lowered code names.
function made(m)
    ci = lowered(m)
    ci === nothing && return Any[]
    out = Any[]
    walk(e) = if e isa GlobalRef
        isdefined(e.mod, e.name) && (v = getglobal(e.mod, e.name); haskey(closures, v) && push!(out, v))
    elseif e isa Expr
        foreach(walk, e.args)
    end
    foreach(walk, ci.code)
    unique!(out)
end

current(m) = try
    Base.which(m.sig) === m
catch
    true
end

Base.visit(Core.methodtable) do m
    file = relfile(String(m.file))
    file === nothing && return
    current(m) || return
    if closure_type(m) !== nothing
        row(file, m.line, closure_type(m), "own", m)
    end
    # Closures Julia files at no line, under the method that creates them
    # (recursively: a generator inside a generator).
    pending = made(m)
    seen = Set{Any}()
    while !isempty(pending)
        W = pop!(pending)
        W in seen && continue
        push!(seen, W)
        for c in closures[W]
            relfile(String(c.file)) === nothing || continue
            row(file, m.line, W, "creator", c)
            append!(pending, made(c))
        end
    end
end
