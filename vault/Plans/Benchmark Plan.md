#implementation #reference

> The benchmark suite (SR-72) and how its results are recorded. Every hypothesis in the design notes that depends on performance has a benchmark here, and every benchmark has a hypothesis. Results go to [[Benchmark Results]], with the machine, the commit, the date and the exact command. A result that contradicts a design note is recorded as such, and the note is corrected (SR-73).
>
> Sources: criterion.rs; Mytkowicz et al., *Producing Wrong Data Without Doing Anything Obviously Wrong!*, ASPLOS 2009 (measurement bias); Georges, Buytaert and Eeckhout, *Statistically Rigorous Java Performance Evaluation*, OOPSLA 2007. code: `crates/sophia-bench` (benches in `benches/`, generators in `src/`, the report writer in `src/bin/sophia-bench-report.rs`).

## Principles

- **Reproducible inputs.** Synthetic stores are generated from a seed with realistic distributions: definition sizes log-normal, references per definition Poisson, recursive groups rare (about 1 %), name depth 2–5. Real corpora (sophialang samples, and later ingested Julia and Rust) are pinned by commit.
- **Statistics, not anecdotes.** criterion's confidence intervals; at least 30 samples; outliers reported, not hidden; machine noise controlled (performance governor, no other heavy work), and the conditions written down.
- **The same suite on every backend.** Backend benchmarks are generic over the `Backend` trait.
- **Sizes from 10³ to 10⁶ definitions** for store benchmarks. 10⁷ when a backend gets that far.
- **Real code, not only synthetic** (SR-83): see B-16.
- **Distributions and memory:** latencies are reported as p50, p90 and p99 with cold and warm caches, together with allocations and resident memory, not only medians.
- **Scripted.** `cargo bench -p sophia-bench` runs everything. `cargo run -p sophia-bench --bin sophia-bench-report` writes a Markdown table that is pasted into [[Benchmark Results]].

## The benchmarks

| # | What | Hypothesis tested | From |
|---|---|---|---|
| B-01 | BLAKE3 throughput by input size (32 B – 64 KiB); our framing's overhead | H2: hashing a definition is < 1 µs | [[Block Size and Hash Overhead]] |
| B-02 | normalise + encode + hash a definition: flat layout vs boxed tree | H5: flat ≥ 3× faster | Block Size §3 |
| B-03 | bulk ingest of N definitions (put component, names, log) per backend | which backend ingests fastest; scaling with N | [[Database Comparison]] |
| B-04 | point lookup by id and by name; dependants and dependencies (one hop); transitive closure from a root | read latency per backend; closure cost | Database Comparison |
| B-05 | store size on disk per backend, with digests vs local ids for references | H3: local ids save ≥ 30 % | Block Size §2 |
| B-06 | storage encodings: canonical TLV vs compact varint, with conversion cost | H6 | Block Size §2 |
| B-07 | corpus statistics: definition sizes, references, dedup ratio by subterm threshold T | H1, H4 | Block Size §5, [[Compression and Canonical Form]] |
| B-08 | zstd with and without a trained dictionary on stored bytes | the size and latency trade-off | Compression §4 |
| B-09 | invalidation precision: facts made stale by adding a method, against the facts actually affected | SP-006's proposal is precise enough | [[SP-006 Open method tables versus content addressing]] |
| B-10 | print and parse latency per profile; a round trip per definition | interactive editing is < 5 ms per definition | [[SPEC-005 Rendering Profiles]] |
| B-11 | query latency: SQL views vs Datalog vs the backend's own dialect | — | SPEC-001 §11 |
| B-12 | interpreter speed on small kernels (fib, sum, nbody) | — | M4 |
| B-13 | compiled code against C, Rust and Julia on the same kernels | SR-52: within 1.2× | M5 |
| B-14 | GC pause times and throughput; marking on a separate core | SR-56 | M7 |
| B-15 | **edit-to-updated-graph latency** through `sophia-interface`: fetch, edit, parse, normalise, store, the change feed, and Moonkale's refresh (measured on Moonkale's side). Cold and warm; p50/p90/p99 | Moonkale stays interactive: < 50 ms p99 for one definition on 10⁵ stores | review §7 |

**B-15 measured (2026-10-20, late night):** p99 of the whole loop 2.4–3.5 ms on 10⁵ definitions, target < 50 ms; the change feed's 500 ms polling dominates what a user would see ([[Benchmark Results]]). The graph-panel and paint parts below are still open.

**B-15 measurement plan (agreed with the Moonkale agent, 2026-10-20).** Moonkale's graph panel refreshes on a `graph_epoch` bump after its own saves, not through the change feed. So the measured path is: `apply()` returns → the epoch bump → the renderer payload is delivered, timed with tracing timestamps, p50/p99 over about 200 saves. Paint time on the agent's headless sway with Mesa software GL is CPU-bound and is reported separately, marked "software GL, not hardware". The real paint number comes from Daniel's desktop.
| B-16 | real corpora: a substantial Julia package, a Rust workspace, a theorem library (Metamath set.mm), and a code base with many tiny definitions. These need frontends or Moonkale's WP-66 normal forms | H1, H3, H4 on real code | review §7 |
| B-17 | **query workloads** in Sophia's query language (SPEC-007) on every backend: direct and transitive dependants, transitive dependencies, count by kind, unreferenced definitions, names by prefix; native executor with and without magic sets; on SQLite also the SQL translation. Plus the zero-copy breakdown of an LMDB record read (SR-101) | S-ADR-0010's re-decision; SR-96 (backend independence costs little); magic sets matter | Daniel's query proposal |
