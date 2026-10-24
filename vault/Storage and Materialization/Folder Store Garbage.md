#measurement #design

> How much of a folder store ([[SPEC-009 Moonkale Folder Store]]) no current file reaches after real history, and a proposal for removing it. Measured on Moonkale's replays of its own history (471 commits) and Lenticulum.jl's (130). **About 60 % of a replayed store is garbage:** old source texts, old file manifests and old definition versions. Removing them brings a replayed store to within 6 % of a fresh build. Part of [[Garbage Collection]]. 2026-10-24.
>
> Sources: [[Garbage Collection]] (roots, reachability, safety); [[Storage Measurements]]. code: `crates/sophia-store/examples/folder_garbage.rs`; the stores from Moonkale's `history_replay` (Moonkale `27c0face`, `MOONKALE_REPLAY_KEEP`), Sophia `def22b0c`, SQLite.

## Method

Moonkale replayed each history into a folder store, one `replace_files` batch per commit, and built a fresh store from the last tree. Its checkpoint comparison found 0 differences. The copies are in `~/.cache/sophia-corpora/folder-replay/` (README there). I measured copies of those copies:

```sh
cargo run --release -p sophia-store --example folder_garbage -- moonkale.sqlite moonkale.ids.tsv
sqlite3 moonkale.sqlite "SELECT name, sum(pgsize) FROM dbstat GROUP BY name ORDER BY 2 DESC"
```

[[folder_garbage]] puts each record in one of three states:
- **live**: reached from a current file manifest, a name or a current generic member, then through `refs`;
- **lineage-only**: an earlier version in a live record's lineage;
- **unreached**: neither.

The simulated collection deletes the non-live records' rows from `t_definition`, `t_ref` and `t_nominal_ref` on a copy, then runs `VACUUM`. The script is `~/.cache/sophia-corpora/scratch/gc/sim.sh`.

**Check:** the live set of each replayed store is the fresh store's record set. No fresh id is missing, and record counts are equal: 5940 and 1770. 79 and 14 records with equal ids differ only in `display`, which is not hashed: when two files have identical text, one source record serves both, and its display keeps the path that was written first.

## Results

Payload bytes = canonical bytes + display + 32 per reference.

| | Moonkale replayed | Moonkale fresh | Lenticulum replayed | Lenticulum fresh |
|---|---|---|---|---|
| records | 11 541 | 5 940 | 2 775 | 1 770 |
| live: records / payload MB | 5 940 / 22.6 | 5 940 / 22.6 | 1 770 / 6.3 | 1 770 / 6.3 |
| unreached: source texts | 1 847 / 29.8 | — | 257 / 3.9 | — |
| unreached: file manifests | 1 986 / 5.5 | — | 310 / 1.4 | — |
| unreached: definitions | 1 034 / 5.6 | — | 412 / 2.3 | — |
| lineage-only definitions | 734 / 3.6 | — | 26 / 0.05 | — |
| log entries | 33 757 | 18 998 | 7 793 | 5 359 |
| file after `VACUUM` | 91.2 MB | 36.2 MB | 19.9 MB | 9.8 MB |
| unreached removed | 42.8 MB | | 10.2 MB | |
| unreached and lineage-only removed | **38.4 MB** | | **10.1 MB** | |

**Observations:**
- **Source texts are most of the garbage**: 73 % of Moonkale's unreached payload. Every change to a file stores a whole new copy of its text, and Moonkale's files are large: 481 live sources hold 6.2 MB.
- **Growth: about 120 KB per commit on Moonkale**: (92.6 − 36.8) MB / 471 commits. In the editor, `replace_files` runs on every save, not every commit, so a working day adds more than a commit does. That is an expectation, not measured.
- **What remains after collection** is about 2 MB on Moonkale (38.4 against 36.2 MB):
  - the longer log (t_log is 3.7 MB);
  - closed relation rows;
  - retired nominal records.
- **No facts** are about non-live records: folder facts follow their files.

## Proposal (not implemented; needs Daniel's agreement, SR-126)

1. **Incremental collection inside `replace_files`**, in the same transaction. The seeds are what the batch's old manifests referenced: the old manifest itself, its source, and its records. A seed is removed when none of these hold:
   - it is a current file manifest (`file/` values);
   - a name targets it (`names_of`);
   - it is a current member of a generic function (`memberships_of`);
   - a stored record references it (`dependants`, the existing `t_ref` reverse index);
   - a fact is about it.

   A removed record's `refs` become seeds in turn. This is reference counting over indexes that already exist. It adds no table and changes no layout: rows are deleted. It needs one new `WriteTx::remove_record` on all four backends. Each removal is logged, as [[Garbage Collection]] asks.
2. **A one-off sweep, `Store::collect_folder()`**, which computes what [[folder_garbage]] computes, for stores written before (1). It runs in one write transaction.
3. **Only in derived mode.** In a store that is not derived, the files are not the truth, so nothing changes there.
4. **Lineage history: Daniel's choice.** In a folder store, git already keeps the history.
   - (a) Collect lineage-only records and keep their lineage entries as ids without bodies. The text of an old version is then fetched from git.
   - (b) Keep lineage history as a root. Measured cost: +11 % on Moonkale (42.8 against 38.4 MB).

   Recommendation: (a). Moonkale's editor reads old versions from git, not from Sophia.
5. **The log is left as it is** for now. It is 4 % of the replayed store. Compaction ([[Garbage Collection]] § The log) is a separate step.

**Expected effect:** a folder store stays the size of a fresh build plus its log, whatever the history. The tests would compare a replay with collection against a fresh build: equal record sets, on all four backends.
