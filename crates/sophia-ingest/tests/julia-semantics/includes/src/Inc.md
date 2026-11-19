#implementation #julia

> The fixture of part 9 of [[Julia Conformance]] (include order): a package `Inc` whose root file includes others in every way Julia allows. Source: `Inc.jl` and the files it includes, one method each.
>
> Sources: code: `Inc.jl`, `../truth.tsv`, `../../includes.jl` ([[includes]]), `tests/includes.rs`

- Literal includes, nested (`sub/b.jl` → `sub/c.jl`, `module Deep` → `sub/deep.jl`), in `module Inner`; one file into two modules (`twice.jl`); one file twice into one module (`again.jl`); a path not in normal form (`sub/../norm.jl`).
- Includes the literal rule does not see: `Inc.include`, `Base.include`; dynamic ones in `module Dyn` (`joinpath`, a loop, a `mapexpr`).
- Includes Julia never evaluates at load: `@static if false`, a function nobody calls, a quoted expression.
- An include in a function defined first and called last (`late.jl`): the order differs.
- `shared` has a method in `Inc.jl` and one in `qualified.jl`, which Sophia misses.

Every file exists, so that discovery does not fail (`a_missing_file_in_a_dead_branch_refuses_the_package` covers that case in a temporary package). Regenerate the truth with `julia includes.jl Inc includes/src | sort > includes/truth.tsv` from `tests/julia-semantics`.
