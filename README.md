# Sophia

**Under development — design stage. No code here runs yet.**

A content-addressed graph database of code, its types, its proofs and all of its intermediate representations, with a compiler expressed as queries over that database.

Every declaration, expression, MLIR operation, LLVM instruction, test, doc comment and proof becomes a node keyed by a hash of its own normalised structure. Frontends elaborate source languages into one common core calculus; backends lower core terms through MLIR into LLVM IR. Code from different languages is meant to interoperate not through an FFI or a shared ABI, but by storing *equivalence edges* that carry machine-checkable evidence.

## Documentation

This repository is also an Obsidian vault (the vault root is the repository root, so every `.md` file in it is a note and `[[wikilinks]]` resolve across code and prose alike).

- **[markdown/Start Here.md](markdown/Start%20Here.md)** — the original statement of the idea
- **[markdown/Design/](markdown/Design/)** — what it becomes once made precise: core calculus, hashing scheme, graph schema, equivalence ladder, roadmap, and an honest risk list. Start at `Design Overview.md`.
- **[markdown/Wiki/](markdown/Wiki/)** — background concept cards (LLVM, MLIR, Unison, hashing, rewriting, semantics, type theory, category theory, Julia internals)
- **[markdown/State of the Art/](markdown/State%20of%20the%20Art/)** — survey of prior art, and a table of what exists versus what does not
- **`Code Map.md`** in `markdown/Design/` — index of the per-file design notes

Formulas are Typst (`wypst`), diagrams are TikZ (`inline-tikz`).

## Layout

```
Cargo.toml          Rust workspace manifest
Project.toml        Julia package manifest
crates/             Rust: hashing, core calculus, store, equivalence, emission, CLI
src/                Julia: frontend, annotation macros, precompilation experiment
markdown/           the vault
```

Julia requires `src/Sophia.jl`, so `src/` belongs to Julia and Rust lives in `crates/`. Every `.rs` and `.jl` file has a `.md` sibling of the same name explaining what it is meant to do. See `markdown/Design/Repository Layout.md`.

**All source files currently contain comments only.** They exist to fix module boundaries while the design settles.

## Roadmap in brief

| | Milestone | Tests the assumption |
| --- | --- | --- |
| M0 | Metamath ingestion, hashing, schema | canonicalisation is deterministic and total |
| M1 | a tiny Lisp, end to end | binders, elaboration, frontend functoriality |
| M2 | emit MLIR → LLVM and run it | querying IR out of the store is fast enough |
| M3 | Julia ingestion + content-addressed code cache | invalidation precision beats Julia's current scheme |
| M4 | equivalence within one language (e-graph) | a rewrite rule set over the core is useful |
| M5 | the proof fragment; specs attached to Julia code | "Julia + Lean", in the tractable form |
| M6 | one witnessed Julia ↔ C++ vertical slice | a cross-language relation can be written at all |
| M7 | unified execution | — |

Full version with rationale: `markdown/Design/Roadmap.md`.

Note the ordering: **M3 is where the project justifies itself**, and it needs none of the cross-language machinery. Even total failure at M6 leaves a content-addressed, queryable code store with exact dependency tracking and exact test and compilation invalidation.

## Databases under evaluation

Graph: LadybugDB, HelixDB, TypeDB, FalkorDB. Relational: DuckDB, TursoDB.

Current leaning is DuckDB first — the schema is two tables wide and the hot queries are hash lookups and bounded traversals, which a columnar engine handles well without a server. The store sits behind a trait so the choice stays reversible; see `markdown/State of the Art/State of the Art - Graph Databases for Code.md`.

## On disagreeing with any of this

The store is content-addressed, so it is not trusted: any node can be re-verified by rehashing it. And migration is re-elaboration from core terms rather than a text rewrite. So if you dislike the schema, replace `crates/sophia-store/src/schema.rs` and migrate — that is a supported operation, not a fork.

## Prior art

The closest existing system is [Unison](https://www.unison-lang.org/), which content-addresses code by the hash of its syntax tree. Sophia borrows that and adds persisted IR layers, a multi-language frontend story, and equivalence edges carrying evidence. What is genuinely new — and genuinely risky — is the last of those; see `markdown/Design/Open Problems and Risks.md`.
