# Ground truth from Julia itself: which generic function owns each method a
# package defines. Prints `file<TAB>line<TAB>owner`, where file is relative
# to the package's `src/` and owner is `Module.Path.name` (constructors:
# `Module.Type.new`; callable objects and unnamed: `?`).
#
# Usage: julia methods.jl PKG [SRCDIR]
#   PKG is loaded with `using` (or `include(SRCDIR/PKG.jl)` when SRCDIR is
#   given and PKG is not installed). Compared with Sophia's grouping by
#   `cargo run -p sophia-ingest --example julia_conformance`.
pkg = ARGS[1]
if length(ARGS) >= 2
    include(joinpath(ARGS[2], pkg * ".jl"))
    mod = getfield(Main, Symbol(pkg))
else
    @eval using $(Symbol(pkg))
    mod = getfield(Main, Symbol(pkg))
end
marker = "/" * pkg * "/src/"

function owner(m::Method)
    sig = Base.unwrap_unionall(m.sig)
    sig isa DataType || return "?"
    ft = Base.unwrap_unionall(sig.parameters[1])
    if ft isa DataType && ft <: Function && isdefined(ft, :instance)
        f = ft.instance
        return string(parentmodule(f)) * "." * string(nameof(f))
    elseif ft isa DataType && ft.name === Type.body.name
        T = Base.unwrap_unionall(ft.parameters[1])
        T isa DataType || return "?"
        return string(parentmodule(T)) * "." * string(nameof(T)) * ".new"
    end
    return "?"
end

Base.visit(Core.methodtable) do m
    file = String(m.file)
    i = findlast(marker, file)
    i === nothing && return
    rel = file[last(i)+1:end]
    println(rel, '\t', m.line, '\t', owner(m))
end
