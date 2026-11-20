#implementation #julia

> **The ground truth for E-3 (dispatch facts and invalidation):** for every call in every compiled method instance of a module (and of the instances those calls reach), the argument types Julia inferred and the methods the call can reach; then one change is evaluated, and the script prints what each call can reach afterwards and which instances Julia's own invalidation log names. Source: `dispatch_facts.jl`; results in [[Dispatch Invalidation]].
>
> Sources: Julia 1.13 `src/gf.c` (`jl_debug_method_invalidation`, the log SnoopCompile's `@snoop_invalidations` reads), `Base._methods_by_ftype`, `Base.code_typed_by_type`, `Base.specializations`, `Core.Compiler.specialize_method`. code: `dispatch_facts.jl`, `src/invalidation.rs` ([[invalidation]]), `tests/invalidation.rs`, `examples/julia_invalidation.rs` ([[julia_invalidation]])

```sh
# from crates/sophia-ingest/tests/julia-semantics; the fixture (in CI), about 1 s per change
for c in invalidation/changes/*.jl; do
  julia dispatch_facts.jl invalidation/setup.jl $c > invalidation/truth/$(basename $c .jl).tsv; done
# LinearAlgebra, about 45 s and 8 MB per change
for c in invalidation/linalg/changes/*.jl; do
  julia dispatch_facts.jl invalidation/linalg/setup.jl $c > $SOPHIA_CORPORA/e3/$(basename $c .jl).tsv; done
```

The header of `dispatch_facts.jl` documents the rows and the type encoding. What it records and why:

- **Instances:** every method instance of a method of `TARGET` (or a submodule) that has compiled code, after `setup.jl`'s workload; then, breadth first, the instance of every call with exactly one method that covers it (Julia compiles those as direct calls, also when an argument type is abstract), up to `max_instances` (default 50 000).
- **Call facts:** each `:call` in the instance's unoptimised inferred code whose callee is a known function or a `Type{X}`, with the widened argument types; calls to builtins and intrinsics have no method table and are skipped; a callee that is not a constant is counted (`calls_unknown_callee`) and skipped. The methods are `Base._methods_by_ftype(sig, -1, world)`.
- **Self facts:** the instance's own `specTypes` with its reachable methods: an instance is invalidated when its own dispatch changes, not only its calls.
- **Edges:** (caller, callee instance, call signature) for the calls followed.
- **The change** is `Base.include(Main, change)`, with the log on (`ccall(:jl_debug_method_invalidation, Any, (Cint,), 1)`). Every `MethodInstance` in the log is "invalidated by Julia". New methods are the methods present after and not before, without the script's own closures.
- **Method ids** are `module.name@file:line`; two methods on one line (a struct's two constructors) share an id. No fact in the runs so far reaches such a pair.

What it cannot see: calls that are not in unoptimised code as `:call` (`invoke`, `Core._apply_iterate` splats, `ccall`), instances outside the closure (counted by the Rust side as `unrecorded`), and runtime dispatch of an untyped call, which Julia records as a method-table edge and the script as a call fact on `Any`.

This is Julia, not Sophia code: it measures Julia. Sophia stays Rust (S-ADR-0004).
