#implementation #julia

> Prints part 8's per-definition parser agreement tally and every wrong row, with file, line and differing fields. `--rows` prints every outcome. Source: `julia_parse.rs`.
>
> Sources: code: `julia_parse.rs`, `src/conformance.rs` (`parse_ledger`), `tests/julia-semantics/parse.jl`, [[Julia Conformance]]

```sh
julia crates/sophia-ingest/tests/julia-semantics/parse.jl <stdlib>/LinearAlgebra/src > parse-la.tsv
cargo run -p sophia-ingest --example julia_parse -- <stdlib>/LinearAlgebra parse-la.tsv
```

The first argument is the package directory with `Project.toml`; the oracle's argument is its `src/` directory. JuliaSyntax errors, malformed truth lines and undiscovered files are reported apart. Counts only, no timings. This does not change a normaliser or content id.
