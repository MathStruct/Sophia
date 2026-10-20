#overview

> Rust in Sophia: identified through Moonkale's WP-66 normal form, resolved only within a file. Planned next: `matrixmultiply`, for a cross-language comparison with Julia's linear algebra (B-16 rung 3). 2026-10-20.
>
> Sources: Moonkale `moonkale-content-id` (Rust normal form); `crates/sophia-ingest/src/code.rs`; `tests/code.rs` (Moonkale's golden fixtures); the Rust Reference (name resolution, traits, macros); [[Real Code Corpora]] §2.

| Level | Status | Evidence |
|---|---|---|
| L1 Parsed | yes: tree-sitter-rust via arborium | golden fixtures |
| L2 Normalised | yes: WP-66, identical ids to Moonkale | `tests/code.rs`: every Rust digest in Moonkale's `references.tsv` |
| L3 Resolved | **same-file names only**, by spelling | struct expressions are uses since v3 |
| L4–L6 | no | — |

## What Rust resolution would need

- **Modules and `use`**: crate-wide paths (`crate::a::b`), re-exports, glob imports. The analogue of [[Julia Name Resolution]], and simpler because Rust's resolution is static.
- **Traits**: a method call `x.f()` resolves through the type and the traits in scope. This is Rust's dispatch: static (monomorphised) or dynamic (`dyn Trait`). The analogue of [[Julia Multiple Dispatch and World Age]], but decidable at compile time, so rust-analyzer or `rustc` can answer it exactly.
- **Macros**: `macro_rules!` and procedural macros expand before resolution; the same options as [[Julia Macros and Generated Functions]], with `cargo expand` as the oracle.
- **Generics and monomorphisation**: each instantiation is a derived program, keyed like a Julia specialisation.

The conformance oracle for Rust is rust-analyzer's or `rustc`'s resolution (SCIP or LSIF exports), as Julia's own method table is for Julia.

## Planned: `matrixmultiply` (rung 3)

A pure-Rust gemm. The question is cross-language: does Sophia find the same **structure** (blocking, packing, micro-kernels) in Rust and in Julia (LinearAlgebra's generic matmul, or Octavian.jl)? Ids will differ (different languages); the comparison is between graphs. Hypothesis and failure criteria in [[Semantic Boundary Experiments]] E-2.
