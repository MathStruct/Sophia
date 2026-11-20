#measurement #julia

> **E-3 (B-024): dispatch facts recorded from Julia, and the facts an added method makes stale, against Julia's own invalidations.** Recall is complete: on a fixture (11 changes) and on LinearAlgebra (9 changes a downstream package typically makes), no call whose reachable methods changed and no compiled instance relying on one is left out of Sophia's stale set, under either rule. Signature overlap alone is coarse where a new method is *less* specific than the existing ones (`area(::Any)`: precision 0.12 on calls); adding a specificity check (a method the call already reaches covers it and is strictly more specific than the new one) makes the calls exact on both: 35 of 35 on the fixture, 10 of 10 on LinearAlgebra, no false ones. At the level of compiled instances the fixture differs from Julia in one instance of 2 028 (103 of 103 invalidations found). On LinearAlgebra Julia invalidated **none** of the 4 259 recorded instances in any of the nine changes, because every call that changed is runtime dispatch on `Any` (or on an unknown `Type{T}`) whose result inference never used; Sophia's specificity rule marks 13 instances over the nine changes (at most 6 of 4 259 in one change), exactly the instances that rely on a changed call. The failure clause ("signature overlap far coarser than Julia's backedges") is **not met** for *dispatch facts* (which method a call can reach) and **met in part** for *compiled code*: what a compiled instance relies on is narrower than what its calls can reach, and only Julia's inference knows the difference.
>
> Sources: Belyakova et al., "World Age in Julia" (OOPSLA 2020); [[Julia Multiple Dispatch and World Age]]; [[SP-006 Open method tables versus content addressing]]; Julia 1.13 `src/gf.c` (`jl_debug_method_invalidation`). code: `crates/sophia-ingest/tests/julia-semantics/dispatch_facts.jl` ([[dispatch_facts]]), `src/invalidation.rs` ([[invalidation]]), `tests/invalidation.rs`, `examples/julia_invalidation.rs` ([[julia_invalidation]]); [[Review 2026-11-19 Codex B-024 (external)]].

## What was measured

A **dispatch fact** is one call in the inferred code of one compiled method instance: the caller, the argument types Julia inferred, and the methods a call with those types can reach (`Base._methods_by_ftype`, in a world). `dispatch_facts.jl` records every such fact for a module's compiled instances and for the instances their single-target calls reach (Base's included), plus each instance's own argument types (a `self` fact) and `invoke` calls (the one method chosen for the invoked signature). Then it adds one change and prints three things: what each call can reach afterwards, the call edges, and every instance in Julia's invalidation log (the log SnoopCompile's `@snoop_invalidations` reads; SnoopCompile is not needed).

Sophia's side (`invalidation.rs`) sees only the facts and the new methods' signatures, and decides with a small type lattice built from the recorded hierarchy:

- **Overlap:** a fact is stale when its signature may intersect a new method's (the rule [[SP-006 Open method tables versus content addressing]] proposed).
- **Specific:** overlap, unless a method the call already reaches covers the whole call and is strictly more specific than the new one; for `invoke`, also unless the new method is strictly smaller than the invoked signature.

Both are scored at two levels: **calls**, against the calls whose reachable methods changed; **instances**, the stale facts' instances and everything that calls them through recorded edges, against Julia's log.

## Fixture (in CI)

`tests/julia-semantics/invalidation/` (`Inval.jl`, `setup.jl` with a module `Helper` reached only by `invoke`, one change per file), truth from Julia 1.13.1 committed, `tests/invalidation.rs`. 386 call facts, 169 instances (`Inval`'s own and the Base and `Helper` instances reached from them).

| change | changed calls | Overlap (calls) | Specific (calls) | Julia invalidates | Specific (instances vs Julia) |
|---|---|---|---|---|---|
| `area(::Sq)`, more specific | 6 | 6/1/0 | 6/0/0 | 18 | 18/0/0 |
| `area(::Any)`, less specific than all | 1 | **1/7/0** | 1/0/0 | 2 | 2/0/0 (Overlap: 2/20/0) |
| `area(::Circle)` replaced | 6 | 6/1/0 | 6/0/0 | 17 | 17/**1**/0 |
| `area(::Shape)` replaced, from another file | 7 | 7/1/0 | 7/0/0 | 20 | 20/0/0 |
| `area(::Shape)` replaced at its own file and line (as Revise does) | 7 | 7/1/0 | 7/0/0 | 20 | 20/0/0 |
| new subtype `Tri <: Shape` with `area(::Tri)` | 5 | 5/1/0 | 5/0/0 | 16 | 16/0/0 |
| `Helper.h(::Int)`, inside a function `Inval` reaches only by `invoke` | 1 | 1/0/0 | 1/0/0 | 4 | 4/0/0 |
| `scale(::Vector{Int})` beside `scale(::Vector{Float64})` | 1 | 1/0/0 | 1/0/0 | 3 | 3/0/0 |
| `first_or(::Vector{Int}, ::Int)` beside a `where` method | 1 | 1/0/0 | 1/0/0 | 3 | 3/0/0 |
| `convert(::Type{Meters}, ::Int)` | 0 | 0/0/0 | 0/0/0 | 0 | 0/0/0 |
| `perimeter(::Sq)`, `length(::Circle)`: other functions | 0 | 0/0/0 | 0/0/0 | 0 | 0/0/0 |
| **sum (12 changes)** | **35** | 35/12/0 (P 0.74) | **35/0/0** | 103 | 103/1/0 (P 0.99) |

Scores are true/false/missed positives. The one disagreement with Julia is explained and tested:

- **Sophia more:** when `area(::Circle)` is replaced, an `area(::Shape)` call can reach a different method, so Sophia marks the instance compiled for `area(::Shape)`; Julia keeps it, because the replaced method was already more specific than `area(::Shape)` and still is. The instances relying on a changed fact include it too (`changed~julia` is 103/1/0, identical to Specific), so the difference is in what "changed" means for an instance's own dispatch, not in the rule.
- `total(v) = sum(area, v)` is invalidated only through Base's `mapreduce` chain; the recorded edges carry the staleness to it (test `a_more_specific_method_is_found_exactly`).
- `invoke(area, Tuple{Shape}, c)` makes an instance of the method `area(::Shape)` for `Circle`, printed exactly like the instance of `area(::Circle)`; instance names carry their method so the two are kept apart (before that, Julia seemed to invalidate an instance whose dispatch had not changed: test `the_instance_invoke_makes_is_its_own`).

## LinearAlgebra

Julia 1.13.1, `invalidation/linalg/setup.jl` (a workload of about 30 operations: products, solves, factorisations, norms, structured matrices), `TARGET = LinearAlgebra`: 948 compiled LinearAlgebra instances, 4 259 with the closure into Base, 14 605 call facts, 47 calls with a callee that is not a constant (skipped). One change per run, about 21 s and 8 MB per run; the runs are in `$SOPHIA_CORPORA/e3/` (commands in [[dispatch_facts]]).

| change (what a downstream package adds) | changed calls | Overlap (calls) | Specific (calls) | Julia invalidates (recorded / outside) | Specific (instances) |
|---|---|---|---|---|---|
| `abs(x) = x` (piracy, less specific than all) | 0 | **0/16/0** | 0/0/0 | 0 / 0 | 0 (Overlap: 135) |
| `MyNum <: Number` with `*`, `+`, `zero` | 8 | 8/0/0 | 8/0/0 | 0 / 0 | 6 |
| `MyMat <: AbstractMatrix{Float64}` with `size`, `getindex` | 0 | 0/0/0 | 0/0/0 | 0 / 0 | 0 |
| `MyOp` with a five-argument `mul!` | 0 | 0/0/0 | 0/0/0 | 0 / 0 | 0 |
| `convert(::Type{Float64}, ::MyReal)` | 1 | 1/0/0 | 1/0/0 | 0 / 4 | 3 |
| `promote_rule(::Type{MyFloat}, ::Type{Float64})` | 0 | 0/0/0 | 0/0/0 | 0 / 0 | 0 |
| `==(::Tag, b)` (loose second argument) | 0 | 0/0/0 | 0/0/0 | 0 / **714** | 0 |
| `dot(::Vector{Float64}, ::Vector{Float64})` (piracy) | 0 | 0/0/0 | 0/0/0 | 0 / 0 | 1 |
| `iterate`, `length` for `Count3` | 1 | 1/0/0 | 1/0/0 | 0 / 0 | 3 |
| **sum (9 changes)** | **10** | 10/16/0 (P 0.38) | **10/0/0** | 0 / 718 | 13 (Overlap: 148) |

- **Every changed call is on `Any` or an unknown `Type{T}`**: `+(::Any, ::Any)` and `zero(::Any)` in the generic matrix-vector product, `*`, `+`, `zero` in `lutype(::Type)` and `eigtype(::Type)`, `length(::Any)`, `convert(::Type{Float64}, ::Any)` in `setindex!`. What these calls can reach did change, but inference had already given up on them, so no compiled code relied on the set of methods, and Julia records no backedge to invalidate.
- **The 13 instances the specificity rule marks are exactly the ones relying on those calls** (`changed~julia` 13 as well): the rule is as precise as dispatch facts allow; the gap to Julia's 0 is between "what a call can reach" and "what compiled code relied on".
- **`dot` piracy:** the one marked instance is `dot(::Vector{Float64}, ::Vector{Float64})` itself, whose own dispatch now goes to the new method; only the workload calls it, so Julia has no caller to invalidate.
- **`==(::Tag, b)`:** Julia invalidates 714 instances, all outside LinearAlgebra's closure (Base's comparison code compiled before); none of LinearAlgebra's 14 605 calls changed.
- **Two lattice refinements came from these runs** (both with unit tests, recall unchanged): two known `Type{X}` overlap only when equal (a struct's constructor had matched every `AbstractArray{Float64}(x)` call: matrix and `mul!` changes, 12 false calls each, now 0); a closed `UnionAll` inside `Type{…}` keeps its spelling (`Type{AbstractArray{Float64}}`). Overlap's remaining 16 false calls are all `abs(x)`.

## What this says

- **For dispatch facts, signature overlap plus specificity is enough** on everything measured: complete recall at both levels, exact on the fixture's calls. Overlap alone fails precisely where SP-006 feared, on a method less specific than the existing ones (`area(::Any)`, `abs(x)`): every call of the function overlaps it, and only the specificity check sees that the new method can never win.
- **For compiled code, a fact store over-approximates Julia by construction.** A call on `Any` can reach a new method, so its dispatch fact is stale, but if inference never used its result no compiled code depends on it, and Julia records no backedge. Sophia cannot tell these apart from the facts recorded here; it would need what inference used (the backedges themselves, or the inferred return type per call). That is the difference between a *dispatch fact* and a *compilation fact*, and the two need separate records.
- **The type lattice decides the precision.** Before two refinements found on LinearAlgebra (two known `Type{X}` overlap only when equal; a closed `UnionAll` inside `Type{…}` keeps its spelling), a new struct's constructor made 12 calls falsely stale per change. Each refinement must keep `overlap` sound: one of them first was not (`Tuple{T} where T<:Real` is `Tuple{Real}`; Codex's second review), and equality is now trusted only for spellings without tuples, unions or `where`.

## Limits

- The facts are recorded from unoptimised inferred code; calls that exist only after optimisation (inlining, `invoke` inserted by the optimiser) are not separate facts, but the edges follow single-target calls, so their instances are recorded.
- The closure stops at single-target calls; an instance reached only by runtime dispatch is recorded only if it is the module's own. On LinearAlgebra Julia's log names instances outside the closure (`unrecorded`: 718 over the nine changes, 714 of them for `==(::Tag, b)`); they are Base's code, outside what LinearAlgebra's facts cover.
- Ambiguities a new method creates are not detected. Method ids put two constructors of one line together (no fact reached such a pair).
- Facts are not stored in Sophia yet: recording them as `tested` evidence citing a world, with what inference relied on beside what a call can reach, is the step this experiment justifies (proposal B-087).
- One workload and nine changes on one package; the lattice ignores type parameters except for equality of concrete types, which has been enough here and may not be on code with heavier parametric dispatch.
