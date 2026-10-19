#reference

> Recorded benchmark runs, newest first. Each entry gives the date, the commit, the machine (CPU, RAM, OS, storage), the command, and the table produced by `sophia-bench-report`. A note follows each entry with what the numbers mean for the hypotheses of the [[Benchmark Plan]]: confirmed, refuted, or unclear.

## 2026-10-20: first store benchmarks (synthetic code)

- **Commit:** the commit that adds this page; the code is the M0/M1 implementation.
- **Machine:** AMD Ryzen 7 7800X3D (8 cores), 30 GiB RAM, Linux 7.2.9 (Arch), NVMe SSD. Release build, nothing else running.
- **Command:** `SOPHIA_BENCH_DATE=2026-10-20 cargo run --release -p sophia-bench --bin sophia-bench-report -- --sizes 1000,10000,100000`
- **Workload:** synthetic sophialang from `sophia_bench::generate`, seed 42. Term nodes per definition are log-normal (median 60, σ 0.9); references are Poisson (mean 3); 1 % of definitions are in recursive pairs. Ingest is in batches of 1000, with a name for each definition. Lookups are 1000 random probes, median of 5 runs, reported per lookup.

### B-07 / B-05: canonical sizes (10 000 synthetic definitions)

| median bytes | p90 bytes | mean bytes | distinct refs per def | reference digests in canonical bytes | key overhead (32 B / mean) | savings with 8-byte local ids |
|---|---|---|---|---|---|---|
| 2588 | 8573 | 3816 | 2.99 | 2.5 % of bytes | 0.8 % | 1.9 % of bytes |

### B-03 / B-04 / B-05: backends (after the SQLite fix described below)

| backend | definitions | ingest | ingest rate | size on disk | bytes / def | record by id | name | dependants | closure of last (size) | full change feed |
|---|---|---|---|---|---|---|---|---|---|---|
| sqlite | 1000 | 0.06 s | 17140 defs/s | 6.6 MiB | 6889 | 1.9 µs | 0.5 µs | 0.7 µs | 0.28 ms (128) | 0.6 ms |
| redb | 1000 | 0.05 s | 20854 defs/s | 16.1 MiB | 16847 | 0.9 µs | 0.4 µs | 0.7 µs | 0.13 ms (128) | 0.6 ms |
| sqlite | 10000 | 0.68 s | 14680 defs/s | 64.6 MiB | 6776 | 3.2 µs | 0.6 µs | 0.9 µs | 2.7 ms (901) | 5.8 ms |
| redb | 10000 | 0.57 s | 17650 defs/s | 128.5 MiB | 13475 | 1.2 µs | 0.5 µs | 0.8 µs | 1.2 ms (901) | 5.7 ms |
| sqlite | 100000 | 10.90 s | 9177 defs/s | 646.5 MiB | 6779 | 4.5 µs | 1.3 µs | 2.2 µs | 22.8 ms (5713) | 70.9 ms |
| redb | 100000 | 8.05 s | 12423 defs/s | 1028.0 MiB | 10779 | 1.4 µs | 0.6 µs | 1.4 µs | 7.5 ms (5713) | 73.8 ms |

**Before the fix**, with SQLite `t_definition` declared `WITHOUT ROWID` and three queries per record, SQLite at 100 000 definitions measured:
- record by id: 18.5 µs;
- closure: 107 ms;
- ingest: 6821 definitions/s;
- 7781 bytes per definition.

See [[sqlite]] for the cause.

### What this says

- **H2 (hashing is irrelevant next to everything else): consistent.** Ingest, which normalises, hashes and writes, runs at 9 000–21 000 definitions/s, i.e. 50–110 µs per definition including the database write. B-01 and B-02 (criterion) will separate hashing from writing.
- **H3 (local ids save ≥ 30 %): refuted on this corpus.** Reference digests are only 2.5 % of canonical bytes, and local ids would save 1.9 %. The reason is that the **framing** dominates: about 40 bytes per term node, because each tag-length-value field carries 8 bytes of lengths plus its tag, and built-in references (`Nominal`) are 32 bytes each. Block Size §2 assumed 8–16 bytes per node. The lever that matters is therefore **B-06 (a compact storage encoding)**, not local ids. Caveat: synthetic definitions are large (median 2.6 KB), and real code may have more references per byte.
- **H1 (median definition 200 B – 4 KiB):** the synthetic median of 2.6 KB lies inside the range, but that only says what the generator assumed. Real code is still needed.
- **Backends:**
  - redb reads 2–3× faster (by id, closure) and ingests 20–35 % faster;
  - SQLite uses 35–60 % less disk, has SQL views, and is what Moonkale already ships;
  - name and dependants lookups are similar;
  - the change feed costs the same (~0.7 µs per entry).

  This does not yet decide anything: DuckDB and LMDB are still to come (M2), and redb's file size has to be re-measured after `compact()`.
- **Scaling:** lookups grow slowly with size (B-tree depth plus the cache). Closure walks are linear in the closure's size. At 10⁵ definitions, every backend answers point queries in a few microseconds, which is fast enough for an interactive editor by two orders of magnitude.

