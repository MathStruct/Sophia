# Ground truth from Julia itself: which global bindings each method of a
# package uses in its body, and what Julia's binding is (a function and its
# methods, a type, a module). Part 3 of the conformance suite (dispatch):
# compared with Sophia's references by
# `cargo run -p sophia-ingest --example julia_dispatch`.
#
# Prints one line per (method, binding used):
#   file  line  name  binding  kind  owner  position  nmethods  here
# - file, line: the method's (file relative to the package's `src/`);
# - name: the name as the body uses it; binding: `Module.name` where Julia
#   looks it up (`A.h` for a qualified `A.h(x)`);
# - kind: function | type | module | other | undefined;
# - owner: a function's `parentmodule.nameof` (the generic function), a
#   type's `parentmodule.nameof`, otherwise the binding;
# - position: call (the callee of a call) or value;
# - nmethods: how many methods the function (or the type's constructors)
#   has, in all of Julia; here: those of them defined in this package,
#   `file:line` joined by `,`, one entry per method (`-` for none).
#
# The body is the method's lowered code (`Base.uncompressed_ast`), so a
# closure's body is its own method at its own line, and macros are already
# expanded. A closure Julia files at no line (a generator's body, `none:0`)
# is read as part of the method that creates it.
#
# Usage: julia calls.jl PKG [SRCDIR [CHANGE]]   (as methods.jl)
#
# CHANGE (part 4, world change) is a file evaluated after the package is
# loaded: into the package's module when it lies under SRCDIR (its methods
# are then the package's), otherwise into `Main` (another package; its
# methods are not). The table is printed in the world after the change.
pkg = ARGS[1]
if length(ARGS) >= 2
    include(joinpath(abspath(ARGS[2]), pkg * ".jl"))  # relative to the caller, not this script
else
    @eval using $(Symbol(pkg))
end
mod = getfield(Main, Symbol(pkg))
# A file of the package: under SRCDIR when given, else under `/PKG/src/`.
marker = length(ARGS) >= 2 ? rstrip(abspath(ARGS[2]), '/') * "/" : "/" * pkg * "/src/"
if length(ARGS) >= 3
    change = abspath(ARGS[3])
    Base.include(startswith(change, marker) ? mod : Main, change)
end

function relfile(file)
    i = findlast(marker, file)
    i === nothing ? nothing : file[last(i)+1:end]
end

# `Main.Disp.f` (loaded by `include`) prints as `Disp.f`.
name_of(m::Module) = join(string.(filter(!=(:Main), collect(fullname(m)))), ".")

function describe(v, b::String)
    if v isa Module
        return "module", name_of(v), Method[]
    elseif v isa Type
        T = Base.unwrap_unionall(v)
        owner = T isa DataType ? name_of(parentmodule(T)) * "." * string(nameof(T)) : b
        return "type", owner, collect(methods(v))
    elseif v isa Function
        return "function", name_of(parentmodule(v)) * "." * string(nameof(v)), collect(methods(v))
    end
    return "other", b, Method[]
end

function here(ms)
    out = String[]
    for m in ms
        r = relfile(String(m.file))
        r === nothing || push!(out, r * ":" * string(m.line))
    end
    isempty(out) ? "-" : join(sort!(out), ",")  # one entry per method: default arguments repeat a line
end

# The bindings a lowered body uses: GlobalRefs, and `getproperty(M, :x)` on
# a module (a qualified name). Returns (name, module, symbol, position).
function uses(ci)
    code = ci.code
    called = Set{Int}()
    for st in code
        st isa Expr && st.head === :call && st.args[1] isa Core.SSAValue && push!(called, st.args[1].id)
    end
    value(x) = x isa GlobalRef ? (isdefined(x.mod, x.name) ? getglobal(x.mod, x.name) : nothing) :
               x isa Core.SSAValue ? value(code[x.id]) : x isa QuoteNode ? x.value : x
    out = Tuple{String,Module,Symbol,String}[]
    for (i, st) in enumerate(code)
        pos = i in called ? "call" : "value"
        if st isa GlobalRef
            push!(out, (string(st.name), st.mod, st.name, pos))
        elseif st isa Expr && st.head === :call && length(st.args) == 3 && value(st.args[1]) === Base.getproperty
            m, s = value(st.args[2]), value(st.args[3])
            if m isa Module && s isa Symbol
                push!(out, (string(s), m, s, pos))
            end
        end
        # A GlobalRef anywhere inside a statement: an argument, the right
        # side of `x = f` (a global assigned to a local), a `return`.
        if st isa Expr
            inner(e, callee) = if e isa GlobalRef
                push!(out, (string(e.name), e.mod, e.name, callee ? "call" : "value"))
            elseif e isa Expr
                for (k, a) in enumerate(e.args)
                    inner(a, e.head === :call && k == 1)
                end
            end
            inner(st, false)
        end
    end
    out
end

lowered(m) = try
    ci = Base.uncompressed_ast(m)
    ci isa Core.CodeInfo ? ci : nothing
catch
    nothing
end

# Methods of closure types (`#f##0…`), by type: `methods(T)` would list
# T's constructors, not the calls of its instances.
closures = Dict{Any,Vector{Method}}()
Base.visit(Core.methodtable) do m
    sig = Base.unwrap_unionall(m.sig)
    sig isa DataType && !isempty(sig.parameters) || return
    T = Base.unwrap_unionall(sig.parameters[1])
    T isa DataType && startswith(string(nameof(T)), "#") || return
    push!(get!(closures, T.name.wrapper, Method[]), m)
end

# A method's uses, plus those of the closures it creates that Julia files
# under no source line (a generator's body is a method at `none:0`): they
# are read as part of the method that creates them.
function all_uses(m, seen = Set{Method}([m]))
    ci = lowered(m)
    ci === nothing && return Tuple{String,Module,Symbol,String}[]
    out = uses(ci)
    for (name, M, s, _) in copy(out)
        startswith(name, "#") && isdefined(M, s) || continue
        T = getglobal(M, s)
        for c in get(closures, T, Method[])
            c in seen && continue
            relfile(String(c.file)) === nothing || continue
            push!(seen, c)
            append!(out, all_uses(c, seen))
        end
    end
    out
end

# A method another one replaced (same signature, defined later) stays in
# the table but is no longer dispatched to in the current world: skip it.
current(m) = try
    Base.which(m.sig) === m
catch
    true  # an ambiguity: keep it
end

Base.visit(Core.methodtable) do m
    file = relfile(String(m.file))
    file === nothing && return
    current(m) || return
    for (name, M, s, pos) in unique(all_uses(m))
        b = name_of(M) * "." * name
        kind, owner, ms = isdefined(M, s) ? describe(getglobal(M, s), b) : ("undefined", b, Method[])
        println(join((file, m.line, name, b, kind, owner, pos, length(ms), here(ms)), '\t'))
    end
end
