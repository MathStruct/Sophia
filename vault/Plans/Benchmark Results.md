#reference

> Recorded benchmark runs, newest first. Each entry gives the date, the commit, the machine (CPU, RAM, OS, storage), the command, and the table produced by `sophia-bench-report`. A note follows each entry with what the numbers mean for the hypotheses of the [[Benchmark Plan]]: confirmed, refuted, or unclear.

## 2026-10-20 (later): four backends (SQLite, redb, LMDB, DuckDB)

- **Machine and workload:** as below (Ryzen 7 7800X3D, synthetic corpus, seed 42, batches of 1000).
- **Commands:**
  - `cargo run --release -p sophia-bench --bin sophia-bench-report -- --sizes 1000,10000`
  - `… -- --sizes 100000 --backends sqlite,redb,lmdb --skip-analytics`
  - `… -- --sizes 10000 --backends sqlite,duckdb --analytics-size 10000`
- **All four backends pass the conformance suite** (store: 66 tests; interface acceptance: 29 tests across the backends).

### B-03 / B-04 / B-05

| backend | definitions | ingest | ingest rate | size on disk | bytes / def | record by id | name | dependants | closure of last (size) | full change feed |
|---|---|---|---|---|---|---|---|---|---|---|
| sqlite | 1000 | 0.06 s | 17772 defs/s | 6.6 MiB | 6889 | 1.9 µs | 0.5 µs | 0.7 µs | 0.28 ms (128) | 0.6 ms |
| redb | 1000 | 0.05 s | 20273 defs/s | 16.1 MiB | 16847 | 1.0 µs | 0.4 µs | 0.7 µs | 0.14 ms (128) | 0.5 ms |
| lmdb | 1000 | 0.03 s | 29245 defs/s | 7.5 MiB | 7889 | 0.7 µs | 0.2 µs | 0.3 µs | 0.10 ms (128) | 0.5 ms |
| duckdb | 1000 | 9.82 s | 102 defs/s | 9.5 MiB | 9974 | 190.2 µs | 141.8 µs | 196.2 µs | 24.6 ms (128) | 1.6 ms |
| sqlite | 10000 | 0.72 s | 13916 defs/s | 64.6 MiB | 6776 | 3.2 µs | 0.6 µs | 0.9 µs | 2.9 ms (901) | 5.9 ms |
| redb | 10000 | 0.60 s | 16573 defs/s | 128.5 MiB | 13475 | 1.2 µs | 0.5 µs | 0.9 µs | 1.2 ms (901) | 6.1 ms |
| lmdb | 10000 | 0.38 s | 26425 defs/s | 95.4 MiB | 10009 | 0.7 µs | 0.2 µs | 0.4 µs | 0.74 ms (901) | 5.0 ms |
| duckdb | 10000 | 91.55 s | 109 defs/s | 132.0 MiB | 13842 | 203.5 µs | 153.2 µs | 206.8 µs | 182.7 ms (901) | 11.7 ms |
| sqlite | 100000 | 12.97 s | 7709 defs/s | 646.5 MiB | 6779 | 4.9 µs | 1.3 µs | 2.5 µs | 26.9 ms (5713) | 74.9 ms |
| redb | 100000 | 9.72 s | 10289 defs/s | 1028.0 MiB | 10779 | 1.3 µs | 0.6 µs | 1.5 µs | 9.2 ms (5713) | 91.0 ms |
| lmdb | 100000 | 5.67 s | 17648 defs/s | 816.8 MiB | 8565 | 0.9 µs | 0.3 µs | 0.7 µs | 8.0 ms (5713) | 67.6 ms |

DuckDB was not run at 10⁵: at about 100 definitions/s, ingest would take about 15 minutes, and the 10³ and 10⁴ rows already show its scaling.

### B-11: analytical queries (10 000 definitions)

| query | SQLite | DuckDB |
|---|---|---|
| count by kind | 0.29 ms | 2.11 ms |
| 10 most referenced (a join and a group) | 11.80 ms | 4.16 ms |
| mean size per status (reads every blob's length) | 89.66 ms | 4.17 ms |

### What this says

- **LMDB is the fastest at everything Moonkale does per keystroke:**
  - ingest is 1.7–2.3× SQLite's;
  - record lookup is 4–5× faster;
  - closure walks are 3–4× faster;
  - its size is between SQLite's and redb's.
- **SQLite is the smallest, queryable, and already in Moonkale.**
- **redb is fast and needs no C toolchain, but has the largest files** (no compaction run here; `Redb::compact` now exists for a fair re-measurement).
- **DuckDB is unsuitable as the primary store.** Each edit is a small transaction, and on that workload it is about 100× slower at reads and about 150× slower at ingest. On analytics it wins clearly (3–20× on joins and scans), apart from a trivial count. **Its place is an analytics replica**, fed in bulk (the Appender API, not row inserts) for the precompilation statistics of SR-66. Per-row ingest is DuckDB's worst case and is not a fair test of bulk loading.
- **H2 still holds:** normalising and hashing are not the bottleneck at these rates.

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

