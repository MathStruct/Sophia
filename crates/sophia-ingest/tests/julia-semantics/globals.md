#implementation #julia

> The ground truth for part 7 of [[Julia Conformance]] (mutable globals): every global binding a package owns that holds data (a constant whose value is not a function, type or module, or any mutable global, whatever it holds: a mutable global bound to a type, `ClosedReal = T where T <: …`, is still state), with its kind in Julia's binding partition (a constant, a mutable global, a `global x` declaration); and per method of the package, each such binding its lowered code reads or writes, and each of its arguments and locals spelled like one. Source: `globals.jl`.
>
> Sources: Julia ≥ 1.12 (binding partitions: `Base.binding_kind` and its `PARTITION_KIND_*` values, internal; `Core.get_binding_type`; `Base.uncompressed_ast`; `CodeInfo.slotnames`). code: `globals.jl`, `src/conformance.rs` (`global_ledger`), `tests/globals.rs`, `examples/julia_globals.rs`

```sh
# from crates/sophia-ingest/tests/julia-semantics
julia globals.jl Glob globals/src | sort > globals/truth.tsv
julia globals.jl LinearAlgebra <stdlib>/LinearAlgebra/src > globals-la.tsv   # a few seconds
```

- `binding  Module.name  kind  type  object`: kind `const` (also `BACKDATED_CONST`, `UNDEF_CONST`), `global` (`PARTITION_KIND_GLOBAL`: assigned, or declared by a function's `global x = v`) or `declared` (`global x` at module level, nothing assigns it); type is a mutable global's declared type (`Any` when none); object is `mutable` or `immutable` for a constant's value (`ismutable`, except that a string or symbol is immutable). Imports, `using` and guard bindings (a name read and never defined, which lowering makes) are not the module's own and are left out, and so are compiler-made names (`#…`), `eval` and `include`.
- `use  file  line  name  Module.name  access  kind`: access is `read` (a `GlobalRef`, `getproperty(M, :x)` or `getglobal` on a module), `write` (`Core.setglobal!(M, :x, v)`, which `global x = v` in a body lowers to, or `setproperty!(M, :x, v)`), or `local` (a slot of the method spelled like a data binding of the method's module: the name is the local there).
- Methods are those under the package's `src/` (or SRCDIR) that `which` still dispatches to. A closure is its own method at its own line; a generator's body (filed at `none:0`) is read as part of the method that creates it.
- A constant's initializer (`const B = 2 * A`) is no method: Julia evaluates it once, when the module loads, so no `use` line comes from it.
- Only the package's own modules are read (the root and the submodules bound in it).

This is Julia, not Sophia code: it measures Julia. Sophia stays Rust (S-ADR-0004).
