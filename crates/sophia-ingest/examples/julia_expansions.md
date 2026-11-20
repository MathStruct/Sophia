#implementation #experiment

> Experiment E-4 (macro expansions as transformation records) on a real package: every module-level macro call's expansion spliced in and ingested; which methods the expansions recover, whether each expansion makes what its form makes, what the expansions cost, and whether equal input addresses give equal texts. Source: `julia_expansions.rs`; the comparison is `sophia_ingest::expansion` ([[expansion]]). Results: [[Semantic Boundary Experiments]] §E-4.
>
> Sources: `tests/julia-semantics/expansions.jl` ([[expansions]]), `tests/julia-semantics/macros.jl` ([[macros]]). code: `julia_expansions.rs`, `src/expansion.rs`

```sh
# from crates/sophia-ingest/tests/julia-semantics
julia expansions.jl LinearAlgebra <stdlib>/LinearAlgebra/src > expansions-la.tsv   # about 20 s
julia macros.jl LinearAlgebra <stdlib>/LinearAlgebra/src > macros-la.tsv           # optional, for faithfulness
cargo run --release -p sophia-ingest --example julia_expansions -- <stdlib>/LinearAlgebra expansions-la.tsv [macros-la.tsv] [--rows]
```

Prints: with `macros-la.tsv`, how many forms' records made exactly the form's methods; forms and records compared; records not stable or not parsed back; methods per outcome; bindings made and named; definitions and bytes as written and expanded; Julia's expansion and evaluation time and Sophia's (two ingests); transformation records, distinct addresses and those shared with a different text; methods per macro and outcome; the first open or refused methods with the reason. `--rows` prints every method instead (file, line, record, macro, owner, signature, outcome, detail). The LinearAlgebra run of 2026-11-19 is kept in `$SOPHIA_CORPORA/e4/`.
