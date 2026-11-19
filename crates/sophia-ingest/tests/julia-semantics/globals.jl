# Ground truth from Julia itself for part 7 of the conformance suite
# (mutable globals): which global bindings of the package each method reads
# or writes, and whether each is a constant or a mutable global. Compared
# with Sophia's references by `cargo run -p sophia-ingest --example
# julia_globals` (and `tests/globals.rs` on the fixture). See globals.md.
#
# Usage: julia globals.jl PKG [SRCDIR]   (as calls.jl)
#
# Prints two kinds of line, tab-separated:
#   binding  Module.name  kind  type  object
#     every binding the package's modules own that holds data: a constant
#     whose value is not a function, type or module (those are parts 1 to 3's),
#     or any mutable global, whatever it holds (its value is state): kind is const | global (mutable) | declared (`global x` with no
#     value); type is the declared type of a mutable global
#     (`Any` when none), `-` for a constant; object is mutable | immutable
#     for a constant's value (`ismutable`, strings excepted: a `Dict` or
#     `Ref` held by a constant binding changes), `-` otherwise;
#   use  file  line  name  Module.name  access  kind
#     one per (method of the package, global binding of the package it
#     touches that is not a function, type or module): access is read
#     (a `GlobalRef`, or `getproperty(M, :x)` on a module) or write
#     (`Core.setglobal!(M, :x, v)`, which `global x = v` lowers to, or
#     `setproperty!(M, :x, v)`) or local (an argument or local variable of
#     the method, its `slotnames`, spelled like a data binding of the
#     method's module: the name is the local there); kind as above.
#
# Binding kinds come from Julia ≥ 1.12's binding partitions
# (`Base.binding_kind`, see `kind_of`), a mutable global's type from
# `Core.get_binding_type`. A method's body is its lowered code, so a
# closure is its own method at its own line, and a generator's body (filed
# at `none:0`) is read as part of the method that creates it.
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

# The package's modules: the root and every submodule bound in it.
modules = Module[]
function collect_modules(m)
    push!(modules, m)
    for s in names(m; all = true)
        isdefined(m, s) || continue
        v = getglobal(m, s)
        v isa Module && v !== m && parentmodule(v) === m && collect_modules(v)
    end
end
collect_modules(mod)
ours = Set(modules)

# A binding that holds data: not a function, type or module, and not one
# the compiler made (`#f#1`) or every module has (`eval`, `include`). The
# kind is the binding's partition in the current world (Julia ≥ 1.12,
# `Base.binding_kind`, internal): a constant (also one declared and not yet
# assigned, or backdated), a mutable global (also one only a function's
# `global x = v` declares, not yet assigned), a `global x` declaration with
# no value. Imports, `using` and guards (a name read but never defined)
# are not M's own bindings and are left out.
function kind_of(M::Module, s::Symbol)
    startswith(string(s), "#") && return nothing
    s in (:eval, :include) && return nothing
    k = Base.binding_kind(M, s)
    if k in (Base.PARTITION_KIND_CONST, Base.PARTITION_KIND_BACKDATED_CONST, Base.PARTITION_KIND_UNDEF_CONST)
        isdefined(M, s) || return "const"
        v = getglobal(M, s)
        (v isa Function || v isa Type || v isa Module) && return nothing
        return "const"
    elseif k == Base.PARTITION_KIND_GLOBAL
        return "global"
    elseif k == Base.PARTITION_KIND_DECLARED
        return "declared"
    end
    nothing
end

# A `String` is a mutable type to `ismutable`, but no operation changes one
# (other `AbstractString` types may be mutable: review B-078, 8).
mutable_object(v) = !(v isa String) && ismutable(v)

for M in modules, s in names(M; all = true, imported = false)
    k = kind_of(M, s)
    k === nothing && continue
    T = k == "const" ? "-" : string(Core.get_binding_type(M, s))
    o = k == "const" && isdefined(M, s) ? (mutable_object(getglobal(M, s)) ? "mutable" : "immutable") : "-"
    println(join(("binding", name_of(M) * "." * string(s), k, T, o), '\t'))
end

lowered(m) = try
    ci = Base.uncompressed_ast(m)
    ci isa Core.CodeInfo ? ci : nothing
catch
    nothing
end

# The module bindings a lowered body reads and writes: (module, symbol, access).
function touches(ci)
    code = ci.code
    value(x) = x isa GlobalRef ? (isdefined(x.mod, x.name) ? getglobal(x.mod, x.name) : nothing) :
               x isa Core.SSAValue ? value(code[x.id]) : x isa QuoteNode ? x.value : x
    out = Tuple{Module,Symbol,String}[]
    walk(e) = if e isa GlobalRef
        push!(out, (e.mod, e.name, "read"))
    elseif e isa Expr
        if e.head === :call && length(e.args) >= 3
            f = value(e.args[1])
            m, s = value(e.args[2]), value(e.args[3])
            if m isa Module && s isa Symbol
                if (f === Core.setglobal! || f === Base.setproperty!) && length(e.args) >= 4
                    push!(out, (m, s, "write"))
                elseif f === Base.getproperty || f === Core.getglobal
                    push!(out, (m, s, "read"))
                end
            end
        end
        foreach(walk, e.args)
    end
    foreach(walk, code)
    out
end

closures = Dict{Any,Vector{Method}}()
Base.visit(Core.methodtable) do m
    sig = Base.unwrap_unionall(m.sig)
    sig isa DataType && !isempty(sig.parameters) || return
    T = Base.unwrap_unionall(sig.parameters[1])
    T isa DataType && startswith(string(nameof(T)), "#") || return
    push!(get!(closures, T.name.wrapper, Method[]), m)
end

# A method's touches, plus those of the closures it creates that Julia files
# under no source line (a generator's body).
function all_touches(m, seen = Set{Method}([m]))
    ci = lowered(m)
    ci === nothing && return Tuple{Module,Symbol,String}[]
    out = touches(ci)
    for (M, s, _) in copy(out)
        startswith(string(s), "#") && isdefined(M, s) || continue
        for c in get(closures, getglobal(M, s), Method[])
            c in seen && continue
            relfile(String(c.file)) === nothing || continue
            push!(seen, c)
            append!(out, all_touches(c, seen))
        end
    end
    out
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
    for (M, s, access) in unique(all_touches(m))
        M in ours || continue
        k = kind_of(M, s)
        k === nothing && continue
        println(join(("use", file, m.line, string(s), name_of(M) * "." * string(s), access, k), '\t'))
    end
    # Arguments and locals of the method named like a data binding of its
    # module: inside the method the name is the local, not the binding.
    ci = lowered(m)
    ci === nothing && return
    m.module in ours || return
    for s in unique(ci.slotnames)
        k = kind_of(m.module, s)
        k === nothing && continue
        println(join(("use", file, m.line, string(s), name_of(m.module) * "." * string(s), "local", k), '\t'))
    end
end
