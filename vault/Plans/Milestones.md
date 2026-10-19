#implementation #design

> The plan, as milestones. Each one ends with something that runs, a set of passing tests and a recorded benchmark run. It follows Daniel's priorities of 2026-10-20: **the Moonkale interface first** (SR-63), then the language and individual languages; **several databases, compared** (SR-64); **sophialang before Metamath or a Lisp** (SR-65); **Rust only** (S-ADR-0004); an **extensive benchmark suite** (SR-72). It replaces the "Milestones" of 2026-10-17. The old Roadmap's M0 to M7 ([[Roadmap]]) are superseded; see [[Reading of the Earlier Vault]].
>
> Sources: original to this vault. code: `crates/`.

## Crates

| Crate | Job | Milestone |
|---|---|---|
| `sophia-hash` | canonical framing, BLAKE3 digests, recursive-component ordering (SPEC-001 §4) | M0 |
| `sophialang-core` | the core terms of SPEC-003, the normaliser (`Language` implementation), flat term layout | M0 |
| `sophia-store` | the `Backend` trait, the store logic (names, log, lineage, nominals, relations, annotations, facts), the conformance suite | M1 |
| backends: features of `sophia-store` (`sqlite`, `redb`; later `duckdb`, `lmdb`) | the backends, compared ([[Database Comparison]]) | M1 (SQLite, redb), M2 (DuckDB, LMDB) |
| `sophia-bench` | the benchmark suite: generators, criterion benches, report writer ([[Benchmark Plan]]) | M0 onwards |
| `sophialang-syntax` | printer and parser per rendering profile, source maps, diagnostics (SPEC-005) | M2 |
| `sophia-interface` | the Moonkale-facing layer: node and edge mapping, operation mapping, profile resolution, query guard (SPEC-004) | M2 |
| `sophia-cli` | the `sophia` command | M1 onwards |
| `sophia-mcp` | the tool server for agents ([[LLMs and Mixed-Language Code]]) | M3 |
| `sophialang-eval`, `-check` | interpreter, simple checker, dispatch resolution, test runner with caching | M4 |
| `sophia-build` | lowering to LLVM, minimal binaries | M5 |

The old comment-only crates (`sophia-core`, `sophia-equiv`, `sophia-emit`, `sophia-frontend-metamath`) leave the workspace. Their notes stay in the vault as ideas.

## Milestones

| | Milestone | Done when | Benchmarks |
|---|---|---|---|
| **M0** | **Identity.** `sophia-hash` + `sophialang-core`: terms, normaliser, components, digests | property tests: α-invariance, determinism, a dependency change gives a new id, recursive groups, holes and unresolved names, nominal references | B-01 hash throughput, B-02 encode and hash (flat vs boxed) |
| **M1** | **Store.** `Backend` trait; SQLite and redb backends; names, log, lineage, nominals and method tables, annotations, facts; the conformance suite on both; `sophia` CLI (`init`, `ls`, `show --core`, `log`, `sql`, `gen`) | SPEC-001 §13 on both backends | B-03 bulk ingest, B-04 point lookups and closure walks, B-05 store size |
| **M2** | **Moonkale interface (WP-68).** `sophialang-syntax` with the `julia` profile (and `rust`, `python`); rendering profiles in the database; `sophia-interface`; the sample store; DuckDB and LMDB backends | SPEC-004 §9 on Sophia's side (an in-repo mock of Moonkale's `Source` calls); the Moonkale agent builds `sources/sophia` | B-06 storage encodings, B-10 print and parse latency |
| **M3** | **Tools.** `sophia-mcp`; the `datalog` dialect; tag and attribute queries | an agent edits sophialang through MCP; Datalog transitive dependants match SQL | B-11 query latency by dialect and backend |
| **M4** | **Running code.** Interpreter over core terms; dispatch resolution as facts; tests as facts with caching; simple type checker | SR-54: a test re-runs only when its recorded inputs change; a new method invalidates exactly the overlapping facts | B-09 invalidation precision, B-12 interpreter speed |
| **M5** | **Compiling.** Lowering to LLVM IR for the explicit fragment; minimal binaries; a JIT cache keyed by content id and argument types | `hello` and a numeric kernel compile to standalone binaries with no Sophia runtime; performance within 1.2× of the C equivalent | B-13 compiled code against C, Rust and Julia |
| **M6** | **Self-hosting starts.** The printer for one profile, written in sophialang and run by the interpreter; the `compile` DSL (type stability); the dependent checker for the core | the sophialang printer reproduces the Rust printer's output on the sample store | — |
| **M7** | **Other languages and research.** Moonkale's WP-66 normal forms for Rust, Julia and Python ingested; abstraction mining ([[Compression and Canonical Form]]); a GC and scheduler runtime; memory and concurrency DSLs; a Lisp or Metamath frontend if useful | each one is an experiment with a written result, whatever it is (SR-73) | B-07 corpus statistics and deduplication, B-08 compression |

## Progress

- **2026-10-20: M0 done.**
  - `sophia-hash` and `sophialang-core`: 27 tests, including property tests.
  - B-01 and B-02 exist as criterion benchmarks. The flat-layout comparison of B-02 waits until a flat layout exists.
- **2026-10-20: M1 done, except DuckDB and LMDB, which move to M2.**
  - `sophia-store` with SQLite and redb: 14 conformance tests per backend, plus the read-only SQL guard test.
  - The `sophia` CLI.
  - `sophia-bench-report`, with its first run in [[Benchmark Results]].
  - Two findings: the SQLite layout fix (4× faster lookups), and H3 refuted (the framing dominates, so a compact encoding matters more than local ids).

- **2026-10-20: M2's Moonkale interface done.** DuckDB and LMDB are still open in M2.
  - `sophialang-syntax`: the `julia` and `rust` profiles round-trip (5 tests, and 20 000 random definitions).
  - Rendering profiles are stored in the database.
  - `sophia-interface`: SPEC-004 §9 on both backends (11 tests).
  - The CLI's `text`, `edit`, `add` and `profile` commands.
  - Relations are nominal (S-ADR-0009).
  - `Match` carries its data type.

## Order of work inside M0 to M2

1. Workspace cleanup; `sophia-hash`: framing, `Digest`, component ordering, with tests.
2. `sophialang-core`: `Term`, `Definition`, `Kind`; the normaliser; flat layout; tests; B-01 and B-02.
3. `sophia-store`: `Backend` trait, the logic, the conformance suite; SQLite; redb; B-03 to B-05; first [[Benchmark Results]].
4. CLI and a generator of synthetic stores (realistic size distributions).
5. `sophialang-syntax` with the `julia` profile, then the round-trip property.
6. Profiles in the database; `sophia-interface`; the sample store; DuckDB and LMDB; [[Database Comparison]] filled in.
