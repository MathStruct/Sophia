#implementation #experiment

> Part 6 of the Julia conformance suite (closures and captured variables) on a real package: for each closure Julia's lowering made, each name in the matching closure of Sophia's parse, classified by Julia and by Sophia's encoding; and every call's keyword name the encoding loses. Source: `julia_closures.rs`; the comparison is `sophia_ingest::conformance::closure_ledger` ([[conformance]]). Results: [[Julia Conformance]] part 6.
>
> Sources: [[Julia Name Resolution]]; `tests/julia-semantics/closures.jl` ([[closures]]). code: `julia_closures.rs`, `src/conformance.rs`

```sh
julia crates/sophia-ingest/tests/julia-semantics/closures.jl LinearAlgebra <stdlib>/LinearAlgebra/src > closures.tsv
cargo run --release -p sophia-ingest --example julia_closures -- <stdlib>/LinearAlgebra closures.tsv [--rows | --all]
```

Prints how many of Julia's closure methods were matched to a closure of Sophia's parse (and how many lie in refused definitions, in no definition, or match none or several, the first ten of them; `--all` lists every one), the count per outcome and per Julia class against outcome, every row that is not a "knows", and the keyword names lost, by name. `--rows` prints every row instead (tab-separated: file, line, definition, closure, name, outcome, detail), then one `KeywordUnspelled` row per keyword name lost.
