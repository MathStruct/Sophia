#measurement #storage #proposal

> B-035, part 1 (2026-11-20): the cheap-fact workload of the fact cache experiment ([[SPEC-001 Sophia Store v0]] §8a, SR-152). It replays the full first-parent histories of Lenticulum.jl, Moonkale and Sophia as folder stores, then ten helper worktrees and review copies as fresh folders.
> - **Along a history, the cache adds almost nothing.** The folder store already holds 71 to 85 % of the definitions in changed files. The cache adds 0.17 % overall, and at most 1.8 % in one history.
> - **A fresh full worktree, or a rebuilt derived store, gets 86 to 89 % from the cache**, and leaves 0 to 2 % to compute. Small partial review copies get only 4 to 23 %.
> - **Size and lookup are small.** Everything ever seen comes to 16 741 distinct ids: 5 to 81 MB per fact kind in SQLite, depending on the value's size. A lookup takes 0.7 to 14 µs (p50).
>
> Two caveats. Julia ids were computed per file, while Moonkale resolves them module-wide. 40 % of Rust definitions got no id from the normal form, so they are not counted.
>
> **Part 2 is open:** the cost of an expensive tool (Lean elaboration). The threshold, and so the default, depend on it. Nothing was built; the cache file waits for Daniel (SR-126).
>
> Sources: [[SPEC-001 Sophia Store v0]] §8a; [[SPEC-009 Moonkale Folder Store]] (facts follow their files); [[Review 2026-11-20 Codex B-035 (external)]]; code: `crates/sophia-ingest/examples/fact_cache.rs` ([[fact_cache]]).

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

## Open: part 2, expensive facts

The threshold policy needs the cost distribution of a tool that is really expensive. Sophia has none (§8a). The planned candidate is Lean elaboration of Mathlib declarations:
- Mathlib `98a293fac8`, Lean v4.36.0-rc1;
- `lake env lean -Dtrace.profiler=true -Dtrace.profiler.threshold=0 FILE` prints an `[Elab.command] [seconds]` line per command;
- a first try on `Mathlib/Data/Nat/Factorial/Basic.lean` (573 lines) took 22 s and printed 187 731 lines, on a busy machine.

Then:
- profile a sample of files;
- take the per-declaration elaboration times;
- set the threshold against the lookup and write costs above.

## Proposal (expectation, until part 2 is measured)

The default would be **`threshold(t)` with `t` small (of the order of 1 ms)**, not 1 s.
- **A threshold against the frontend.** A miss costs microseconds. A fact that costs more than about 1 ms to compute, beyond the frontend, is worth a lookup many times over.
- **The fresh-folder case** is where the cache pays. A 1 s threshold would cache nothing from today's tools.
- **`all-content-only`** is just as defensible at these sizes. The difference between the two policies only matters once a tool emits many facts that cost microseconds. Part 2 decides.

The cache store itself is a new storage file, which needs Daniel's agreement (SR-126). Nothing was added.
