#implementation

> Runs the store benchmarks at sizes from 10³ to 10⁵ (or more) on every backend, and prints the Markdown that goes into [[Benchmark Results]]. Source: `sophia-bench-report.rs`.
>
> Sources: [[Benchmark Plan]] B-03, B-04, B-05, B-07. code: `sophia-bench-report.rs`

```sh
SOPHIA_BENCH_DATE=$(date -I) cargo run --release -p sophia-bench --bin sophia-bench-report -- --sizes 1000,10000,100000
```

It reports:
- the machine;
- canonical size statistics: median, p90, mean, references per definition, and the share of reference digests in the bytes, with the saving that 8-byte local ids would give;
- per backend and size: ingest time and rate, size on disk, bytes per definition, the median latency of a record lookup by id, a name lookup and a dependants lookup (1000 random probes), the closure of the last definition, and a full read of the change feed.

**B-17 and zero-copy** (2026-10-20):
- `--query-size N` sets the size for the query workloads (B-17); `--skip-queries` and `--only-queries` select them.
- `--zero-copy` adds the LMDB read breakdown.
- Bare flags go through `flag()`. Before that fix, `arg()` expected a value after every flag, so `--skip-analytics` never worked.
