#implementation #julia

> The ground truth for part 5 of [[Julia Conformance]] (macros): for each module-level form of a package that calls a macro, Julia's own account of what the form made (methods, by generic function; names bound) and whether the macro changes the definition it wraps. Source: `macros.jl`.
>
> Sources: Julia ≥ 1.12 (`Method.primary_world`, `Base.get_world_counter`, `Meta.lower`, `Base.visit(Core.methodtable)`). code: `macros.jl`, `src/conformance.rs` (`macro_ledger`), `tests/macros.rs`, `examples/julia_macros.rs`

```sh
# from crates/sophia-ingest/tests/julia-semantics
julia macros.jl Mac macros/src | sort > macros/truth.tsv
julia macros.jl LinearAlgebra <stdlib>/LinearAlgebra/src > macros-la.tsv   # about 10 s
```

- **Loading.** The script loads the package from its source itself, form by form, as `include` does: it parses each file (`Meta.parseall`), evaluates each module-level form on its own and reads Julia's world counter before and after it. Every method entering Julia's method table gets a `primary_world`, so each method belongs to the form during which it was made, **wherever Julia files it**: a method a macro's `quote` makes carries the macro definition's line (`@twice` in the fixture), `@enum`'s carry Base's. Bodies of `module … end` are followed form by form, and so is `include("literal.jl")` at module level; a module with a docstring is loaded without the docstring. A computed `include` runs as written (its forms are one form).
- **Which forms.** A form is printed when it is a macro call (a docstring is `Core.@doc`) or contains one outside a definition (`for … @eval … end`, `@static if`). Macro calls inside a function's body are not part 5's subject: part 3 compares bodies against lowered code, where they are already expanded.
- **`form`** lines: the form's file and first line, its module, the macro's binding (resolved in the module: `Base.@inline`, `Base.Enums.@enum`) or the block's head (`for`, `if`), and `changes`. For a macro call whose last argument is a definition, `changed` if `Meta.lower` of the form differs from `Meta.lower` of the bare definition after line numbers and gensym counters are erased, `same` if not; `@eval` on a definition without `$` is `same` (it evaluates its argument as written), with `$` it is `-` (a template: there is no bare definition to compare); a docstring is `doc`; anything else `-`.
- **`method`** lines: one per method the form made, with its generic function (`parentmodule.nameof`; `(::M.T)` for a callable object's method, `(::Type)` for a constructor of a type parameter, `(::Type{T})(…) where T`) and its `file:line` (`elsewhere:line` outside the package). Closures and other compiler-made functions (`#…`) and keyword sorters (`Core.kwcall`) are left out.
- **`binding`** lines: each name the form bound in its module (bound after, not before: `isdefined`), with its kind (`function`, `macro`, `type`, `module`, `value`). An `export` lists a name before it is bound, so names are compared by `isdefined`, not by `names`.
- A form that throws stops the script with Julia's error (nothing is printed); a literal `include` inside a form (not at module level) is run by Julia's own `include`, so its methods count for that form.
- The fixture's output is sorted before committing (`truth.tsv`).

This is Julia, not Sophia code: it measures Julia. Sophia stays Rust (S-ADR-0004).
