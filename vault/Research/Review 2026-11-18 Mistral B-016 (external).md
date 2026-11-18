#reference

> A correctness review of B-016 (SP-010 step 1: relation epochs and per-read-set query snapshots) by Mistral Large 4 through the `vibe` CLI (`review-rust` agent) on 2026-11-18, in the helper worktree `../Sophia-mistral` (branch `mistral/b016-review`, the uncommitted diff applied). It ran its probes: 8 tests, all passing. It found no defect; one deviation from the old wording of SP-010 is intended and now tested. Its probes are kept as `crates/sophia-query/tests/review_mistral_b016.rs`.
>
> Sources: the review's answer file, kept outside the repository at `~/.cache/sophia-corpora/vibe/review-b016.md`. code: `crates/sophia-store/src/logic.rs`, `types.rs`, `folder.rs`; `crates/sophia-query/src/sophia_query.rs`, `eval.rs`, `magic.rs`; `tests/execution.rs`, `tests/review_mistral_b016.rs`.

## How it was run

```sh
vibe -p --prompt-file scratch/b016/BRIEF.md --agent review-rust --trust --workdir ../Sophia-mistral --max-price 1 --max-turns 100
```

Its sandbox refused `CARGO_TARGET_DIR=…`, so it built into that worktree's own `target/`.

## Points

| # | point | right? | what was done |
|---|---|---|---|
| 1 | Every base-data write in `logic.rs` and `folder.rs` is logged under an op whose `relations()` covers it | right (no defect) | its probes `every_op_invalidates_its_readers_without_stale_rows`, `folder_replace_files_invalidates_its_readers`, `collected_definitions_invalidate_their_readers` kept as tests |
| 2 | Every store read of the executor, including through the magic-set rewrite, is covered by `Prepared::reads` | right (no defect) | `magic_query_reads_only_its_own_relations` kept |
| 3 | Method changes rewrite the nominal row's version without bumping the `nominal` epoch, against SP-010's old wording ("`member` and `nominal` on method changes") | a deviation, not a defect: `nominal(n, kind)` does not show the version (SPEC-007 §2), so no query result changes | intended; SP-010 now lists `member` only; the test, renamed `method_changes_keep_nominal_results`, says that exposing the version needs `nominal` added to the method ops |
| 4 | Eviction leaves only current entries in the current layout, and removes older and malformed keys | right (no defect) | `eviction_leaves_only_current_layout_and_snapshots` kept |
| 5 | A store without epoch keys never gives a stale hit, even with old-layout entries planted under the old result ids | right (no defect) | `pre_epoch_store_never_stale_hits` kept |
| 6 | Unverified: a writer committing between `run_cached`'s read and write transactions leaves an entry under a snapshot already stale; never hit (epochs only grow), evicted on the next miss | right, and older than this change (SPEC-007 §4.3 documents it) | nothing |
| 7 | Unverified: `ensure_store_id` races between two first runs | right, older than this change, no wrong result | nothing |
