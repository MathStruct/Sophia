#implementation #experiment

> Compares Sophia's view of a Julia package with Julia's own: for each method Sophia stores, does Julia say the same generic function owns it, and which of Julia's methods does Sophia not see? Source: `julia_conformance.rs`. Results: [[Julia Conformance]].
>
> Sources: [[Julia Name Resolution]]; `tests/julia-semantics/methods.jl` (the ground truth, printed by Julia from its global method table). code: `julia_conformance.rs`, `sophia_ingest::binding`

```sh
julia crates/sophia-ingest/tests/julia-semantics/methods.jl LinearAlgebra > truth.tsv
cargo run --release -p sophia-ingest --example julia_conformance -- <stdlib>/LinearAlgebra truth.tsv
```

- **Julia's side:** `Base.visit(Core.methodtable)` (Julia ≥ 1.12 has one global method table); for each method from the package's `src/`, its file, line and owner: `parentmodule(f).name`, or `Module.T.new` for a constructor.
- **Sophia's side:** the module-mode ingest and `binding`, the rule `store_items` uses.
- **Matching** is by (file, line of the definition's first token). Constructor naming differs by convention, so `.new` is ignored on both sides.
- **Output:** methods with the same owner, a different owner (grouped by module pattern, with examples), or no Julia method at that line; and how many of Julia's methods Sophia never sees.
- **`--by-name`** reproduces the grouping before 2026-10-20 (bare name under the package), so the effect of the (module, name) key is measurable: 53.7 % → 94.5 % on LinearAlgebra.
