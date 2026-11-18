#reference

> A correctness review of B-014 (the set-oriented native executor: prefix access path, anti-joins, borrowed reads, the shared byte count) by Mistral Large 4 through the `vibe` CLI on 2026-11-18, in the helper worktree `../Sophia-mistral` (branch `mistral/b014-review`). Its shell commands were all refused in headless mode, so it compiled and ran nothing: every point is a line-level argument. It found no defect and two missing tests, now added.
>
> Sources: the review's answer file, kept outside the repository at `~/.cache/sophia-corpora/vibe/review-b014.md`. code: `eval.rs`, `plan.rs`, `tests/set_oriented.rs`, `sqlite.rs`, `kv.rs`, `duckdb_store.rs`.

## How it was run

```sh
vibe -p --prompt-file review-brief.md --agent review --trust --workdir ../Sophia-mistral --max-price 1 --max-turns 100
```

Cost: $0 (Mistral Large 4). The `review` agent's `cargo` and `git` calls were denied ("needs approval, which a headless run cannot give"), unlike the B-002 and B-003 runs; the cause was not investigated. The probes it was asked for were therefore not written.

## Points and triage

| # | point | right? | done |
|---|---|---|---|
| 1 | The prefix path is exact on every backend: byte order everywhere, no `\0` in valid paths so `last + "\0"` neither skips nor repeats, the namespace bound never cuts a match even when `rfind('.')` lands inside a backquoted segment | agrees with the tests (`prefix_range` on four backends, backquoted prefixes included) | nothing |
| 2 | The anti-join is equivalent when the unbound arguments are distinct variables; `!refs(x, x)` stays tuple at a time | agrees (unit test `anti_join_needs_distinct_free_variables`) | nothing |
| 3 | The shared byte counter is charged at every place that keeps tuples | agrees (`bytes_count_base_reads_inside_joins`) | nothing |
| 4 | No unguarded panic in the new paths | argument only | nothing |
| 5 | Grouping as solutions arrive equals `count(DISTINCT …)` per group | agrees (`sql_agrees`) | nothing |
| F1 | No test runs the limits (rows, steps, cancel) over the prefix path or an anti-join | right (a gap, not a defect) | `limits_hold_on_the_new_paths` added; passes |
| F2 | No negation whose only bound column is a constant; 3-column relations (`annotation`, `fact`) untested | right for the first, added (`!def(_, :nothing)`, `!def(_, :method)` in `QUERIES`); the test store has no annotations or facts, so the second stays open | partly |
| F3 | `agree`'s `startswith` assertion is weak for constant prefixes | right, but harmless: `agree` compares against tuple at a time, which keeps the `startswith` filter, so a superset would differ | nothing |
| F4 | `plan::order(…, Some(delta)).expect("checked")` is never exercised by the checker | not a B-014 change; pre-existing, and forcing a positive literal first only adds bindings | nothing |
| F5 | Nothing was compiled by the reviewer | right | gates run by Claude: `cargo test --workspace --all-features` 385 passed, 0 failed; clippy `-D warnings` and `fmt --check` clean |
