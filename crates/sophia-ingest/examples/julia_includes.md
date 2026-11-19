#implementation #julia

> Prints part 9's include-order tally (files and methods), every row that is not `Knows`, and the files whose order differs. `--rows` prints every row. Source: `julia_includes.rs`.
>
> Sources: code: `julia_includes.rs`, `src/conformance.rs` (`include_ledger`), `tests/julia-semantics/includes.jl`, [[Julia Conformance]]

```sh
julia crates/sophia-ingest/tests/julia-semantics/includes.jl LinearAlgebra <stdlib>/LinearAlgebra/src > includes-la.tsv
cargo run --release -p sophia-ingest --example julia_includes -- <stdlib>/LinearAlgebra includes-la.tsv
```

The first argument is the package directory with `Project.toml`; the oracle's argument is its `src/` directory. Run `julia` directly, not through `nix develop -c`, whose banner goes to standard output and becomes one malformed line. Counts only, no timings.
