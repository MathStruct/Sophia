#implementation #experiment

> Part 7 of the Julia conformance suite (mutable globals) on a real package: for each global binding that holds data, whether Sophia gives it a definition with an id, and for each use of one in a method, whether Sophia's id treats it as content, as a generic function, as a local, or as state (only the spelling). Source: `julia_globals.rs`; the comparison is `sophia_ingest::conformance::global_ledger` ([[conformance]]). Results: [[Julia Conformance]] part 7.
>
> Sources: [[Julia Name Resolution]]; `tests/julia-semantics/globals.jl` ([[globals]]). code: `julia_globals.rs`, `src/conformance.rs`

```sh
julia crates/sophia-ingest/tests/julia-semantics/globals.jl LinearAlgebra <stdlib>/LinearAlgebra/src > globals.tsv
cargo run --release -p sophia-ingest --example julia_globals -- <stdlib>/LinearAlgebra globals.tsv [--rows]
```

Prints the bindings by Julia's kind against outcome (and each one that is not a "knows"), then the uses by Julia's kind, access and outcome, every use row that is not a "knows" and the uses not compared. `--rows` prints every binding and use row instead (tab-separated).
