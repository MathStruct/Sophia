#implementation #julia

> The ground truth for part 9 of [[Julia Conformance]] (include order): every file Julia evaluates when it loads a package from source, in the order it starts each one and with the module the file's code runs in; and for each method of the package, the file, line and module Julia records for it. Source: `includes.jl`.
>
> Sources: Julia 1.13 `base/loading.jl` (`Base.include_callbacks`, run in `_include` before a file's code is evaluated, on the path `_include_dependency` returns in normal form); `Method.file`, `Method.line`, `Method.module`. code: `includes.jl`, `src/conformance.rs` (`include_ledger`), `tests/includes.rs`, `examples/julia_includes.rs`

```sh
# from crates/sophia-ingest/tests/julia-semantics
julia includes.jl Inc includes/src | sort > includes/truth.tsv
julia includes.jl LinearAlgebra <stdlib>/LinearAlgebra/src > includes-la.tsv   # a few seconds; loads LinearAlgebra from source
```

- `include  seq  Module  file`: one line per evaluation of a file, `seq` counting from 0 (the root file, evaluated in `Main`, whose module is printed `-`). Every form of include reaches the callback: `include("x.jl")`, `M.include(…)`, `Base.include(M, …)`, `include(mapexpr, …)`, a computed path, an include in a function body that runs at load. A file evaluated twice has two lines. `file` is relative to SRCDIR, or `outside:<absolute path>`.
- `method  file  line  Module  name`: each (file, line, module, name) of a method defined in a file Julia evaluated (also outside SRCDIR, keyed as in `include` lines; module `-` for Main) that `which` still dispatches to (a method replaced by a later evaluation of the same signature is not listed).
- The package is loaded by `include`-ing its root file into `Main`, as parts 7 and 8 do: `__init__` does not run, and includes it would make are not seen. Precompilation is not involved.

This is Julia, not Sophia code: it measures Julia. Sophia stays Rust (S-ADR-0004).
