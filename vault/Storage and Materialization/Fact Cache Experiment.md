#measurement #storage #proposal

> B-035, part 1 (2026-11-20): the cheap-fact workload of the fact cache experiment ([[SPEC-001 Sophia Store v0]] §8a, SR-152). It replays the full first-parent histories of Lenticulum.jl, Moonkale and Sophia as folder stores, then ten helper worktrees and review copies as fresh folders. **Along a history, the cache adds almost nothing:** the folder store already holds 71 to 85 % of the definitions in changed files, and the cache adds 0.01 to 1.8 %. **For a fresh full worktree, or a rebuilt derived store, the cache serves 86 to 89 %**, and leaves 0 to 2 % to compute. Small partial review copies are served far less: 4 to 21 %. Everything ever seen comes to 16 705 distinct ids. That is about 5 to 80 MB per fact kind in SQLite, depending on the value's size. A lookup takes microseconds. **Part 2 is open:** the cost of an expensive tool (Lean elaboration). The threshold, and so the default, depend on it. Nothing was built; the cache file waits for Daniel (SR-126). **Codex's review (2026-11-20) corrected this page and the program** ([[Review 2026-11-20 Codex B-035 (external)]]). Two caveats remain:
- **Julia ids were computed per file.** Moonkale resolves Julia references module-wide, so the Julia counts approximate what its folder store would see.
- **The tables below come from the program before the review's fixes.** They must be re-run on an idle machine.
>
> Sources: [[SPEC-001 Sophia Store v0]] §8a; [[SPEC-009 Moonkale Folder Store]] (facts follow their files); code: `crates/sophia-ingest/examples/fact_cache.rs` ([[fact_cache]]).

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
| Lenticulum.jl | `2252593667` | 339 |
| Moonkale | `d0e784c761` | 294 |
| Sophia | `74e345aa63` | 132 |

The heads are each checkout's files on 2026-11-20, uncommitted changes included. The run took 37 s and peaked at 228 MB. The outputs are in `$O/summary.tsv` and `$O/commits.tsv`. The model is in [[fact_cache]]: one content-only fact per definition, keyed by its id; a file whose text changed recomputes its definitions. The check passed: each replayed live set equals a fresh build of its HEAD.

## Results: hits

The **local** column counts definitions that the folder store already held: unchanged definitions in a changed file, moved definitions, and duplicates within one event. The **cache** column counts those it adds beyond that.

| folder | event | defs recomputed without reuse | local | cache (same folder) | cache (other folder) | runs left |
|---|---|---|---|---|---|---|
| Lenticulum.jl | 338 commits | 45 101 | 38 234 (84.8 %) | 3 | 0 | 6 864 |
| Moonkale | 293 commits | 28 410 | 20 722 (72.9 %) | 4 | 6 | 7 678 |
| Sophia | 131 commits | 6 710 | 4 749 (70.8 %) | 1 | 122 (1.8 %) | 1 838 |
| Lenticulum.jl-codex | fresh | 5 362 | 732 | 0 | 4 607 (85.9 %) | 23 |
| Lenticulum.jl-mistral | fresh | 5 264 | 712 | 0 | 4 548 (86.4 %) | 4 |
| Moonkale-dev-claude | fresh | 6 352 | 745 | 0 | 5 501 (86.6 %) | 106 |
| Moonkale-codex | fresh | 6 250 | 730 | 0 | 5 520 (88.3 %) | 0 |
| Moonkale-dev-mistral | fresh | 6 301 | 742 | 0 | 5 546 (88.0 %) | 13 |
| Sophia-codex | fresh | 1 089 | 119 | 0 | 970 (89.1 %) | 0 |
| Sophia-mistral | fresh | 1 134 | 129 | 0 | 1 005 (88.6 %) | 0 |

**The three Lenticulum.jl review copies do not follow that pattern.** They hold only a few files (24, 28 and 240 definitions), often files being changed. The cache served 4.2 %, 21.4 % and 8.8 % of them, and left 25.0 %, 57.1 % and 55.8 % to compute.

**Reading:**
- **Along one history, the folder store is the cache.** It already holds everything a commit keeps. Reverts and definitions that come back are almost absent: 8 of 80 221 definitions across the three histories. Sophia has 122 cross-folder hits. Where they came from was not recorded.
- **The cache pays for fresh full folders.** These are a new helper worktree, and a derived store deleted and rebuilt (SPEC-009). Here 86 to 89 % of the work is served from the cache, and nearly all of the rest is duplicate ids within the tree, which the folder store dedups itself. In Daniel's setup, three to five helper worktrees per project are made and thrown away every few days, so this is the common case.
- **Columns.** "Same folder" means the id was live in this folder at an earlier event. Where it was computed is not tracked.
- **Hit rate depends on keying facts by content.** These rates hold only for content-only facts. Facts with an `Input::Relation`, such as dispatch resolutions, stay in their store (§8a, point 3) and are recomputed in a fresh folder.

## Results: size and lookup

- **Size.** All three histories and ten checkouts together give **16 705 distinct definition ids**. That is the size of the `all-content-only` cache per fact kind, with nothing ever evicted.
- **Frontend cost.** The per-file frontend (parse, normalise, hash), the cheapest tool there is, spent 32.6 s on 91 395 definitions in 5 905 distinct file texts: **357 µs per definition**.

Cache stores of that size are built through `Store::record_facts`, so the log is included. Sizes are taken after the store is closed. Lookups were 16 705 hits (drawn with replacement) and 16 705 misses, interleaved, each a `facts_of` in its own read transaction. The percentiles below pool hits and misses; the fixed program reports them apart. Sizes are apparent file lengths: redb's 1 KiB store allocates 46.2 MB of its 67.4 MB.

| backend | value bytes | file length | bytes per fact | write µs per fact | lookup p50 µs | p90 µs | p99 µs |
|---|---|---|---|---|---|---|---|
| SQLite | 8 | 5.4 MB | 323 | 9.8 | 3.6 | 4.6 | 5.4 |
| redb | 8 | 16.8 MB | 1 008 | 9.6 | 1.3 | 1.5 | 1.8 |
| SQLite | 72 | 6.7 MB | 398 | 13.9 | 3.7 | 5.1 | 6.2 |
| redb | 72 | 33.7 MB | 2 016 | 10.6 | 1.3 | 1.7 | 2.4 |
| SQLite | 1 024 | 80.8 MB | 4 839 | 47.4 | 6.3 | 14.9 | 16.3 |
| redb | 1 024 | 67.4 MB | 4 033 | 15.0 | 1.4 | 1.9 | 3.8 |

**Caveat on timing.** A Julia job of Daniel's was running on the machine during both runs. Every time on this page (frontend, write, lookup) may be inflated and should be measured again on an idle machine. The counts and sizes do not depend on load.

**Reading:**
- **Size does not decide the policy.** Even unbounded, and even with 1 KiB values, the cache stays under 100 MB per fact kind for three projects' whole history.
- **Lookup does not decide it either.** A lookup takes 1 to 16 µs and a write 10 to 50 µs. The frontend's 357 µs per definition is not saved by a hit, though: a changed file goes through the frontend anyway, to get the ids to look up. So a hit saves only the tool's own cost beyond the frontend. That cost was not measured, and neither was a replay timed with and without the cache. A lookup costs 1 to 15 % of the frontend's time on the same definition, which bounds the overhead per miss.
- **Facts that come free with another run gain nothing.** Source spans are the example: they come out of the parse. A cache entry saves only a run that would really have happened.
- **SQLite and large values.** With 1 KiB values, SQLite's `t_fact` takes 4.7 KB per row (78.3 of 80.8 MB; `dbstat`). The table is `WITHOUT ROWID`, and SQLite advises against that for rows this large. A certificate-sized value (§8a, point 1) belongs in a rowid table, or a blob outside the row. This is a storage detail for the cache file, not for now.
- **redb's files are allocated in regions,** so at this small size its bytes per fact say little.

## Open: part 2, expensive facts

The threshold policy needs the cost distribution of a tool that is really expensive. Sophia has none (§8a). The planned candidate is Lean elaboration of Mathlib declarations:
- Mathlib `98a293fac8`, Lean v4.36.0-rc1;
- `lake env lean -Dtrace.profiler=true -Dtrace.profiler.threshold=0 FILE` prints an `[Elab.command] [seconds]` line per command;
- a first try on `Mathlib/Data/Nat/Factorial/Basic.lean` (573 lines) took 22 s and printed 187 731 lines.

It was stopped because the machine was busy. To do:
- profile a sample of files on an idle machine;
- take the per-declaration elaboration times;
- set a threshold against the lookup and write costs above;
- measure again the timings of part 1.

## Proposal (expectation, until part 2 is measured)

The default would be **`threshold(t)` with `t` small (of the order of 1 ms)**, not 1 s.
- **Facts that cost less than about 1 ms** gain little per hit. They are numerous, though, and the lookup is cheap, so including them costs little.
- **The fresh-folder case** is where the cache pays. A 1 s threshold would cache nothing from today's tools.
- **`all-content-only`** is just as defensible at these sizes. The difference between the two policies only matters once a tool emits many facts that cost microseconds. Part 2 decides.

The cache store itself is a new storage file, which needs Daniel's agreement (SR-126). Nothing was added.
