#implementation #julia

> Additional source-scope cases for part 8 of [[Julia Conformance]]: a nested named method, explicit `@doc`, a typed constant a conditional definition and a typed keyword default inside `@deprecate`. Source: `More.jl`.
>
> Sources: code: `More.jl`, `Pars.jl`, `../../parse.jl`, `../truth.tsv`, `tests/parse.rs`

Included literally by `Pars.jl`, so both source files enter `module::discover`. Nested methods are definitions; their signature assignments and local typed assignments are not module-level typed globals. A typed constant must not also be emitted by the oracle as a typed global.

The typed keyword default is an argument of an unexpanded macro call, not a module-level typed global or a source method. Sophia currently emits a false short-method record for it.
