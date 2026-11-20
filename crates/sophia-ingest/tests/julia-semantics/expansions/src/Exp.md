#implementation #julia

> The E-4 fixture: a package whose module-level macro calls cover the cases [[expansions]] distinguishes, each with the outcome `tests/expansions.rs` expects. Source: `Exp.jl`.
>
> Sources: [[Semantic Boundary Experiments]] §E-4. code: `Exp.jl`, `../truth.tsv` (`expansions.jl`), `../macros.tsv` (`macros.jl`), `tests/expansions.rs`

- `@eval` loops: a qualified name spliced twice (`Base.$f`), types as values, a splat `$(args...)`, docstrings made inside the loop, `@static` around a loop, `@eval Inner ex` (another module).
- Hidden state: a closure as a loop value (its `repr` does not read back), a macro that counts its calls (`@counted`). Both must be `stable = no`.
- Macros of the package (`@twice`, hygienic locals) and of Base (`@deprecate`, `@enum`, `@kwdef`, `@inline`, with and without a docstring).
- A macro inside an `if`: not expanded, its method is record 0.

Regenerate both truths after a change (from `tests/julia-semantics`): `julia expansions.jl Exp expansions/src > expansions/truth.tsv` and `julia macros.jl Exp expansions/src | sort > expansions/macros.tsv`.
