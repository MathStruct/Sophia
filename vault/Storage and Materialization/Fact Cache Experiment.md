#measurement #storage #proposal

> B-035, part 1 (2026-11-20): the cheap-fact workload of the fact cache experiment ([[SPEC-001 Sophia Store v0]] §8a, SR-152). It replays the full first-parent histories of Lenticulum.jl, Moonkale and Sophia as folder stores, then ten helper worktrees and review copies as fresh folders.
> - **Along a history, the cache adds almost nothing.** The folder store already holds 71 to 85 % of the definitions in changed files. The cache adds 0.17 % overall, and at most 1.8 % in one history.
> - **A fresh full worktree, or a rebuilt derived store, gets 86 to 89 % from the cache**, and leaves 0 to 2 % to compute. Small partial review copies get only 4 to 23 %.
> - **Size and lookup are small.** Everything ever seen comes to 16 741 distinct ids: 5 to 81 MB per fact kind in SQLite, depending on the value's size. A lookup takes 0.7 to 14 µs (p50).
>
> Two caveats. Julia ids were computed per file, while Moonkale resolves them module-wide. 40 % of Rust definitions got no id from the normal form, so they are not counted.
>
> **Part 2 (2026-11-21): the expensive tool.** Lean elaborated 60 sampled Mathlib files, command by command. **99 % of declarations cost 1 ms or more** (median 8 ms, p99 219 ms, the largest 674 ms); 44 % cost 10 ms or more and hold 88 % of the time. Reviewed by Codex ([[Review 2026-11-21 Codex B-035 part 2 (external)]]) and re-run after the fixes. A cache write costs 9 to 46 µs and a miss 0.7 to 3.6 µs, so one hit on a declaration of 1 ms repays its write 20 to 100 times.
>
> **Proposed default: `threshold(1 ms)`** (for Daniel). Nothing was built; the cache file waits for Daniel (SR-126).
>
> Sources: [[SPEC-001 Sophia Store v0]] §8a; [[SPEC-009 Moonkale Folder Store]] (facts follow their files); [[Review 2026-11-20 Codex B-035 (external)]]; [[Lean Overview]] (export sizes); code: `crates/sophia-ingest/examples/fact_cache.rs` ([[fact_cache]]), `crates/sophia-ingest/examples/lean_elab_costs.rs` ([[lean_elab_costs]]), `experiments/lean-elab-costs/elab_costs.lean` ([[elab_costs]]).

## How it was measured

```sh
cargo build --release -p sophia-ingest --example fact_cache
O=$SOPHIA_CORPORA/b035; C=~/Code/CategoryTheory; M=~/Code/Moonkale
systemd-run --user --wait --collect -p MemoryMax=8G -- ./target/release/examples/fact_cache $O \
  --history $C/Lenticulum.jl --history $M/Moonkale --history $C/Sophia \
  --head $C/Lenticulum.jl-codex --head $C/Lenticulum.jl-mistral \
  --head $C/Lenticulum.jl-Review-2026-11-09 --head $C/Lenticulum.jl-Review-2026-11-15 --head $C/Lenticulum.jl-Review-2026-11-20 \
  --head $M/Moonkale-dev-claude --head $M/Moonkale-codex --head $M/Moonkale-dev-mistral \
  --head $C/Sophia-codex --head $C/Sophia-mistral
```

**Inputs:**

| repository | HEAD | commits (first parent) |
|---|---|---|
| Lenticulum.jl | `3ff93526df` | 340 |
| Moonkale | `d0e784c761` | 294 |
| Sophia | `05b802aea2` | 133 |

The heads are each checkout's files on 2026-11-20 at 23:54, uncommitted changes included.

**Conditions.** The machine was idle: load average 0.1 before the run. The run took 33 s and peaked at 229 MB. Two runs a minute apart agree within 1 % on every time. The outputs are in `$O/summary.tsv` and `$O/commits.tsv`; the first, pre-review run on a busy machine is kept in `$O/run1/`.

**The model and the check** are in [[fact_cache]]. A tool computes one content-only fact per definition, keyed by the definition's id. A file whose text changed recomputes its definitions. The check passed: each replayed live set equals a fresh build of its HEAD.

**Coverage.** Each distinct text went through Sophia's per-file frontend once. No file was refused whole. Definitions the normal form refuses get no id, so they get no fact and are not counted:

| language | distinct texts | definitions with an id | refused |
|---|---|---|---|
| Julia | 1 856 | 44 253 | 1 378 (3 %) |
| Rust | 2 969 | 34 323 | 23 194 (40 %) |
| Python | 68 | 116 | 151 (57 %) |

Two consequences:
- **Moonkale's and Sophia's counts cover about 60 % of their Rust definitions.** Refusals are not random: the review gives a function with a `for` loop as an example. So the uncounted definitions may change more or less often than the counted ones.
- **Julia ids are per file.** Moonkale resolves references module-wide (`cid::julia::module`), so when one file changes, ids in another can change too. The Julia counts approximate what Moonkale's folder store would see; a module-wide mode is B-035 part 1b.

## Results: hits

The **local** column counts definitions the folder store already holds: unchanged definitions in a changed file, moved definitions, and duplicates within one event. The cache columns count what the cache adds beyond that:
- **same folder:** the id was live in this folder at an earlier event;
- **other folder:** it never was.

Where an id was computed is not tracked.

| folder | event | defs recomputed without reuse | local | cache (same folder) | cache (other folder) | runs left |
|---|---|---|---|---|---|---|
| Lenticulum.jl | 339 commits | 45 379 | 38 442 (84.7 %) | 3 | 0 | 6 934 |
| Moonkale | 293 commits | 28 410 | 20 722 (72.9 %) | 4 | 6 | 7 678 |
| Sophia | 132 commits | 6 722 | 4 750 (70.7 %) | 1 | 122 (1.8 %) | 1 849 |
| Lenticulum.jl-codex | fresh | 5 362 | 732 | 0 | 4 617 (86.1 %) | 13 |
| Lenticulum.jl-mistral | fresh | 5 264 | 712 | 0 | 4 548 (86.4 %) | 4 |
| Moonkale-dev-claude | fresh | 6 352 | 745 | 0 | 5 501 (86.6 %) | 106 |
| Moonkale-codex | fresh | 6 250 | 730 | 0 | 5 520 (88.3 %) | 0 |
| Moonkale-dev-mistral | fresh | 6 301 | 742 | 0 | 5 546 (88.0 %) | 13 |
| Sophia-codex | fresh | 1 089 | 119 | 0 | 970 (89.1 %) | 0 |
| Sophia-mistral | fresh | 1 134 | 129 | 0 | 1 005 (88.6 %) | 0 |
| Lenticulum.jl-Review-2026-11-09 | fresh, partial copy | 24 | 17 | 0 | 1 (4.2 %) | 6 (25.0 %) |
| Lenticulum.jl-Review-2026-11-15 | fresh, partial copy | 28 | 6 | 0 | 6 (21.4 %) | 16 (57.1 %) |
| Lenticulum.jl-Review-2026-11-20 | fresh, partial copy | 240 | 85 | 0 | 56 (23.3 %) | 99 (41.3 %) |

The first commit of each history is a fresh build and is left out of its totals.

**Reading:**
- **Along one history, the folder store is the cache.** It already holds everything a commit keeps. Reverts and definitions that come back are almost absent: 8 of 80 511 definitions across the three histories. Sophia's 122 cross-folder hits were not traced to their files.
- **The cache pays for fresh full folders.** These are a new helper worktree, and a derived store deleted and rebuilt (SPEC-009). The cache serves 86 to 89 % of the work. Nearly all of the rest is duplicate ids within the tree, which the folder store dedups itself. In Daniel's setup, helper worktrees are made and thrown away every few days, so this is a common case.
- **Partial review copies gain less.** They hold a few files, often the ones being changed, so 25 to 57 % of their definitions are new.
- **These rates hold only for content-only facts.** Facts with an `Input::Relation`, such as dispatch resolutions, stay in their store (§8a, point 3) and are recomputed in a fresh folder.

## Results: size and lookup

All three histories and ten checkouts give **16 741 distinct definition ids**. That is the size of the `all-content-only` cache per fact kind, with nothing ever evicted.

**The frontend.** Sophia's per-file frontend (parse, normalise, hash) spent 27.0 s on 78 692 definitions in 4 893 distinct texts: **343 µs per definition**.

**The cache stores.** One fact per id, written through `Store::record_facts`, so the log is included. Sizes are taken after closing:
- **file:** the apparent length;
- **allocated:** the blocks the file system gave it.

Lookups are 50 000 hits (drawn with replacement) and 50 000 misses, interleaved, each a `facts_of` in its own read transaction. Times are in µs.

| backend | value bytes | file | allocated | file bytes per fact | write per fact | hit p50 | hit p99 | miss p50 | miss p99 |
|---|---|---|---|---|---|---|---|---|---|
| SQLite | 8 | 5.4 MB | 5.4 MB | 323 | 9.3 | 4.3 | 5.2 | 2.6 | 3.5 |
| redb | 8 | 16.8 MB | 15.2 MB | 1 006 | 9.0 | 1.4 | 1.8 | 0.7 | 0.8 |
| SQLite | 72 | 6.7 MB | 6.7 MB | 400 | 12.4 | 4.4 | 5.7 | 2.6 | 3.6 |
| redb | 72 | 33.7 MB | 18.1 MB | 2 012 | 9.4 | 1.4 | 1.6 | 0.7 | 0.8 |
| SQLite | 1 024 | 81.0 MB | 81.0 MB | 4 839 | 46.0 | 13.7 | 16.6 | 3.3 | 3.6 |
| redb | 1 024 | 67.4 MB | 46.3 MB | 4 024 | 13.8 | 1.5 | 1.8 | 0.7 | 0.8 |

**Reading:**
- **Size does not decide the policy.** Even unbounded, and with 1 KiB values, the cache stays under 100 MB per fact kind for three projects' whole history.
- **What a hit saves.** A hit does not save the frontend's 343 µs, since a changed file goes through the frontend anyway to get the ids to look up. It saves only the tool's own cost beyond the frontend, which was not measured. Neither was a replay timed with and without the cache.
- **What a miss costs.** A lookup that misses takes 0.7 to 3.6 µs, 1 % of the frontend's time on the same definition or less. That bounds what the cache costs where it does not help.
- **Facts that come free with another run gain nothing.** Source spans are the example: they come out of the parse.
- **SQLite and large values.** With 1 KiB values, each SQLite fact takes 4.8 KB, and a hit is 9× slower than in redb. The fact table is `WITHOUT ROWID`, which SQLite advises against for rows this large. A certificate-sized value (§8a, point 1) belongs in a rowid table or a blob outside the row. This is a detail of the cache file, for later.
- **redb's file grows in regions.** At this small size, its bytes per fact say little.

## Part 2: the cost of an expensive tool (Lean elaboration)

The threshold policy needs the costs of a tool that is really expensive. Sophia has none yet (§8a), so Lean's elaborator stands in: the fact would be a declaration's elaborated term, and its cost the time Lean spends on the declaration.

```sh
cargo build --release -p sophia-ingest --example lean_elab_costs
O=$SOPHIA_CORPORA/b035-lean/run2
systemd-run --user -P --wait --collect -p MemoryMax=8G --setenv=PATH=$HOME/.elan/bin:/usr/bin:/bin --working-directory=$PWD -- \
  ./target/release/examples/lean_elab_costs $O ~/Code/CategoryTheory/mathlib4 experiments/lean-elab-costs/elab_costs.lean --sample 60
```

**Inputs.** Mathlib `98a293fac8`, Lean v4.36.0-rc1 (its prebuilt cache). 60 of Mathlib's 8 650 files, drawn by a seeded hash of the path (seed 35; [[lean_elab_costs]]): 15 189 lines, from 11 directories of `Algebra` to 2 of `Probability`.

**Method.** The driver [[elab_costs]] runs Lean's own frontend over each file, one source command at a time, synchronously (`Elab.async = false`, so a proof is charged to its theorem). It records each command's wall time and heartbeats. **The unit is the source command**, as a cache entry would be keyed by one source definition: a command that a macro turns into several declarations (one in `DomAct/Basic.lean` makes instances for 24 classes) counts once, with all its time. Lean's trace profiler was tried first and dropped: it prints each macro expansion of a command beside the command itself, so its lines count the work twice (145 lines for 68 commands in `Mathlib/Data/WSeq/Relation.lean`), and at `threshold=0` it inflated times by about 14 %.

**Conditions and checks.**
- **Two runs.** Run 1 (`$SOPHIA_CORPORA/b035-lean/run1`) was on an idle machine: load average 0.14, 4 min 3 s, peak 3.8 GB. Codex's review found defects in the driver and the runner; run 2 (`run2`) is after the fixes, same sample. During run 2, two Julia jobs not started by this session used 2 of the 8 cores (load average 2.1 at the start); Lean is single-threaded here. Run 2 took 5 min 31 s, peak 4.0 GB. The tables below are run 2's.
- **The runs agree.** Per file, run 2's command time divided by run 1's has a median of 0.999. The commands that moved most were early commands of small files, which dropped from about 25 ms to 5 to 15 ms. Run 2 imports with `leakEnv` as `lean` does, the most likely cause of that one-time cost (review, point 3); it was not isolated. In total, 41.6 s against 41.0 s.
- Every file elaborated without an error, in `lean` (its exit status) and in the driver (per command).
- **The driver against `lean`.** Per file, the driver's import time plus its command times, divided by the wall time of `lean -DElab.async=false FILE` (run warm, after the driver), is 1.00 to 1.18, median 1.07. Summed: 52.9 s of imports and 41.6 s of commands, against 89.4 s of `lean`. **The driver's times are thus about 7 % above what `lean` spends, process start included.** The cause is not known; the driver process also holds Lean's own environment for interpreting itself (peak 4 GB against about 1.6 GB). It is far below the factor of 10 between thresholds.
- Time and heartbeats agree: the correlation of their logarithms over the declarations is 0.995.

**Results.** 2 918 commands, of which 1 846 are declarations: Lean's `declaration` command (theorem, def, instance, `example`, ...), or any command that added a constant (Mathlib's `lemma`, `alias`, and some `notation`, `attribute` and `deriving` commands). Times are in ms.

| set | count | total | p10 | p50 | p90 | p99 | max |
|---|---|---|---|---|---|---|---|
| declarations | 1 846 | 40.7 s | 2.2 | 8.1 | 50 | 219 | 674 |
| all commands | 2 918 | 41.6 s | 0.42 | 3.8 | 33 | 179 | 674 |

| threshold | declarations at or above it | their share of declaration time | their share of all command time |
|---|---|---|---|
| 0.1 ms | 1 846 (100 %) | 100 % | 97.8 % |
| 1 ms | 1 832 (99.2 %) | 100 % | 97.7 % |
| 10 ms | 820 (44.4 %) | 88.2 % | 86.2 % |
| 100 ms | 76 (4.1 %) | 35.9 % | 35.1 % |
| 1 s | 0 | 0 | 0 |

By kind, the median declaration costs 8.7 ms (926 theorems), 9.9 ms (372 `lemma`s), 12.0 ms (224 definitions) and 6.3 ms (226 instances). The 1 072 bookkeeping commands (`variable`, `section`, `end`, `open`, docs) cost 0.9 s in all. The imports cost 0.5 to 1.2 s per file, once per file, not per declaration.

**Reading:**
- **Two clusters, far apart.** Sophia's frontend spends 343 µs per definition, and the facts it yields (spans, call edges) come out of that run for free. Lean's elaborator spends 1 ms or more on 99 % of declarations, and 8 ms on the median one. A threshold of 1 ms falls between them.
- **What a hit is worth.** A cache write costs 9 to 46 µs and a miss 0.7 to 3.6 µs (part 1). One hit on a declaration of 1 ms repays its write 20 to 100 times; on the median declaration, 180 to 900 times.
- **What it costs in space.** Not measured per declaration here. As an order of magnitude, `lean4export` writes 5.9 GB for Mathlib and its dependencies' 717 000 declarations ([[Lean Overview]]), about 8 KB each with the shared term graph. A cache holding all of Mathlib's elaborated terms would be some GB. For three projects' cheap facts, part 1 gave tens of MB.
- **Not covered.** No declaration in the sample reached 1 s. Mathlib has some that take much longer (those that raise `maxHeartbeats`), and a sample of 60 files is unlikely to contain one. A larger or a targeted sample would show the tail; it does not change the threshold, only the share of time above 100 ms.

## Proposal: the default (for Daniel)

**`facts/cache-policy = threshold(1 ms)`.**
- **It separates what is cheap from what is not.** The facts Sophia makes today (spans, call edges, imports) come out of its frontend, which spends 343 µs per definition on all of them together (part 1), so they fall below it. Almost every elaborated declaration (99 %) is above it.
- **Above 1 ms, caching always pays.** A hit on a 1 ms declaration saves 300 to 1 400 times what a miss costs, and a write is repaid on the first hit.
- **`all-content-only` adds only cheap facts.** They would fill the cache with values a fresh folder recomputes in well under a millisecond (part 1: the cache serves 86 to 89 % of a fresh worktree, but each hit saves only the tool's cost beyond the frontend). The difference is small at today's sizes; the threshold keeps the cache to what is worth it.
- **A size cap is a separate question.** If the cache must be bounded (Mathlib's elaborated terms alone are some GB), a higher threshold is the cheap way: 10 ms keeps 44 % of the declarations and 88 % of the time. Eviction by cost per byte, as `all-content-only` has, would serve the same end. Not proposed until a cache is near a bound.

The cache store itself is a new storage file, which needs Daniel's agreement (SR-126). Nothing was added.
