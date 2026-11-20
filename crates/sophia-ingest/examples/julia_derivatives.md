#implementation #experiment

> Prints E-5's ledger on the fixture `tests/julia-semantics/rules` ([[Rules]]): per variant, which derived programs and which of the three keys changed against `base`; each key's score; per gradient the pieces, the first-call, walk and generator times and the gradient; the closures; the in-session call counts. Source: `julia_derivatives.rs`. The numbers are in [[Semantic Boundary Experiments]] §E-5.
>
> Sources: [[derivative]], [[derivatives]]. code: `julia_derivatives.rs`

```sh
cargo run -p sophia-ingest --example julia_derivatives
```

It reads the checked-in `truth.tsv`; to measure again, run the oracle first ([[derivatives]]).
