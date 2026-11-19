#implementation #julia

> Source-only cases for part 8 of [[Julia Conformance]]: signatures, keywords, typed globals, destructuring constants, operators, callable objects, Unicode, strings, scopes and macro/doc wrappers. Source: `Pars.jl`.
>
> Sources: code: `Pars.jl`, `../../parse.jl`, `../truth.tsv`, `tests/parse.rs`

Each case starts on its own line. The package is parsed, never evaluated; this permits extending Base syntax without importing its functions and inspecting constructor syntax without installing methods. Regenerate the truth with the command in [[parse]]. Nested comments, public declarations and string macro spellings are deliberately included in the source around definitions.
