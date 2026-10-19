#implementation

> The canonical tag-length-value framing shared with Moonkale's WP-66 normaliser. Source: `canonical.rs`.
>
> Sources: Moonkale Code Graph Schema, "WP-66 normalizer v1 → Request and encoding"; [[SPEC-001 Sophia Store v0]] §4.1. code: `canonical.rs`

## What it does

A field is `u32le(tag length) ‖ tag ‖ u32le(payload length) ‖ payload`. `Writer` builds fields, including nested ones (`open` reserves the length and `close` patches it). `Reader` walks them, and every malformed input is an error, never a panic (the truncation test cuts a field at every byte).

## Why this framing

- It is **injective**: a stream of fields can be parsed back in exactly one way, so two different structures can never produce the same bytes and therefore the same hash.
- It is **self-describing**, so the store codec ([[codec]]) uses it for values as well.
- It is **Moonkale's framing**, so the two systems build canonical streams the same way. Their ids still differ, because the prefixes differ.

## Costs and difficulties

Each field costs 8 bytes of lengths plus its tag. On the synthetic corpus that made canonical definitions about 40 bytes per term node ([[Benchmark Results]] 2026-10-20), larger than [[Block Size and Hash Overhead]] assumed. That is acceptable for hashing. For storage, a compact encoding is benchmark B-06's question.
