#implementation #julia

> CI checks parser agreement against committed JuliaSyntax source truth, without invoking Julia. Source: `parse.rs`.
>
> Sources: code: `parse.rs`, `julia-semantics/parse/truth.tsv`, `src/conformance.rs` (`parse_ledger`), [[Julia Conformance]]

```sh
cargo test -p sophia-ingest --test parse
```

The fixture checks every case, including keyword argument loss, a typed global mistaken for a short method, refusals and the shared struct kind. Falsified truth independently changes each comparison field and removes/adds definitions, so misses, extras and disagreements cannot pass silently. Malformed rows (including invalid forms, counts, spans, keyword ordering and contradictory duplicate keys) and Julia error diagnostics are tested separately. A recovery-error definition is explicitly refused with or without a Julia diagnostic; oracle files outside literal include discovery are listed apart.
