#reference

> Recorded benchmark runs, newest first. Each entry gives the date, the commit, the machine (CPU, RAM, OS, storage), the command, and the table produced by `sophia-bench-report`. A note follows each entry with what the numbers mean for the hypotheses of the [[Benchmark Plan]]: confirmed, refuted, or unclear.

## 2026-10-20 (late night): B-15, Moonkale's edit loop through Sophia

**The first end-to-end measurement with Moonkale.** Moonkale (`26752448`, on Sophia `202e137a`) times its own edit path through the `Source` trait: resolve and fetch a definition, save an edited text with `apply`, read the name's new id, list the definitions that still reference the old id. Script: `experiments/b15-edit-loop/run.sh` ([[run]]); 200 runs per store, each alternating the original and the edited text; release build; this machine (see the B-16 section for hardware).

| Store | Edited definition | Dependants | fetch | save | new id | dependants | whole loop p50 | p90 | **p99** |
|---|---|---|---|---|---|---|---|---|---|
| SPEC-004 sample (in memory) | `sample.add` | 1 | 50 µs | 186 µs | 17 µs | 17 µs | 0.27 ms | 0.35 ms | **1.0 ms** |
| 1 000 synthetic | 8.8 KB of text | 22 | 2.0 ms | 3.4 ms | 31 µs | 164 µs | 5.6 ms | 6.1 ms | **7.0 ms** |
| 10 000 synthetic | 52 B | 119 | 71 µs | 324 µs | 35 µs | 969 µs | 1.4 ms | 1.7 ms | **3.5 ms** |
| 100 000 synthetic (3 runs, file pre-read) | 71 B | 85 | 64 µs | 310–331 µs | 29 µs | 0.96–1.05 ms | 1.3 ms | 1.5–1.6 ms | **2.4–3.5 ms** |

Step columns are medians. **The target, < 50 ms p99 for one definition on a 10⁵ store, is met by more than an order of magnitude.**

**Cold first save** (the only slow one):
- on a store file already in the OS page cache: first save 8–19 ms, first dependants query 2.2–2.6 ms (whole first loop 11–22 ms);
- on a freshly copied file (btrfs: a copy starts with a cold page cache): up to 0.65 s for the save and 0.2–1.5 s for the dependants, all of it waiting for the disk. Sophia's `Interface` called directly (`crates/sophia-interface/examples/edit_timing.rs`) shows the same pattern: 208 ms on a cold file, 2 ms once the file has been read. So the cold cost is the operating system's, not Sophia's or Moonkale's adapter.

**What this says.**
- **Store size does not matter** between 10⁴ and 10⁵ definitions; what grows the loop is the edited definition's **text length** (fetch and save parse, elaborate, normalise and print it: 8.8 KB takes 5 ms) and its **number of dependants** (about 10 µs each to list).
- **Moonkale's adapter adds little**: its steps are within a few µs to tens of µs of the direct `Interface` calls.
- **The user-visible latency is dominated by something else:** Moonkale's change feed polls every 500 ms. Edits take a millisecond, so the feed should push (SR-37 allows a push subscription), or poll much faster when idle costs allow.
- Not measured: the workspace's re-fetch after save, the change-feed path, and paint (needs Daniel's desktop), as the Moonkale agent listed.

## 2026-10-20 (night): B-16 on a real library, Metamath set.mm

- Machine: as below.
- Command: `sophia-bench-report --sizes 1000 --backends sqlite,redb,lmdb --metamath ~/.cache/sophia-corpora/set.mm/set.mm --skip-queries --only-queries`.

| backend | assertions | proofs | records | parse | ingest (parse + store) | size | queries (median of 5) |
|---|---|---|---|---|---|---|---|
| sqlite | 51159 | 48083 | 98136 | 2.25s | 15.26s | 564 MiB | axioms behind 2p2e4: 15 rows, 3.87 ms; axioms behind fta (fundamental thm of algebra): 46 rows, 79.34 ms; proofs using ax-mp directly: 5361 rows, 14.53 ms; count by kind: 3 rows, 97.06 ms |
| redb | 51159 | 48083 | 98136 | 2.17s | 10.91s | 1028 MiB | axioms behind 2p2e4: 15 rows, 2.42 ms; axioms behind fta (fundamental thm of algebra): 46 rows, 46.53 ms; proofs using ax-mp directly: 5361 rows, 13.53 ms; count by kind: 3 rows, 66.98 ms |
| lmdb | 51159 | 48083 | 98136 | 2.17s | 6.21s | 791 MiB | axioms behind 2p2e4: 15 rows, 1.85 ms; axioms behind fta (fundamental thm of algebra): 46 rows, 37.14 ms; proofs using ax-mp directly: 5361 rows, 10.04 ms; count by kind: 3 rows, 52.63 ms |

### Where the space goes (SQLite, set.mm)

| Table | MiB | Share |
|---|---|---|
| definitions (canonical bytes, display, refs blob) | 238 | 42 % |
| `t_ref` (reference edges) | 125 | 22 % |
| `t_ref_dst` (the reverse index) | 115 | 20 % |
| log | 28 | 5 % |
| everything else | 59 | 11 % |

- Canonical bytes: 49 MiB of statements and 83 MiB of proofs.
- 1.5 M reference edges.

### What this says

- **Real queries are fast:**
  - the axioms behind `2p2e4` take 1.9 ms on LMDB;
  - the axioms behind the fundamental theorem of algebra (46 of them, a deep chain) take 37 ms;
  - LMDB is fastest on every query, redb next, SQLite last but within about 2×.
- **Ingest:** LMDB 6.2 s, redb 10.9 s, SQLite 15.3 s, of which 2.2 s is parsing.
- **Reference edges dominate storage on real code** ([[SP-011 Reference edges dominate storage on real code]]): about 31 references per proof, against 3 in the synthetic generator. **H3 is refuted on synthetic code and confirmed on real code.** Synthetic benchmarks understated the cost of 32-byte references by about 10×. That is the reason for B-16.
- What the library taught the design: [[Real Code Corpora]] §4.

## 2026-10-20 (evening): B-17, Sophia's own query language (SPEC-007), and the zero-copy breakdown

- Machine: AMD Ryzen 7 7800X3D, 8 threads, 30 GiB RAM, Linux 7.2.9-arch1-1.
- Commit: the M3a commit (`sophia-query`).
- Commands:
  - `cargo run --release -p sophia-bench --bin sophia-bench-report -- --sizes 1000 --query-size 10000 --only-queries`
  - `… --query-size 100000 --zero-copy --skip-queries --only-queries`
  - `… --query-size 100000 --backends sqlite,redb,lmdb --only-queries` (DuckDB left out at 10⁵: ingest takes about 17 minutes)
- Workload: the synthetic corpus (seed 42). Per-probe workloads use 20 random probes, and report time and rows per probe.
- "No magic sets" runs compute the whole relation whatever the probe, so they are timed **once, on the first probe** (its row count is that probe's, not the average).
- Native step limit: 50M join steps. SQL step limit: 50M SQLite VM steps. The two limits are not comparable.

### B-17 at 10 000 definitions

| backend | definitions | workload | executor | rows (per probe) | median |
|---|---|---|---|---|---|
| sqlite | 10000 | direct dependants | native | 2 | 0.005 ms |
| sqlite | 10000 | direct dependants | SQL | 2 | 0.005 ms |
| sqlite | 10000 | transitive dependants | native | 101 | 0.171 ms |
| sqlite | 10000 | transitive dependants | native, no magic sets (1 probe) | 0 | 6160.590 ms |
| sqlite | 10000 | transitive dependants | SQL | 101 | 0.167 ms |
| sqlite | 10000 | transitive dependants | SQL, no magic sets (1 probe) | 0 | aborted at the step limit |
| sqlite | 10000 | transitive dependencies | native | 406 | 0.970 ms |
| sqlite | 10000 | transitive dependencies | native, no magic sets (1 probe) | 689 | 7358.344 ms |
| sqlite | 10000 | transitive dependencies | SQL | 406 | 0.839 ms |
| sqlite | 10000 | transitive dependencies | SQL, no magic sets (1 probe) | 0 | aborted at the step limit |
| sqlite | 10000 | count by kind | native | 1 | 10.068 ms |
| sqlite | 10000 | count by kind | SQL | 1 | 4.622 ms |
| sqlite | 10000 | unreferenced definitions | native | 1 | 19.895 ms |
| sqlite | 10000 | unreferenced definitions | SQL | 1 | 2.742 ms |
| sqlite | 10000 | names by prefix | native | 100 | 3.818 ms |
| sqlite | 10000 | names by prefix | SQL | 100 | 0.443 ms |
| redb | 10000 | direct dependants | native | 2 | 0.002 ms |
| redb | 10000 | transitive dependants | native | 101 | 0.180 ms |
| redb | 10000 | transitive dependants | native, no magic sets (1 probe) | 0 | 5945.146 ms |
| redb | 10000 | transitive dependencies | native | 406 | 1.219 ms |
| redb | 10000 | transitive dependencies | native, no magic sets (1 probe) | 689 | 7286.904 ms |
| redb | 10000 | count by kind | native | 1 | 5.438 ms |
| redb | 10000 | unreferenced definitions | native | 1 | 13.600 ms |
| redb | 10000 | names by prefix | native | 100 | 3.485 ms |
| lmdb | 10000 | direct dependants | native | 2 | 0.002 ms |
| lmdb | 10000 | transitive dependants | native | 101 | 0.132 ms |
| lmdb | 10000 | transitive dependants | native, no magic sets (1 probe) | 0 | 5954.229 ms |
| lmdb | 10000 | transitive dependencies | native | 406 | 0.995 ms |
| lmdb | 10000 | transitive dependencies | native, no magic sets (1 probe) | 689 | 7523.000 ms |
| lmdb | 10000 | count by kind | native | 1 | 4.794 ms |
| lmdb | 10000 | unreferenced definitions | native | 1 | 8.254 ms |
| lmdb | 10000 | names by prefix | native | 100 | 3.005 ms |
| duckdb | 10000 | direct dependants | native | 2 | 0.336 ms |
| duckdb | 10000 | transitive dependants | native | 101 | 18.754 ms |
| duckdb | 10000 | transitive dependants | native, no magic sets (1 probe) | 0 | 8273.397 ms |
| duckdb | 10000 | transitive dependencies | native | 406 | 146.489 ms |
| duckdb | 10000 | transitive dependencies | native, no magic sets (1 probe) | 689 | 9887.424 ms |
| duckdb | 10000 | count by kind | native | 1 | 7.601 ms |
| duckdb | 10000 | unreferenced definitions | native | 1 | 1982.238 ms |
| duckdb | 10000 | names by prefix | native | 100 | 6.366 ms |

### B-17 at 100 000 definitions (SQLite, redb, LMDB)

The first attempt at this size was stopped by hand: the non-magic closure grew past 14 GB before reaching the 50M-step limit. That is how the **derived-tuple cap** (5M by default) came about: the step budget alone does not bound memory. With the cap, the run stayed near 2 GB.

| backend | definitions | workload | executor | rows (per probe) | median |
|---|---|---|---|---|---|
| sqlite | 100000 | direct dependants | native | 2 | 0.004 ms |
| sqlite | 100000 | direct dependants | SQL | 2 | 0.006 ms |
| sqlite | 100000 | transitive dependants | native | 225 | 0.643 ms |
| sqlite | 100000 | transitive dependants | native, no magic sets (1 probe) | 0 | aborted at the derived-tuple limit |
| sqlite | 100000 | transitive dependants | SQL | 225 | 0.611 ms |
| sqlite | 100000 | transitive dependants | SQL, no magic sets (1 probe) | 0 | aborted at the step limit |
| sqlite | 100000 | transitive dependencies | native | 2055 | 7.572 ms |
| sqlite | 100000 | transitive dependencies | native, no magic sets (1 probe) | 0 | aborted at the derived-tuple limit |
| sqlite | 100000 | transitive dependencies | SQL | 2055 | 6.903 ms |
| sqlite | 100000 | transitive dependencies | SQL, no magic sets (1 probe) | 0 | aborted at the step limit |
| sqlite | 100000 | count by kind | native | 1 | 105.918 ms |
| sqlite | 100000 | count by kind | SQL | 1 | 57.352 ms |
| sqlite | 100000 | unreferenced definitions | native | 1 | 309.679 ms |
| sqlite | 100000 | unreferenced definitions | SQL | 1 | 32.478 ms |
| sqlite | 100000 | names by prefix | native | 100 | 38.649 ms |
| sqlite | 100000 | names by prefix | SQL | 100 | 4.823 ms |
| redb | 100000 | direct dependants | native | 2 | 0.002 ms |
| redb | 100000 | transitive dependants | native | 225 | 0.494 ms |
| redb | 100000 | transitive dependants | native, no magic sets (1 probe) | 0 | aborted at the derived-tuple limit |
| redb | 100000 | transitive dependencies | native | 2055 | 6.854 ms |
| redb | 100000 | transitive dependencies | native, no magic sets (1 probe) | 0 | aborted at the derived-tuple limit |
| redb | 100000 | count by kind | native | 1 | 65.018 ms |
| redb | 100000 | unreferenced definitions | native | 1 | 167.503 ms |
| redb | 100000 | names by prefix | native | 100 | 39.374 ms |
| lmdb | 100000 | direct dependants | native | 2 | 0.001 ms |
| lmdb | 100000 | transitive dependants | native | 225 | 0.368 ms |
| lmdb | 100000 | transitive dependants | native, no magic sets (1 probe) | 0 | aborted at the derived-tuple limit |
| lmdb | 100000 | transitive dependencies | native | 2055 | 5.853 ms |
| lmdb | 100000 | transitive dependencies | native, no magic sets (1 probe) | 0 | aborted at the derived-tuple limit |
| lmdb | 100000 | count by kind | native | 1 | 51.038 ms |
| lmdb | 100000 | unreferenced definitions | native | 1 | 98.657 ms |
| lmdb | 100000 | names by prefix | native | 100 | 31.189 ms |

### Zero-copy breakdown (LMDB, 100 000 definitions)

| step | ns per record |
|---|---|
| get, borrowed from the map (no copy) | 207 |
| get + copy into a Vec | 620 |
| get + walk the framing in place (no allocation) | 323 |
| `ReadTx::record` (copy + decode the record) | 868 |
| record + decode the core term (heap objects) | 11597 |

### What this says

**Magic sets are the first-order effect.**
- Without them, "transitive dependants of X" evaluates the closure of the whole store:
  - 6–8 s per query natively;
  - over the step limit in SQL.
- With them, it takes 0.13–0.18 ms on SQLite, redb and LMDB. That is 4.5 orders of magnitude, far more than any backend difference.
- The rewrite is an IR → IR transformation, so the SQL translation benefits identically (0.167 ms).

**At 10⁵ the shape holds.** With magic sets, transitive dependants take:
- 0.37 ms on LMDB;
- 0.49 ms on redb;
- 0.64 ms on SQLite natively, and 0.61 ms as SQL.

Without magic sets, every executor stops at its limit. Native count-by-kind and negation scale linearly (LMDB: 51 ms and 99 ms; SQLite SQL: 57 ms and 32 ms).

**Backend independence costs little on recursive queries.**
- On SQLite, the native executor and SQLite's own engine running the translated SQL are equal: 0.171 against 0.167 ms for transitive dependants, 0.97 against 0.84 ms for dependencies.
- LMDB is the fastest native backend (0.132 ms). It has no query language of its own, so this is exactly the case S-ADR-0011 was for.

**Set-at-a-time workloads favour SQL by 2–9× on SQLite:**

| Workload | Native | SQL |
|---|---|---|
| count by kind | 10.1 ms | 4.6 ms |
| unreferenced definitions | 19.9 ms | 2.7 ms |
| names by prefix | 3.8 ms | 0.44 ms |

The native executor works tuple at a time:
- with `Val` allocations per tuple;
- one `dependants` lookup per definition for the negation;
- a full name scan where a prefix range scan would do.

These are known, fixable costs (SPEC-007 §11):
- a prefix access path;
- set-at-a-time anti-joins for negation over a scanned relation;
- borrowed field reads.

LMDB natively is already within 2× of SQLite's SQL on count by kind.

**DuckDB runs the native executor correctly but slowly on point lookups** (about 0.3 ms each): 18.8 ms for transitive dependants, 2 s for the negation. It was 1.9 s for count by kind before its set-at-a-time access paths (`all_kinds`, `all_refs`) were added, and 7.6 ms after. Confirms S-ADR-0010: DuckDB is for analytics, not a primary.

**Zero-copy (SR-101):** see [[Zero-Copy Storage of Canonical Graphs]].
- Borrowed LMDB reads cost 0.21 µs; the store's copy-and-decode costs 0.87 µs.
- Decoding the core term into heap objects costs 11.6 µs.
- The database is not where time is lost. Heap term construction is.

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

