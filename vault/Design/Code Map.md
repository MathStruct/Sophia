#overview #implementation

> Sources: index of the per-file design notes

Every source file in the repository has a markdown sibling of the same name describing what it is for. Because the vault root is the repository root, those siblings are ordinary vault notes and link into the rest of the design. This note is the index of them.

**All code files are comments only.** Nothing is implemented, nothing is meant to run. See [[Repository Layout]] for the tree and the Rust/Julia split, and [[Roadmap]] for the order in which these would actually be written.

## Rust — `crates/` (implemented, 2026-10-20)

| Crate | File | Note | Role |
| --- | --- | --- | --- |
| `sophia-hash` | `sophia_hash.rs` | [[sophia_hash]] | identity: framing, digests, components; language-agnostic |
| | `canonical.rs` | [[canonical]] | the tag-length-value framing shared with Moonkale |
| | `component.rs` | [[component]] | SCCs and the canonical order of recursive groups |
| | `merkle.rs` | [[merkle]] | component, member and nominal digests; `Record` |
| | `digest.rs` | [[digest]] | the 256-bit digest type |
| `sophialang-core` | `sophialang_core.rs` | [[sophialang_core]] | sophialang's core calculus and normaliser |
| | `term.rs` | [[term]] | core terms |
| | `definition.rs` | [[definition]] | definition kinds and bodies |
| | `encode.rs` | [[encode]] | canonical encoding, decoding, display data |
| | `normalise.rs` | [[normalise]] | batches of drafts → records |
| | `samples.rs` | [[samples]] | the 16-definition sample program |
| `sophia-store` | `sophia_store.rs` | [[sophia_store]] | the store over interchangeable backends |
| | `backend.rs` | [[backend]] | the backend traits |
| | `logic.rs` | [[logic]] | the store's rules |
| | `types.rs` | [[types]] | names, log, nominal entities, lineages, annotations, facts |
| | `codec.rs`, `query.rs`, `error.rs` | [[codec]], [[query]], [[error]] | value encoding, query results, errors |
| | `sqlite.rs` | [[sqlite]] | SQLite backend with the read-only guard |
| | `redb_store.rs` | [[redb_store]] | redb backend |
| `sophia-bench` | `sophia_bench.rs`, `bin/sophia-bench-report.rs` | [[sophia_bench]], [[sophia-bench-report]] | the benchmark suite |
| `sophia-cli` | `main.rs` | [[main]] | the `sophia` command |

The comment-only crates of the first vault (`sophia-core`, `sophia-equiv`, `sophia-emit`, `sophia-frontend-metamath`) are kept as ideas in `attic/`: [[sophia_core]], [[elaborate]], [[effects]], [[sophia_equiv]], [[egraph]], [[witness]], [[sophia_emit]], [[metamath]].

## Julia — `src/` (comment-only; not part of the plan since S-ADR-0004, Rust only)

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

1. [[Status]] and [[SPEC-001 Sophia Store v0]]: what the system is now
2. [[sophia_hash]] → [[canonical]] → [[component]] → [[merkle]]: how identity works, and what can go wrong with it
3. [[sophialang_core]] → [[term]] → [[encode]] → [[normalise]]: the first language
4. [[sophia_store]] → [[backend]] → [[logic]] → [[sqlite]] / [[redb_store]]: how it is stored, and the measurements ([[Benchmark Results]])

## Where the risk is concentrated

Three files, for three different reasons:

- [[canonical]] and [[encode]] — the only silent failure mode in the system. An over-coarse canonical form identifies two different programs and nothing downstream detects it.
- [[sophia_store]] — carries the empirical questions of [[Database Comparison]] and that could invalidate the architecture: whether querying IR back out is competitive with regenerating it ([[Open Problems and Risks]]).
- [[sophia_equiv]] — where a careless default turns the project into a miscompilation generator, via the congruence problem in [[Equivalence and Witnesses]].

## Related

- [[Repository Layout]] · [[Design Overview]] · [[Roadmap]]
- [[Tests and Documentation as Nodes]] — these paired files are a manual, filesystem-level version of the `DOCUMENTS` edge
