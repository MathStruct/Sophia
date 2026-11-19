#implementation #experiment

> Part 3 of the Julia conformance suite (dispatch): for every module-level name Sophia resolves in a definition's body, does Julia's lowered code use the same function, method or binding; and which of Julia's uses does Sophia leave open? Source: `julia_dispatch.rs`; the comparison is `sophia_ingest::conformance` ([[conformance]]). Results: [[Julia Conformance]].
>
> Sources: [[Julia Name Resolution]]; [[Julia Multiple Dispatch and World Age]]; `tests/julia-semantics/calls.jl` ([[calls]]). code: `julia_dispatch.rs`, `src/conformance.rs`

```sh
julia crates/sophia-ingest/tests/julia-semantics/calls.jl LinearAlgebra > calls.tsv
cargo run --release -p sophia-ingest --example julia_dispatch -- <stdlib>/LinearAlgebra calls.tsv [--rows]
```

Prints the count per outcome and examples of each; `--rows` prints every comparison instead.

It also prints part 4's count (world change): the functions with a single definition that some caller refers to by content, how many definitions refer to them directly, and per function how many ids contain its id, the ids a second method would move (`Ledger::dependants`), with the five largest.
