#design

> **Status: accepted (2026-10-20, Daniel's message: "please separate between sophia as just a store and sophialang as the programming language of the store").** Related: SR-25, [[SPEC-001 Sophia Store v0]], [[SPEC-002 Sophialang Principles]].

## Context

The 2026-10-17 draft mixed the store and its language in one specification. Daniel wants every programming language to live in the store eventually, with sophialang as one of them.

## Decision

- The store knows only **languages**, each of which registers a normaliser that produces canonical bytes. It handles framing, components, names, nominal entities, lineages, annotations, facts and queries for all of them alike.
- Sophialang is the first language, specified separately (SPEC-002, SPEC-003).

## Consequences

- `sophia-store` has no dependency on `sophialang-core`. The dependency goes the other way, through the `Language` trait.
- Every definition row carries `lang` and `normaliser`.
- Moonkale's WP-66 normal forms for Rust, Julia and Python could be stored in the same tables later (SR-74).
