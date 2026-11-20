# Macro expansions as transformation records (experiment E-4): for each
# module-level form of a package that calls a macro, Julia's expansion of it
# as source text, what that text makes when Julia evaluates it in place of
# the form, and the inputs the expansion was computed from. Compared with
# what the form itself makes (`macros.jl`) and with the definitions Sophia
# finds in the text by `sophia_ingest::expansion` (tests/expansions.rs).
#
# The package is loaded from its source form by form, as `macros.jl` does.
# A form that calls a macro is replaced by its expansion:
# - every `@eval ex` (and `@eval M ex`) outside a definition is one
#   **eval record**: `ex` with its `$` splices filled in from the values
#   they had when the loop ran; its inputs are those values (`repr`);
# - a form that contains an `@eval` (outside a definition) is evaluated with
#   each `@eval` replaced as above, and its other macro calls as written
#   (`@static if … for … @eval … end end`: eval records only);
# - any other form that is a macro call is one **macro record**: the
#   macro's output, one level (`macroexpand(…; recursive = false)`), with
#   hygiene removed (`esc` dropped, the macro's own names qualified by its
#   module); `@eval` calls in that output become eval records of their own;
# - a docstring is dropped and the documented form is expanded (or
#   evaluated as written if it calls no macro);
# - any other form with a macro call (`if … @inline f(x) = 1 end`) is
#   evaluated as written: methods it makes are record 0 (`rest`).
# Each record's text is parsed back and evaluated (`$(Expr(:meta, …))`
# and other literal `Expr`s printed by Julia are rebuilt), so the methods a
# record lists come from its printed text, not from Julia's own expression.
# If the text does not parse or throws, Julia's expression is evaluated
# instead, and the record says so.
#
# Prints:
#   julia    VERSION
#   record   file  line  k  kind  by  module  stable  status  ns_expand  ns_eval  bytes  inputs  template  text
#   method   file  line  k  owner  sig
#   binding  file  line  k  kind  name
# - k: the record's number within the form (1, 2, …; 0 for `rest`);
# - kind: `eval`, `macro`; by: the macro's binding (`Base.@deprecate`,
#   `Core.@eval`); module: the module the record was evaluated in (another
#   one than the form's for `@eval M ex`);
# - stable: `yes` if the expansion is recomputed from the recorded inputs
#   alone and prints the same (an eval record: the template filled with
#   the inputs parsed back; a macro record: a second `macroexpand`, gensym
#   numbers erased), `no` if not, `-` for what cannot be recomputed;
# - status: `ok` (the text was parsed and evaluated), `unparsed` (the text
#   does not parse; Julia's expression was evaluated), `failed` (the text
#   threw; Julia's expression was evaluated);
# - ns_expand: time to make the expression (macro expansion, or filling
#   the template); ns_eval: time to evaluate it; bytes: the text's length;
# - template: an eval record's `@eval` argument as written, with its `$`
#   (line numbers dropped; empty for a macro record);
# - inputs, template, text: escaped (`\\`, `\n`, `\t`, `\r`); inputs
#   joined by `\t` before escaping;
# - owner as in `macros.jl` (closures, compiler-made functions and keyword
#   sorters left out); sig: the method's signature without its function.
#
# Usage: julia expansions.jl PKG SRCDIR

pkg = ARGS[1]
srcdir = rstrip(abspath(ARGS[2]), '/') * "/"

name_of(m::Module) = join(string.(filter(!=(:Main), collect(fullname(m)))), ".")
relfile(file) = startswith(file, srcdir) ? file[length(srcdir)+1:end] : nothing
esc_field(s) = replace(s, '\\' => "\\\\", '\n' => "\\n", '\t' => "\\t", '\r' => "\\r")

mutable struct Record
    file::String
    line::Int
    k::Int
    kind::String
    by::String
    stable::String
    status::String
    ns_expand::UInt64
    ns_eval::UInt64
    inputs::Vector{String}
    template::String
    text::String
    w0::UInt
    w1::UInt
    mod::Module
    added::Vector{Symbol}
    parent::Int   # the k of the record whose text made this one; 0 if none
end

records = Record[]
# (file, line, module, w0, w1, before, added) of each form expanded, for `rest`.
forms = Any[]

isdef(x) = x isa Expr && (x.head in (:function, :struct, :macro, :abstract, :primitive) ||
    (x.head === :(=) && x.args[1] isa Expr && x.args[1].head in (:call, :where, :(::))))
hasmacro(x) = x isa Expr && !isdef(x) && (x.head === :macrocall || any(hasmacro, x.args))
lastarg(x) = (a = filter(y -> !(y isa LineNumberNode), x.args[2:end]); isempty(a) ? nothing : a[end])
# At the latest world: a binding made by a form evaluated inside this call
# is not visible in the world the call started in (Julia ≥ 1.12).
defined(mod) = Base.invokelatest(() -> Set(filter(s -> isdefined(mod, s), names(mod; all = true))))

function binding_of(mod, head)
    m = try
        Core.eval(mod, head)
    catch
        nothing
    end
    m isa Function ? name_of(parentmodule(m)) * "." * string(nameof(m)) : string(head)
end
iseval(mod, x) = x isa Expr && x.head === :macrocall && binding_of(mod, x.args[1]) in ("Core.@eval", "Base.@eval")

# ---------------------------------------------------------------- printing

# Hygiene removed: `esc(x)` is `x`; in a macro's own scope a name the macro's
# module binds, and the user's module does not bind to the same thing, is
# qualified by the macro's module (`depwarn` → `Base.depwarn`). Local names
# of the macro's code are kept as written (an approximation: one could
# shadow a name of the user's module).
function unhygiene(x, user::Module, m)
    if x isa Expr
        if x.head === :escape
            return unhygiene(x.args[1], user, nothing)
        elseif x.head === Symbol("hygienic-scope")
            return unhygiene(x.args[1], user, x.args[2])
        end
        return Expr(x.head, (unhygiene(a, user, m) for a in x.args)...)
    elseif x isa Symbol && m isa Module && m !== user && isdefined(m, x) && !Base.isdeprecated(m, x)
        same = Base.invokelatest(() -> isdefined(user, x) && getglobal(user, x) === getglobal(m, x))
        return same ? x : GlobalRef(m, x)
    end
    x
end

# Module-level statements: `toplevel` and `block` at the top are flattened
# (at module level a `begin … end` is its statements).
function statements(x, out = Any[])
    if x isa Expr && x.head in (:toplevel, :block)
        foreach(a -> statements(a, out), x.args)
    elseif !(x isa LineNumberNode) && x !== nothing
        push!(out, x)
    end
    out
end

# Gensyms renamed to plain names (`var"#40#x"` → `x`, or `x_2` if the
# record already uses `x`), so a record's text neither depends on how many
# gensyms the process made before it nor holds names the shared parser
# refuses. A clash is avoided within the record; Julia's evaluation of the
# text checks the result.
gensym_base(s) = (m = match(r"^#\d+#(.*)$", s); m === nothing && (m = match(r"^##(.*)#\d+$", s)); m === nothing ? nothing : m.captures[1])
function symbols!(x, out)
    x isa Symbol ? push!(out, x) : x isa QuoteNode ? symbols!(x.value, out) :
        x isa Expr ? foreach(a -> symbols!(a, out), x.args) : nothing
    out
end
function rename(x, seen, used)
    if x isa Symbol
        b = gensym_base(string(x))
        b === nothing && return x
        return get!(seen, x) do
            b = isempty(b) ? "g" : b
            name, k = Symbol(b), 1
            while name in used
                k += 1
                name = Symbol(b, "_", k)
            end
            push!(used, name)
            name
        end
    elseif x isa QuoteNode && x.value isa Symbol
        # A quoted gensym too (`:var"##x#158"`): the same name as the
        # gensym gets elsewhere in the record (review B-025, 3).
        return QuoteNode(rename(x.value, seen, used))
    elseif x isa Expr
        return Expr(x.head, (rename(a, seen, used) for a in x.args)...)
    end
    x
end

# A short definition `f(x) = body` printed as `function f(x) body end`:
# Julia prints `f(x) where T = body` as `(f(x) where T) = body`, which the
# shared parser does not take for a definition. A macro call's line
# (`#= /path/file.jl:12 =#` when printed) is dropped: it names the machine.
function signature_call(x)
    while x isa Expr && x.head in (:where, :(::)) && !isempty(x.args)
        x = x.args[1]
    end
    x isa Expr && x.head === :call
end
function source_form(x; defs = true)
    x isa Expr || return x
    args = Any[source_form(a; defs) for a in x.args]
    if x.head === :macrocall && length(args) >= 2 && args[2] isa LineNumberNode
        args[2] = nothing
    elseif defs && x.head === :(=) && length(args) == 2 && signature_call(args[1])
        body = args[2] isa Expr && args[2].head === :block ? args[2] : Expr(:block, args[2])
        return Expr(:function, args[1], body)
    end
    Expr(x.head, args...)
end
# `mod`'s bindings count as used, so a gensym a macro binds at module level
# does not take a name another record already bound.
function printed(stmts, mod = nothing)
    used = mod === nothing ? Set{Symbol}() : defined(mod)
    foreach(s -> symbols!(s, used), stmts)
    filter!(x -> gensym_base(string(x)) === nothing, used)
    seen = Dict{Symbol,Symbol}()
    join((string(rename(source_form(s), seen, used)) for s in stmts), "\n")
end

# Parsing back: `$(Expr(:meta, :inline))` and other `Expr`s Julia prints
# with literal arguments are rebuilt.
literal(x) = x isa Union{Symbol,QuoteNode,Number,String,Bool,Nothing} ||
    (x isa Expr && x.head === :call && x.args[1] === :Expr && all(literal, x.args[2:end]))
function rebuild(x)
    x isa Expr || return x
    if x.head === :$ && length(x.args) == 1 && x.args[1] isa Expr && x.args[1].head === :call &&
       x.args[1].args[1] === :Expr && all(literal, x.args[1].args[2:end])
        return Core.eval(Main, x.args[1])
    end
    Expr(x.head, (rebuild(a) for a in x.args)...)
end

# ---------------------------------------------------------------- templates

# The `$` splices of a template at depth 1 (outside nested quotes), in order.
# A `QuoteNode` is looked into: `Base.$f` is `Base.:($f)` (a field name).
function splices(x, depth = 1, out = Any[])
    x isa QuoteNode && return splices(x.value, depth, out)
    x isa Expr || return out
    if x.head === :$
        # A splat `$(xs...)` is one splice whose value is the collection.
        a = x.args[1]
        a isa Expr && a.head === :... && (a = a.args[1])
        depth == 1 ? push!(out, a) : splices(x.args[1], depth - 1, out)
    elseif x.head === :quote
        foreach(a -> splices(a, depth + 1, out), x.args)
    else
        foreach(a -> splices(a, depth, out), x.args)
    end
    out
end

# The template filled with these values, as quote interpolation fills it
# (a splat `$(xs...)` splices the elements).
function fill(x, vals, depth = 1)
    x isa QuoteNode && x.value isa Expr && return QuoteNode(fill(x.value, vals, depth))
    x isa Expr || return x
    if x.head === :$ && depth == 1
        return popfirst!(vals)
    elseif x.head === :$
        return Expr(:$, fill(x.args[1], vals, depth - 1))
    end
    d = x.head === :quote ? depth + 1 : depth
    args = Any[]
    for a in x.args
        if a isa Expr && a.head === :$ && d == 1 && a.args[1] isa Expr && a.args[1].head === :...
            append!(args, popfirst!(vals))
        else
            push!(args, fill(a, vals, d))
        end
    end
    Expr(x.head, args...)
end

# The template with its depth-1 splices replaced, in order, by `$(v)` (or
# `$(v...)` for a splat): the template filled from locals.
function respliced(x, vs, depth = 1)
    x isa QuoteNode && x.value isa Expr && return QuoteNode(respliced(x.value, vs, depth))
    x isa Expr || return x
    if x.head === :$ && depth == 1
        v = popfirst!(vs)
        a = x.args[1]
        return a isa Expr && a.head === :... ? Expr(:$, Expr(:..., v)) : Expr(:$, v)
    elseif x.head === :$
        return Expr(:$, respliced(x.args[1], vs, depth - 1))
    end
    d = x.head === :quote ? depth + 1 : depth
    Expr(x.head, (respliced(a, vs, d) for a in x.args)...)
end

reparse(mod, s) = try
    Core.eval(mod, Meta.parse(s))
catch
    nothing
end

# ---------------------------------------------------------------- records

# Evaluates one record's statements from its text, between two world
# readings; falls back to Julia's expression.
parents = Int[]  # the records whose text is being evaluated, innermost last

function run_record!(r::Record, mod, stmts)
    r.text = printed(stmts, mod)
    before = defined(mod)
    r.w0 = Base.get_world_counter()
    t = time_ns()
    parsed = try
        rebuild(Meta.parseall(r.text))
    catch
        nothing
    end
    if parsed === nothing || (parsed isa Expr && any(a -> a isa Expr && a.head in (:error, :incomplete), parsed.args))
        r.status = "unparsed"
        foreach(s -> Core.eval(mod, s), stmts)
    else
        try
            push!(parents, r.k)
            try
                for s in statements(parsed)
                    process(mod, s, r)
                end
            finally
                pop!(parents)
            end
            # A statement with `@eval` made records of its own, which hold
            # its expansion: this record's text keeps the other statements,
            # so the spliced package has each definition once (review
            # B-025, 4).
            if any(x -> x.file == r.file && x.line == r.line && x.parent == r.k, records)
                r.text = printed(filter(s -> !haseval(mod, s), statements(parsed)), mod)
            end
            r.status = "ok"
        catch e
            r.status = "failed"
            @warn "record failed" r.file r.line r.k exception = e
            foreach(s -> Core.eval(mod, s), stmts)
        end
    end
    r.ns_eval = time_ns() - t
    r.w1 = Base.get_world_counter()
    r.added = sort!(collect(setdiff(defined(mod), before)))
    r
end

current = Ref{Any}(nothing)  # (file, line) of the form being loaded
nextk(file, line) = count(r -> r.file == file && r.line == line, records) + 1

function new_record(kind, by, mod)
    file, line = current[]
    r = Record(file, line, nextk(file, line), kind, by, "-", "", 0, 0, String[], "", "", 0, 0, mod, Symbol[],
        isempty(parents) ? 0 : parents[end])
    push!(records, r)
    r
end

# Called by the rewritten `@eval`: one eval record.
function eval_record(mod::Module, template, ex, vals, ns)
    r = new_record("eval", "Core.@eval", mod)
    r.ns_expand = ns
    r.inputs = [repr(v) for v in vals]
    r.template = string(source_form(Base.remove_linenums!(deepcopy(template)); defs = false))
    back = [reparse(mod, s) for s in r.inputs]
    filled = try
        fill(template, Any[back...])
    catch
        nothing
    end
    r.stable = filled !== nothing && string(Base.remove_linenums!(deepcopy(filled))) ==
        string(Base.remove_linenums!(deepcopy(ex))) ? "yes" : "no"
    run_record!(r, mod, statements(Base.remove_linenums!(ex)))
    nothing
end

# `@eval ex` → `eval_record(mod, template, <ex filled>, [splices…], ns)`,
# outside definitions.
function rewrite(mod, x)
    x isa Expr || return x
    isdef(x) && return x
    if iseval(mod, x)
        a = filter(y -> !(y isa LineNumberNode), x.args[2:end])
        target, ex = length(a) == 2 ? (a[1], a[2]) : (Expr(:macrocall, Symbol("@__MODULE__"), LineNumberNode(0)), a[1])
        # Each splice is evaluated once, into a local, and the template is
        # filled from the locals: a splice with a side effect runs once, as
        # under Julia's own `@eval` (review B-025, 2).
        t = gensym("t")
        e = gensym("e")
        vs = [gensym("v") for _ in splices(ex)]
        return Expr(:let, Expr(:block),
            Expr(:block,
                Expr(:(=), t, Expr(:call, time_ns)),
                (Expr(:(=), v, x) for (v, x) in zip(vs, splices(ex)))...,
                Expr(:(=), e, Expr(:quote, respliced(ex, copy(vs)))),
                Expr(:call, eval_record, target, QuoteNode(ex), e, Expr(:vect, vs...),
                    Expr(:call, -, Expr(:call, time_ns), t))))
    end
    Expr(x.head, (rewrite(mod, a) for a in x.args)...)
end

haseval(mod, x) = x isa Expr && !isdef(x) && (iseval(mod, x) || any(a -> haseval(mod, a), x.args))

# One statement of a form, or of a record's text.
function process(mod, x, parent = nothing)
    if x isa Expr && x.head === :macrocall && binding_of(mod, x.args[1]) == "Core.@doc" && lastarg(x) !== nothing &&
       !(lastarg(x) isa Expr && lastarg(x).head === :module)
        inner = lastarg(x)
        return hasmacro(inner) ? process(mod, inner, parent) : Core.eval(mod, x)
    elseif haseval(mod, x)
        return Core.eval(mod, rewrite(mod, x))
    elseif parent === nothing && x isa Expr && x.head === :macrocall
        r = new_record("macro", binding_of(mod, x.args[1]), mod)
        t = time_ns()
        ex = macroexpand(mod, x; recursive = false)
        r.ns_expand = time_ns() - t
        again = macroexpand(mod, x; recursive = false)
        # Gensyms are renamed by `printed`, so two expansions that differ only
        # in gensym numbers print the same; any other difference is kept
        # (review B-025, 3: erasing digits hid quoted gensyms).
        clean(e) = printed(statements(Base.remove_linenums!(unhygiene(e, mod, nothing))))
        r.stable = clean(ex) == clean(again) ? "yes" : "no"
        run_record!(r, mod, statements(Base.remove_linenums!(unhygiene(ex, mod, nothing))))
        return nothing
    end
    Core.eval(mod, x)
end

isdoc(mod, x) = x isa Expr && x.head === :macrocall && binding_of(mod, x.args[1]) == "Core.@doc" && lastarg(x) !== nothing

function form(file, line, mod, x)
    if !hasmacro(x) || (isdoc(mod, x) && !hasmacro(lastarg(x)))
        Core.eval(mod, x)
        return nothing
    end
    current[] = (file, line)
    w0 = Base.get_world_counter()
    before = defined(mod)
    process(mod, x)
    push!(forms, (file, line, mod, w0, Base.get_world_counter(), setdiff(defined(mod), before)))
    nothing
end

# ---------------------------------------------------------------- loading (as macros.jl)

here = Expr(:macrocall, GlobalRef(Base, Symbol("@__MODULE__")), LineNumberNode(0))
isinclude(x) = x isa Expr && x.head === :call && x.args[1] === :include && length(x.args) == 2 && x.args[2] isa String
isdocmodule(x) = x isa Expr && x.head === :macrocall && x.args[1] == GlobalRef(Core, Symbol("@doc")) &&
    lastarg(x) isa Expr && lastarg(x).head === :module

function wrap(x, lnn, file)
    if isdocmodule(x)
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
    Expr(:toplevel, lnn, Expr(:call, form, file, lnn.line, here, QuoteNode(x)))
end

function load(mod, path)
    ast = Meta.parseall(read(path, String); filename = path)
    lnn = LineNumberNode(1, Symbol(path))
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

# ---------------------------------------------------------------- output

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
        return "(::Type)"
    end
    startswith(string(nameof(T)), "#") && return nothing
    "(::" * name_of(parentmodule(T)) * "." * string(nameof(T)) * ")"
end

function signature(m)
    sig = Base.unwrap_unionall(m.sig)
    "(" * join(string.(sig.parameters[2:end]), ", ") * ")"
end

kind(v, s) = v isa Module ? "module" : v isa Type ? "type" :
    v isa Function ? (startswith(string(s), "@") ? "macro" : "function") : "value"

# Each method to the innermost window it entered the method table in: a
# record's, else its form's (`rest`).
# Record 0 is keyed by the form's index, not its line: a file included into
# two modules has two forms at one line (review B-025, 5).
# Forms first, then records in the order they were made, and ties go to the
# later window: a record whose text made another record (`@mkeval`) may
# have the same window as it, and the inner one made the method.
windows = Any[(f[4], f[5], (:form, i)) for (i, f) in enumerate(forms)]
append!(windows, [(r.w0, r.w1, (:record, i)) for (i, r) in enumerate(records)])
made = Dict{Any,Vector{Method}}()
Base.visit(Core.methodtable) do m
    w = m.primary_world
    best = nothing
    for x in windows
        if x[1] < w <= x[2] && (best === nothing || x[2] - x[1] <= best[2] - best[1])
            best = x
        end
    end
    best === nothing && return
    push!(get!(made, best[3], Method[]), m)
end

# A name a record's child bound is the child's, not also its parent's.
for r in records, c in records
    c.file == r.file && c.line == r.line && c.parent == r.k && c !== r && setdiff!(r.added, c.added)
end

println("julia\t", VERSION)
rows_of(ms) = sort!([(o, signature(m)) for m in ms for o in (owner(m),) if o !== nothing])
emit(file, line, k, rows, added, mod) = begin
    rf = relfile(file)
    for (o, s) in rows
        println(join(("method", rf, line, k, o, s), '\t'))
    end
    for s in added
        startswith(string(s), "#") && continue
        println(join(("binding", rf, line, k, kind(getglobal(mod, s), s), name_of(mod) * "." * string(s)), '\t'))
    end
end
recorded = Set{Tuple{Module,Symbol}}()
for (i, r) in enumerate(records)
    rf = relfile(r.file)
    rf === nothing && continue
    println(join(("record", rf, r.line, r.k, r.kind, r.by, name_of(r.mod), r.stable, r.status, r.ns_expand, r.ns_eval,
        ncodeunits(r.text), esc_field(join(r.inputs, '\t')), esc_field(r.template), esc_field(r.text)), '\t'))
    emit(r.file, r.line, r.k, rows_of(get(made, (:record, i), Method[])), r.added, r.mod)
    union!(recorded, ((r.mod, a) for a in r.added))
end
for (i, f) in enumerate(forms)
    relfile(f[1]) === nothing && continue
    rest = rows_of(get(made, (:form, i), Method[]))
    added = sort!([a for a in f[6] if !((f[3], a) in recorded) && !startswith(string(a), "#")])
    (isempty(rest) && isempty(added)) && continue
    println(join(("record", relfile(f[1]), f[2], 0, "rest", "-", name_of(f[3]), "-", "-", 0, 0, 0, "", "", ""), '\t'))
    emit(f[1], f[2], 0, rest, added, f[3])
end
