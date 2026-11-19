#implementation #experiment

> Part 5 of the Julia conformance suite (macros) on a real package: for every module-level form that calls a macro, what Julia's form made against the definitions Sophia records in it, and whether Sophia's id sees a macro around a definition. Source: `julia_macros.rs`; the comparison is `sophia_ingest::conformance::macro_ledger` ([[conformance]]). Results: [[Julia Conformance]] part 5.
>
> Sources: [[Julia Macros and Generated Functions]]; `tests/julia-semantics/macros.jl` ([[macros]]). code: `julia_macros.rs`, `src/conformance.rs`

```sh
julia crates/sophia-ingest/tests/julia-semantics/macros.jl LinearAlgebra <stdlib>/LinearAlgebra/src > macros.tsv
cargo run --release -p sophia-ingest --example julia_macros -- <stdlib>/LinearAlgebra macros.tsv [--rows]
```

Prints the forms compared (and those Sophia's parse has no form for), Julia's methods made by them and how many were left open, the count per outcome, the count per macro and outcome, and the methods left open per macro. `--rows` prints every row instead (tab-separated: file, line, macro, name, outcome, detail).
