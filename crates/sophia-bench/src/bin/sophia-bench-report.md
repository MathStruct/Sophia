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
