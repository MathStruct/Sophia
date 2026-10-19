#implementation

> The benchmark suite's shared parts. Source: `sophia_bench.rs`.
>
> Sources: [[Benchmark Plan]]. code: `sophia_bench.rs`, `benches/micro.rs` (criterion: B-01, B-02, B-04), [[sophia-bench-report]]

- **`Rng`**: a seedable SplitMix64.
- **`generate(seed, n, shape)`**: a synthetic sophialang program. Its **assumed** distributions are:
  - term nodes per definition: log-normal, median 60;
  - references to earlier definitions: Poisson, mean 3;
  - recursion: 1 % mutually recursive pairs.
- **`ingest`**: loads a program in even-sized batches with names, rewriting drafts across batch boundaries into references.
- **`closure`**: walks the transitive dependencies of a definition.
- **`median_time`**: the median duration over several runs.

Synthetic results say how the **store** scales, not how large **real** code is. B-07 on real code needs a frontend (M2) or ingested code (M7).
