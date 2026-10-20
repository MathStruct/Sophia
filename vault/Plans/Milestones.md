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
| `sophia-query` | Sophia's own query language: IR, Julia-flavoured syntax, canonical ids, native executor, magic sets, SQL translation ([[SPEC-007 Sophia Query Language]]) | M3a |
| `sophia-mcp` | the tool server for agents ([[LLMs and Mixed-Language Code]]) | M3b |
| `sophialang-eval`, `-check` | interpreter, simple checker, dispatch resolution, test runner with caching | M4 |
| `sophia-build` | lowering to LLVM, minimal binaries | M5 |

The old comment-only crates (`sophia-core`, `sophia-equiv`, `sophia-emit`, `sophia-frontend-metamath`) leave the workspace. Their notes stay in the vault as ideas.

## Milestones

| | Milestone | Done when | Benchmarks |
|---|---|---|---|
| **M0** | **Identity.** `sophia-hash` + `sophialang-core`: terms, normaliser, components, digests | property tests: α-invariance, determinism, a dependency change gives a new id, recursive groups, holes and unresolved names, nominal references | B-01 hash throughput, B-02 encode and hash (flat vs boxed) |
| **M1** | **Store.** `Backend` trait; SQLite and redb backends; names, log, lineage, nominals and method tables, annotations, facts; the conformance suite on both; `sophia` CLI (`init`, `ls`, `show --core`, `log`, `sql`, `gen`) | SPEC-001 §13 on both backends | B-03 bulk ingest, B-04 point lookups and closure walks, B-05 store size |
| **M2** | **Moonkale interface (WP-68).** `sophialang-syntax` with the `julia` profile (and `rust`, `python`); rendering profiles in the database; `sophia-interface`; the sample store; DuckDB and LMDB backends | SPEC-004 §9 on Sophia's side (an in-repo mock of Moonkale's `Source` calls); the Moonkale agent builds `sources/sophia` | B-06 storage encodings, B-10 print and parse latency |
| **M3a** | **Sophia's own query language** (S-ADR-0011, SPEC-007). Storage-independent IR (stratified Datalog), Julia-flavoured syntax, canonical ids, queries stored as definitions, native executor on every backend, magic sets, SQL translation, the `query` dialect for Moonkale | the same query gives the same rows plain and magic, on every backend and through SQL; stored queries print, edit and verify through the interface | B-17 query workloads; zero-copy breakdown |
| **M3b** | **Tools.** `sophia-mcp` exposing the query API and edits; tag and attribute queries | an agent edits sophialang and runs stored queries through MCP | B-11 query latency by dialect and backend |
| **M4** | **Running code.** Interpreter over core terms; dispatch resolution as facts; tests as facts with caching; simple type checker | SR-54: a test re-runs only when its recorded inputs change; a new method invalidates exactly the overlapping facts | B-09 invalidation precision, B-12 interpreter speed |
| **M5** | **Compiling.** Lowering to LLVM IR for the explicit fragment; minimal binaries; a JIT cache keyed by content id and argument types | `hello` and a numeric kernel compile to standalone binaries with no Sophia runtime; performance within 1.2× of the C equivalent | B-13 compiled code against C, Rust and Julia |
| **M6** | **Self-hosting starts.** The printer for one profile, written in sophialang and run by the interpreter; the `compile` DSL (type stability); the dependent checker for the core | the sophialang printer reproduces the Rust printer's output on the sample store | — |
| **M7** | **Other languages and research.** Moonkale's WP-66 normal forms for Rust, Julia and Python ingested; abstraction mining ([[Compression and Canonical Form]]); a GC and scheduler runtime; memory and concurrency DSLs; a Lisp or Metamath frontend if useful | each one is an experiment with a written result, whatever it is (SR-73) | B-07 corpus statistics and deduplication, B-08 compression |

## The P0 gate (SR-89, from the external review)

Foundational invariants (identity, worlds, editable projections) count as settled only once three demonstrations pass:

| # | Demonstration | Status |
|---|---|---|
| 1 | **Symmetric recursion:** two symmetric mutually recursive definitions normalise, store, reload and verify without loss, on both backends, stable under renaming and permutation | **passes** (`crates/sophia-store/tests/symmetric.rs`, after the quotient fix) |
| 2 | **Two-view editing:** two clients cannot silently overwrite each other or an unintended alias | **passes**: on Sophia's side (`group_members_cannot_be_overwritten_silently`), and through Moonkale's `Source` with two clients in two profiles (Moonkale `two_clients_in_two_profiles_cannot_overwrite_each_other`, `46a06b65` on its dev branch) |
| 3 | **Reproducible dispatch:** a method table changes, and an earlier world snapshot still reproduces the original | **passes at the store level** (`worlds_are_reproducible`). End-to-end dispatch needs the interpreter (M4) |

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

- **2026-10-20: M2 done.**
  - LMDB (heed) and DuckDB backends; redb and LMDB now share `kv.rs`.
  - The four-backend comparison is recorded, with S-ADR-0010 proposed.
  - The P0 gate's demonstration 2 passes through Moonkale's `Source` (Moonkale `46a06b65`, on its dev branch).

- **2026-10-20: M3a done in its first form** (Daniel's query-language proposal, forwarded with an external review):
  - `sophia-query`; the `query` dialect in `sophia-interface`; `sophia query` in the CLI;
  - the execution suite on all four backends, and the interface acceptance test;
  - B-17 and the zero-copy breakdown in [[Benchmark Results]];
  - S-ADR-0010 accepted in amended form; S-ADR-0011 accepted; S-ADR-0012 proposed.

- **2026-10-20: M3a stabilised** after its external review ([[Review 2026-10-20 M3a (external)]]):
  - the cache race and the cache's row-cap behaviour are fixed;
  - an exact canonical form;
  - a byte budget, deadline and cancellation;
  - property tests for canonical invariance, and differential tests of magic sets and SQL (one guard bug found and fixed);
  - SPEC-006 §6a, the transformation-evidence interface.

- **2026-10-20: B-16 rung 1, Metamath:** `sophia-ingest`, set.mm and iset.mm stored and queried ([[Real Code Corpora]] §4). On the way the store gained:
  - annotations by id;
  - membership lookups by member;
  - batch members and annotations;
  - planner selectivity classes;
  - magic sets for negated calls.

- **2026-10-20: B-16 rung 2, Julia LinearAlgebra** through the shared normaliser v6, identical ids to Moonkale ([[Real Code Corpora]] §5).

- **2026-10-20: real-code review** ([[Review 2026-10-20 Real Code (external)]]):
  - Metamath dependency analysis checked against metamath.exe;
  - generic functions keyed by (module, name), checked against Julia's method table (53.7 % → 94.5 %);
  - record kinds enforced (SP-012);
  - new vault sections [[Storage and Materialization]] and [[Languages]];
  - the experiment order replaced by [[Semantic Boundary Experiments]] (SR-124).

**Order from here (SR-113, revised by SR-124 to SR-126):**
1. **Semantic boundaries:** [[Semantic Boundary Experiments]] E-0 to E-7 (trust records, Julia conformance, cross-language structure, dispatch facts, macros, AD, Reactant), each small, with an oracle. Full Lux.jl and Lenticulum.jl after.
2. **B-15:** Moonkale's edit loop, once Moonkale moves to the current revision (SR-125: a priority, in parallel).
3. **The set-oriented native executor:** a prefix access path, anti-joins for negation over scans, borrowed field reads.
4. **M3b:** the agent server, with a hard memory bound for untrusted queries (SR-108).

## Order of work inside M0 to M2

1. Workspace cleanup; `sophia-hash`: framing, `Digest`, component ordering, with tests.
2. `sophialang-core`: `Term`, `Definition`, `Kind`; the normaliser; flat layout; tests; B-01 and B-02.
3. `sophia-store`: `Backend` trait, the logic, the conformance suite; SQLite; redb; B-03 to B-05; first [[Benchmark Results]].
4. CLI and a generator of synthetic stores (realistic size distributions).
5. `sophialang-syntax` with the `julia` profile, then the round-trip property.
6. Profiles in the database; `sophia-interface`; the sample store; DuckDB and LMDB; [[Database Comparison]] filled in.
