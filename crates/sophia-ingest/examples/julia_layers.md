#implementation #experiment

> Prints E-6's measurements on the E-5 fixture's `base` variant: per gradient the size of the derived program at each layer (Zygote's with its pieces, Enzyme's normalised and raw), the addresses its text had, the Zygote–Enzyme distance in ulps and the times; then the totals, what the layers share, and what an SQLite store grows by when both layers are stored. Source: `julia_layers.rs`. The numbers are in [[Semantic Boundary Experiments]] §E-6.
>
> Sources: [[layers]], [[layer_programs]]. code: `julia_layers.rs`

```sh
cargo run -p sophia-ingest --example julia_layers
```

It reads the checked-in `tests/julia-semantics/layers/truth.tsv`; to measure again, run the oracle first (`tests/julia-semantics/layer_programs.md`).
