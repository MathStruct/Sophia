#implementation #julia

> **Sophia's stale set for an added Julia method, computed from recorded dispatch facts, and scored against Julia.** E-3 of [[Semantic Boundary Experiments]] (B-024); the results are in [[Dispatch Invalidation]]. `dispatch_facts.jl` records the facts; this module decides, from the facts and the new methods' signatures alone, which facts and which compiled instances are stale, and compares that with what actually changed and with Julia's own invalidation log.
>
> Sources: [[SP-006 Open method tables versus content addressing]]; [[Julia Multiple Dispatch and World Age]]; Julia 1.13 `src/gf.c` (`jl_debug_method_invalidation`). code: `invalidation.rs`, `tests/julia-semantics/dispatch_facts.jl` ([[dispatch_facts]]), `tests/invalidation.rs`, `examples/julia_invalidation.rs` ([[julia_invalidation]])

## What it does

- **`Ty`, `Ty::parse`:** a Julia type in the script's prefix encoding (named types with their full spelling, unions, `Type{X}`, `Vararg`, type variables by upper bound, tuples).
- **`Lattice`:** the nominal hierarchy the run needed (abstract or not, supertypes by name).
  - `overlap(a, b)` may answer *yes* where Julia's `typeintersect` is empty, never *no* where it is not: parameters are ignored, except that two concrete types overlap only when equal; a type variable counts as its upper bound; `Type{X}` overlaps only the kinds.
  - `subtype(a, b)` answers *yes* only when sure: a supertype is trusted by name only when it has no parameters; a variable on the right is never a supertype.
- **`Rule::Overlap`:** a fact is stale when its call signature overlaps a new method's ([[SP-006 Open method tables versus content addressing]]'s proposal).
- **`Rule::Specific`:** overlap, unless a method the call already reaches covers the whole call and is strictly more specific than the new one (then the new one can never be chosen). Sound because `subtype` only says yes when sure.
- **`Run::affected`:** the compiled instances that rely on marked facts: each marked fact's instance, the instance a marked call reached (by the recorded edge), and every caller of those, transitively. Facts of kind `self` are an instance's own argument types (what reached it); they count only here.
- **`Run::report`:** precision and recall at two levels: calls (against the calls whose reachable methods changed) and instances (against the instances Julia's log names).

## What it does not do

- It implements no part of Julia's subtyping beyond the nominal tree; anything else is answered conservatively (overlap: yes, subtype: no). Diagonal rules, `Union` distribution inside parameters and variance are not modelled.
- It stores nothing: the facts live in the run's TSV. Recording dispatch facts in the store (as `tested` evidence citing a world) is the step after this experiment, not built.
- It does not detect ambiguities a new method creates.
