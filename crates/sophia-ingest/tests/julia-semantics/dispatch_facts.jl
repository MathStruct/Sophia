# Dispatch facts recorded from Julia, and what one added method does to them
# (E-3, B-024; see dispatch_facts.md and [[Dispatch Invalidation]]).
#
#   julia dispatch_facts.jl <setup.jl> <change.jl> [max_instances]
#
# `setup.jl` loads the code, compiles a workload and defines `TARGET` (a module).
# The script records a **dispatch fact** for every call in the inferred code of
# every compiled method instance of `TARGET` (and of the instances those calls
# reach, up to `max_instances`): the caller, the call's argument types and the
# methods the call can reach. Then it evaluates `change.jl` into `Main` with
# Julia's invalidation log on, and prints, as tab-separated rows:
#
#   type     <name> <abstract 0|1> <supertypes, space-separated>
#   method   <id> <encoded signature>
#   new      <id> <encoded signature>          (methods the change added)
#   instance <instance> <invalidated 0|1>       (every recorded instance; `<printed> in <method id>`)
#   fact     <instance> <encoded call signature> <methods before> <methods after> <call|invoke|self>
#            (`invoke`: `invoke(f, T, …)`, signature `T`, the one method chosen for it;
#             `self`: the instance's own argument types, what Julia dispatched to reach it,
#             only when its method is among them)
#   edge     <instance> <callee instance> <encoded call signature>  (a call with one target that covers it,
#            or an `invoke`, whose callee is the invoked method specialised on the actual arguments)
#   logged   <instance>                         (every instance in Julia's log)
#   summary  <key> <value>
#
# Signatures are encoded in prefix form, tokens separated by spaces:
# `any`, `bot`, `unk` (also a type variable with a lower bound), `n <name> <exact>` (a named type; `exact` is the full
# type without spaces, `~` when it has free type variables), `u <k> …` (a union
# of k types), `t <type>` (`Type{X}`), `v <type>` (`Vararg`), `tv <type>` (a type
# variable with this upper bound), `tup <k> …` (a tuple of k types). Methods are
# `<module>.<name>@<file>:<line>#<world it was added in>` (a method re-evaluated at
# its own location, as Revise does, gets a new id); a list of methods is space-separated, `-` when
# empty, `?` when Julia could not list them.

const SETUP = ARGS[1]
const CHANGE = ARGS[2]
const MAX_INSTANCES = length(ARGS) >= 3 ? parse(Int, ARGS[3]) : 50_000

Base.include(Main, SETUP)
const target = Main.TARGET

const widen = Core.Compiler.widenconst
const types = Dict{String,Tuple{Bool,Vector{String}}}()

typename_of(T::DataType) = string(T.name.module, ".", T.name.name)

function note_type!(T::DataType)
    name = typename_of(T)
    haskey(types, name) && return name
    supers = String[]
    S = T
    while S !== Any
        S = supertype(S)
        push!(supers, S === Any ? "Any" : typename_of(S))
    end
    types[name] = (isabstracttype(T), supers)
    name
end

function encode(T)::String
    T === Any && return "any"
    T === Union{} && return "bot"
    # A lower bound is not encoded; such a variable is unknown (never a supertype).
    T isa TypeVar && return T.lb === Union{} ? "tv " * encode(T.ub) : "unk"
    T isa UnionAll && return encode(Base.unwrap_unionall(T))
    T isa Union && (us = Base.uniontypes(T); return "u $(length(us)) " * join(map(encode, us), " "))
    T isa Core.TypeofVararg && return "v " * (isdefined(T, :T) ? encode(T.T) : "any")
    T isa DataType || return "unk"
    if T.name === Base.typename(Tuple)
        return "tup $(length(T.parameters))" * join(map(p -> " " * encode(p), T.parameters))
    elseif T.name === Base.typename(Type)
        X = T.parameters[1]
        # A closed `UnionAll` value (`Type{AbstractArray{Float64}}`) keeps its spelling,
        # so that two `Type{…}` can be compared by equality.
        if X isa UnionAll && !Base.has_free_typevars(X) && Base.unwrap_unionall(X) isa DataType
            return "t n $(note_type!(Base.unwrap_unionall(X))) $(replace(string(X), " " => ""))"
        end
        return "t " * (X isa Type || X isa TypeVar ? encode(X) : "unk")
    end
    name = note_type!(T)
    exact = Base.has_free_typevars(T) ? "~" : replace(string(T), " " => "")
    "n $name $exact"
end

method_id(m::Method) = string(m.module, ".", m.name, "@", basename(string(m.file)), ":", m.line, "#", m.primary_world)

const methods_seen = Dict{String,Method}()

function matches(sig, world)
    ms = Base._methods_by_ftype(sig, -1, world)
    ms isa Vector || return "?"
    isempty(ms) && return "-"
    ids = String[]
    for mm in ms
        id = method_id(mm.method)
        methods_seen[id] = mm.method
        push!(ids, id)
    end
    join(sort(ids), " ")
end

# What `invoke(f, T, …)` reaches: the one method chosen for `T`.
function invoked_method(sig, world)
    m = try
        Base._which(sig; world = world).method
    catch
        return "?"
    end
    id = method_id(m)
    methods_seen[id] = m
    id
end

within(m::Module, top::Module) = m === top || (m !== parentmodule(m) && within(parentmodule(m), top))

function argtype(ci, x)
    x isa Core.SSAValue && return ci.ssavaluetypes[x.id]
    x isa Core.SlotNumber && return ci.slottypes[x.id]
    x isa Core.Argument && return ci.slottypes[x.n]
    if x isa GlobalRef
        return isdefined(x.mod, x.name) && isconst(x.mod, x.name) ? Core.Const(getglobal(x.mod, x.name)) : Any
    end
    x isa QuoteNode && return Core.Const(x.value)
    x isa Expr && return Any
    Core.Const(x)
end

struct Fact
    caller::Core.MethodInstance
    sig::Any
    before::String
    kind::String   # "call", "invoke" or "self"
end

reach(f::Fact, world) = f.kind == "invoke" ? invoked_method(f.sig, world) : matches(f.sig, world)

function record(world)
    roots = Core.MethodInstance[]
    Base.visit(Core.methodtable) do m
        within(m.module, target) || return
        for mi in Base.specializations(m)
            isdefined(mi, :cache) && push!(roots, mi)
        end
    end
    seen = Set{Core.MethodInstance}(roots)
    queue = copy(roots)
    facts = Fact[]
    edges = Tuple{Core.MethodInstance,Core.MethodInstance,Any}[]
    calls_unknown = 0
    instances_failed = 0
    while !isempty(queue)
        mi = popfirst!(queue)
        local ci
        try
            res = Base.code_typed_by_type(mi.specTypes; optimize = false, world = world)
            ci = first(res)[1]
        catch
            instances_failed += 1
            continue
        end
        ci isa Core.CodeInfo || (instances_failed += 1; continue)
        # Only an instance that dispatch on its own types reaches has a self fact
        # (one made by `invoke` for a less specific method has none).
        own = matches(mi.specTypes, world)
        if own == "?" || (mi.def isa Method && method_id(mi.def) in split(own, ' '))
            push!(facts, Fact(mi, mi.specTypes, own, "self"))
        end
        sigs = Set{Any}()
        for stmt in ci.code
            stmt isa Expr && stmt.head === :call || continue
            ft = argtype(ci, stmt.args[1])
            invoked = nothing
            if ft isa Core.Const && ft.val === Core.invoke && length(stmt.args) >= 3
                # `invoke(f, T, args…)` reaches the method chosen for `T`: a fact on `T`.
                ft = argtype(ci, stmt.args[2])
                tt = argtype(ci, stmt.args[3])
                invoked = tt isa Core.Const && tt.val isa Type && tt.val <: Tuple ? tt.val : Tuple{Vararg{Any}}
            elseif ft isa Core.Const
                f = ft.val
                (f isa Core.Builtin || f isa Core.IntrinsicFunction) && continue
            end
            F = widen(ft)
            if !(F isa DataType && (Base.issingletontype(F) || F.name === Base.typename(Type)))
                calls_unknown += 1
                continue
            end
            sig = if invoked === nothing
                Tuple{F,(widen(argtype(ci, a)) for a in stmt.args[2:end])...}
            else
                Base.rewrap_unionall(Tuple{F,Base.unwrap_unionall(invoked).parameters...}, invoked)
            end
            kind = invoked === nothing ? "call" : "invoke"
            (sig, kind) in sigs && continue
            push!(sigs, (sig, kind))
            f = Fact(mi, sig, "", kind)
            push!(facts, Fact(mi, sig, reach(f, world), kind))
            if kind == "invoke"
                # Follow `invoke` to the invoked method, specialised on the actual arguments.
                m = try
                    Base._which(sig; world = world).method
                catch
                    nothing
                end
                if m !== nothing
                    at = Tuple{F,(widen(argtype(ci, a)) for a in stmt.args[4:end])...}
                    ti, env = ccall(:jl_type_intersection_with_env, Any, (Any, Any), at, m.sig)::Core.SimpleVector
                    if ti !== Union{}
                        callee = Core.Compiler.specialize_method(m, ti, env)
                        push!(edges, (mi, callee, sig))
                        if !(callee in seen) && length(seen) < MAX_INSTANCES
                            push!(seen, callee)
                            push!(queue, callee)
                        end
                    end
                end
                continue
            end
            # One method that covers the call: Julia compiles it as a direct call
            # (also when an argument type is abstract).
            ms = Base._methods_by_ftype(sig, -1, world)
            if ms isa Vector && length(ms) == 1 && ms[1].fully_covers
                callee = Core.Compiler.specialize_method(ms[1])
                push!(edges, (mi, callee, sig))
                if !(callee in seen) && length(seen) < MAX_INSTANCES
                    push!(seen, callee)
                    push!(queue, callee)
                end
            end
        end
    end
    facts, edges, seen, length(roots), calls_unknown, instances_failed
end

function all_methods()
    s = Set{Method}()
    Base.visit(m -> push!(s, m), Core.methodtable)
    s
end

world0 = Base.get_world_counter()
facts, edges, instances, n_roots, calls_unknown, instances_failed = record(world0)
before_methods = all_methods()

log = ccall(:jl_debug_method_invalidation, Any, (Cint,), 1)
try
    Base.include(Main, CHANGE)
finally
    ccall(:jl_debug_method_invalidation, Any, (Cint,), 0)
end
world1 = Base.get_world_counter()

logged = Set{Core.MethodInstance}(x for x in log if x isa Core.MethodInstance)
# The script's own closures are made after the snapshot; only the change counts.
new_methods = [m for m in all_methods() if !(m in before_methods) && m.file !== :none && String(m.file) != @__FILE__]

rows = String[]
# Two instances can print alike (`area(::Circle)` of `area(::Circle)` and of
# `area(::Shape)`, made by `invoke`): the method's id keeps them apart.
iname(mi) = replace(string(mi), "\t" => " ", "\n" => " ") * (mi.def isa Method ? " in " * method_id(mi.def) : "")
for f in facts
    after = reach(f, world1)
    push!(rows, join(("fact", iname(f.caller), encode(f.sig), f.before, after, f.kind), "\t"))
end
for (a, b, sig) in edges
    push!(rows, join(("edge", iname(a), iname(b), encode(sig)), "\t"))
end
for mi in instances
    push!(rows, join(("instance", iname(mi), mi in logged ? "1" : "0"), "\t"))
end
for mi in logged
    push!(rows, join(("logged", iname(mi)), "\t"))
end
for m in new_methods
    push!(rows, join(("new", method_id(m), encode(m.sig)), "\t"))
end
for (id, m) in methods_seen
    push!(rows, join(("method", id, encode(m.sig)), "\t"))
end
for (name, (abstract, supers)) in types
    push!(rows, join(("type", name, abstract ? "1" : "0", join(supers, " ")), "\t"))
end
unique!(sort!(rows))
foreach(println, rows)
println("summary\troots\t", n_roots)
println("summary\tinstances\t", length(instances))
println("summary\tcalls_unknown_callee\t", calls_unknown)
println("summary\tinstances_not_inferred\t", instances_failed)
println("summary\tmax_instances\t", MAX_INSTANCES)
println("summary\tjulia\t", VERSION)
