# Ground truth from Julia itself: what each module-level form of a package
# that calls a macro defines, and whether the macro changes the definition it
# wraps. Part 5 of the conformance suite (macros): compared with Sophia's
# definitions by `sophia_ingest::conformance::macro_ledger` (tests/macros.rs).
#
# The package is loaded from its source by this script, form by form, as
# `include` would: each module-level form is evaluated on its own, between
# two world-counter readings, and a method belongs to the form during which
# it entered the method table (`primary_world`). So a method a macro makes
# belongs to the macro call even when Julia files it at a line inside the
# macro's definition, or in another package (`@enum`'s, in `Enums.jl`).
# `include("lit.jl")` at module level is followed; a computed include is
# evaluated as written (its forms are not told apart).
#
# Prints, for each form that is a macro call or contains one outside a
# definition (`for … @eval … end`, `@static if`):
#   form     file  line  module  macro  changes
#   method   file  line  owner   at
#   binding  file  line  kind    name
# - file, line: the form's first line (file relative to SRCDIR);
# - macro: the macro's binding (`Base.@inline`, `Core.@doc` for a
#   docstring), or the head of a block that contains one (`for`, `if`);
# - changes, for a macro call whose last argument is a definition: `changed`
#   if the definition's lowered code differs with and without the macro,
#   `same` if not, `doc` for a docstring; `-` otherwise;
# - method: one per method the form put in the method table; owner is the
#   generic function (`parentmodule.nameof`; `(::M.T)` for a callable
#   object's method), at its `file:line` (relative,
#   or `elsewhere:line` outside SRCDIR). Compiler-made functions and
#   closures (`#…`) and keyword sorters (`Core.kwcall`) are left out;
# - binding: one per name the form bound in its module, kind `function`,
#   `macro`, `type`, `module` or `value`.
#
# Usage: julia macros.jl PKG SRCDIR

pkg = ARGS[1]
srcdir = rstrip(abspath(ARGS[2]), '/') * "/"

name_of(m::Module) = join(string.(filter(!=(:Main), collect(fullname(m)))), ".")
relfile(file) = startswith(file, srcdir) ? file[length(srcdir)+1:end] : nothing

mutable struct Form
    file::String
    line::Int
    mod::Module
    label::String     # "-" for a form without a macro call
    changes::String
    w0::UInt
    w1::UInt
    before::Set{Symbol}
    added::Vector{Symbol}
end
forms = Form[]

isdef(x) = x isa Expr && (x.head in (:function, :struct, :macro, :abstract, :primitive) ||
    (x.head === :(=) && x.args[1] isa Expr && x.args[1].head in (:call, :where, :(::))))

# A macro call anywhere in a form, not inside a definition's body.
hasmacro(x) = x isa Expr && !isdef(x) && (x.head === :macrocall || any(hasmacro, x.args))

lastarg(x) = (a = filter(y -> !(y isa LineNumberNode), x.args[2:end]); isempty(a) ? nothing : a[end])

function label(mod, x)
    x isa Expr || return "-"
    if x.head === :macrocall
        m = try
            Core.eval(mod, x.args[1])
        catch
            nothing
        end
        m isa Function && return name_of(parentmodule(m)) * "." * string(nameof(m))
        return string(x.args[1])
    end
    hasmacro(x) ? string(x.head) : "-"
end

hasinterp(x) = x isa Expr && (x.head === Symbol(raw"$") || any(hasinterp, x.args))

normal(s) = replace(s, r"#\d+" => "#", r"[^\s()]*:\d+" => "")
lowered(mod, x) = try
    normal(string(Meta.lower(mod, Base.remove_linenums!(deepcopy(x)))))
catch e
    "error: " * sprint(showerror, e)
end

function changes(mod, x, lab)
    x isa Expr && x.head === :macrocall || return "-"
    lab == "Core.@doc" && return "doc"
    inner = lastarg(x)
    isdef(inner) || return "-"
    # `@eval ex` without `$` evaluates `ex` itself; with `$` it is a
    # template, and there is no bare definition to compare with.
    endswith(lab, ".@eval") && return hasinterp(inner) ? "-" : "same"
    lowered(mod, x) == lowered(mod, inner) ? "same" : "changed"
end

# The names bound in a module (an `export` lists a name before it is bound).
defined(mod) = Set(filter(s -> isdefined(mod, s), names(mod; all = true)))

function mark_begin(file, line, mod, x)
    lab = label(mod, x)
    ch = lab == "-" ? "-" : changes(mod, x, lab)
    push!(forms, Form(file, line, mod, lab, ch, Base.get_world_counter(), 0, defined(mod), Symbol[]))
    nothing
end

function mark_end()
    f = forms[end]
    f.w1 = Base.get_world_counter()
    f.added = sort!(collect(setdiff(defined(f.mod), f.before)))
    empty!(f.before)
    nothing
end

here = Expr(:macrocall, GlobalRef(Base, Symbol("@__MODULE__")), LineNumberNode(0))

isinclude(x) = x isa Expr && x.head === :call && x.args[1] === :include && length(x.args) == 2 && x.args[2] isa String

isdocmodule(x) = x isa Expr && x.head === :macrocall && x.args[1] == GlobalRef(Core, Symbol("@doc")) &&
    lastarg(x) isa Expr && lastarg(x).head === :module

function wrap(x, lnn, file)
    if isdocmodule(x)  # a module with a docstring: the module, its docstring dropped
        return wrap(lastarg(x), lnn, file)
    elseif x isa Expr && x.head === :module
        body = Expr(:block)
        at = lnn
        for y in x.args[3].args
            if y isa LineNumberNode
                at = y
                push!(body.args, y)
            else
                push!(body.args, wrap(y, at, file))
            end
        end
        return Expr(:module, x.args[1], x.args[2], body)
    elseif isinclude(x)
        return Expr(:call, load, here, joinpath(dirname(file), x.args[2]))
    end
    Expr(:toplevel, lnn, Expr(:call, mark_begin, file, lnn.line, here, QuoteNode(x)), x, Expr(:call, mark_end))
end

function load(mod, path)
    ast = Meta.parseall(read(path, String); filename = path)
    lnn = LineNumberNode(1, Symbol(path))
    # As `Base.include`: a computed `include` inside a form resolves
    # against the file being loaded.
    tls = task_local_storage()
    old = get(tls, :SOURCE_PATH, nothing)
    tls[:SOURCE_PATH] = path
    for x in ast.args
        if x isa LineNumberNode
            lnn = x
            continue
        end
        Core.eval(mod, wrap(x, lnn, path))
    end
    old === nothing ? delete!(tls, :SOURCE_PATH) : (tls[:SOURCE_PATH] = old)
    nothing
end

load(Main, joinpath(srcdir, pkg * ".jl"))

# Each method to the form during which it entered the method table.
starts = [f.w0 for f in forms]
methods_of = Dict{Int,Vector{Method}}()
Base.visit(Core.methodtable) do m
    w = m.primary_world
    i = searchsortedlast(starts, w - 1)  # w0 < w
    i >= 1 && w <= forms[i].w1 || return
    push!(get!(methods_of, i, Method[]), m)
end

function owner(m)
    sig = Base.unwrap_unionall(m.sig)
    T = Base.unwrap_unionall(sig.parameters[1])
    T isa DataType || return nothing
    if T.name === typeof(Core.kwcall).name
        return nothing
    elseif T <: Function && isdefined(T, :instance)
        f = T.instance
        name = string(nameof(f))
        startswith(name, "#") && return nothing
        return name_of(parentmodule(f)) * "." * name
    elseif T <: Type
        P = Base.unwrap_unionall(T.parameters[1])
        P isa DataType && return name_of(parentmodule(P)) * "." * string(nameof(P))
        # `(::Type{T})(…) where T`: a constructor of a type parameter.
        return "(::Type)"
    end
    # A callable object: `(p::MulAddMul)(x) = …`; a closure's type is
    # compiler-made.
    startswith(string(nameof(T)), "#") && return nothing
    "(::" * name_of(parentmodule(T)) * "." * string(nameof(T)) * ")"
end

function kind(v, s)
    v isa Module ? "module" : v isa Type ? "type" : v isa Function ? (startswith(string(s), "@") ? "macro" : "function") : "value"
end

for (i, f) in enumerate(forms)
    f.label == "-" && continue
    file = relfile(f.file)
    file === nothing && continue
    println(join(("form", file, f.line, name_of(f.mod), f.label, f.changes), '\t'))
    for m in get(methods_of, i, Method[])
        o = owner(m)
        o === nothing && continue
        r = relfile(String(m.file))
        println(join(("method", file, f.line, o, (r === nothing ? "elsewhere" : r) * ":" * string(m.line)), '\t'))
    end
    for s in f.added
        startswith(string(s), "#") && continue
        println(join(("binding", file, f.line, kind(getglobal(f.mod, s), s), name_of(f.mod) * "." * string(s)), '\t'))
    end
end
