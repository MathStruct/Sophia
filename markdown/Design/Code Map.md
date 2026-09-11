# Code Map

Every source file in the repository has a markdown sibling of the same name describing what it is for. Because the vault root is the repository root, those siblings are ordinary vault notes and link into the rest of the design. This note is the index of them.

**All code files are comments only.** Nothing is implemented, nothing is meant to run. See [[Repository Layout]] for the tree and the Rust/Julia split, and [[Roadmap]] for the order in which these would actually be written.

## Rust — `crates/`

| Crate | File | Note | Role |
| --- | --- | --- | --- |
| `sophia-hash` | `sophia_hash.rs` | [[sophia_hash]] | public hashing API, the three-hash scheme |
| | `canonical.rs` | [[canonical]] | term → canonical form. **The most dangerous file here.** |
| | `merkle.rs` | [[merkle]] | Merkle hashing, SCC construction, erasure |
| `sophia-core` | `sophia_core.rs` | [[sophia_core]] | the SC term language, kernel, `Prf`/`Cmp` fragments |
| | `elaborate.rs` | [[elaborate]] | surface → core, NbE, the `Frontend` trait |
| | `effects.rs` | [[effects]] | effect rows, regions, borrows |
| `sophia-store` | `sophia_store.rs` | [[sophia_store]] | backend trait, the IR-persistence measurement |
| | `schema.rs` | [[schema]] | node/edge kinds, SQL mapping, migrations |
| `sophia-equiv` | `sophia_equiv.rs` | [[sophia_equiv]] | claims, levels, modulo, substitution policy |
| | `egraph.rs` | [[egraph]] | saturation and extraction over `egg` |
| | `witness.rs` | [[witness]] | the six witness formats and their checkers |
| `sophia-emit` | `sophia_emit.rs` | [[sophia_emit]] | lowering to MLIR and LLVM |
| `sophia-frontend-metamath` | `metamath.rs` | [[metamath]] | the M0 frontend |
| `sophia-cli` | `main.rs` | [[main]] | the `sophia` command-line driver |

## Julia — `src/`

| File | Note | Role |
| --- | --- | --- |
| `Sophia.jl` | [[Sophia]] | package entry point; why there is a Julia half |
| `CoreIR.jl` | [[CoreIR]] | mirror of the core term language, for differential testing |
| `Hashing.jl` | [[Hashing]] | hashing plus the Rust conformance suite |
| `Store.jl` | [[Store]] | thin client; the `context` query |
| `Frontend.jl` | [[Frontend]] | Julia lowered/typed IR → core terms |
| `Annotations.jl` | [[Annotations]] | `@equiv`, `@spec`, `@sophia_test`, `@assume` |
| `Precompile.jl` | [[Precompile]] | the content-addressed code cache |

## Reading order for someone new

1. [[Design Overview]] — what the system is
2. [[sophia_hash]] → [[canonical]] → [[merkle]] — how identity works, and what can go wrong with it
3. [[sophia_core]] → [[effects]] — the common language
4. [[schema]] → [[sophia_store]] — how it is stored and what has to be measured
5. [[sophia_equiv]] → [[witness]] — the central claim and its safeguards
6. [[Precompile]] — the part that could pay for itself first

## Where the risk is concentrated

Three files, for three different reasons:

- [[canonical]] — the only silent failure mode in the system. An over-coarse canonical form identifies two different programs and nothing downstream detects it.
- [[sophia_store]] — carries the empirical question that could invalidate the architecture: whether querying IR back out is competitive with regenerating it ([[Open Problems and Risks]]).
- [[sophia_equiv]] — where a careless default turns the project into a miscompilation generator, via the congruence problem in [[Equivalence and Witnesses]].

## Related

- [[Repository Layout]] · [[Design Overview]] · [[Roadmap]]
- [[Tests and Documentation as Nodes]] — these paired files are a manual, filesystem-level version of the `DOCUMENTS` edge
