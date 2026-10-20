#implementation #julia

> The ground truth for [[Julia Conformance]]: Julia itself lists every method a package defines, with its file, line and owning generic function. Source: `methods.jl`.
>
> Sources: Julia ≥ 1.12 (`Base.visit(Core.methodtable)`: one global method table). code: `methods.jl`, `examples/julia_conformance.rs`

```sh
julia crates/sophia-ingest/tests/julia-semantics/methods.jl LinearAlgebra > truth.tsv
```

- Owner of a function's method: `parentmodule(f).nameof(f)`, so `Base.Math.cbrt`, not the path it was written as.
- Owner of a constructor (`Type{T}` as the first signature element): `Module.T.new`.
- Callable objects and anything else: `?`.
- Only methods whose file is under `<PKG>/src/` are printed; the file is relative to it.

This is Julia, not Sophia code: it measures Julia. Sophia stays Rust (S-ADR-0004).
