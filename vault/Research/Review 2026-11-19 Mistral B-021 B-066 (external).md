#reference

> A defect-only review of B-066 (`run_bounded` waits for end of file) and B-021 (proof trust records, E-0) by Mistral Large 4 through `vibe` (agent `review-rust`), 2026-11-19. One finding, right, fixed with a test that failed before the fix; every other probe confirmed the requirement. Mistral's `review-rust` allowlist worked this time: the brief listed the allowed command prefixes and forbade `cd`, pipes and `&&`.
>
> Sources: Mistral's answer, `~/.cache/agent-usage/vibe/2026-11-19-b021-b066-review-ANSWER.md` (outside the repository); its scratch probes stayed in `../Sophia-mistral-b021`. code: `crates/sophia-query/src/bounded.rs`, `tests/bounded.rs`; `crates/sophia-ingest/src/trust.rs`, `tests/trust.rs`.

## How it was run

The diff and the two requirements (not Claude's reasoning) in a fresh worktree `../Sophia-mistral-b021` holding the whole uncommitted state; Mistral wrote scratch tests and ran them.

## Findings

| # | Finding | Right? | What was done |
|---|---|---|---|
| 1 | A worker that forks a child inheriting its pipes and then hangs: the kill reaches only the worker, the child keeps the pipes open, and the caller blocks forever joining its reader threads, so `Limit` never comes (older than B-066, but against its stated guarantee) | **right**: reproduced with `a_grandchild_holding_the_pipes_does_not_hold_the_caller` in `tests/bounded.rs` (failed: still waiting after 10 s) | fixed: after a kill `run_bounded` returns without joining the readers, and it never joins the writer; such threads end when the last holder of the pipe exits. The test passes (`cargo test -p sophia-query --all-features --test bounded`: 8 passed) |

## Observations (not defects), with what was done

- `metamath_verify_failures` misses a label wrapped onto the next line: the bench sets `set width 100000`, as the note says; no change.
- The version is read from metamath.exe's banner: checked against the binary by Claude (`Metamath - Version 0.200.pre …` → `0.200.pre`, the E-0 run).
- A proof whose reference is missing from the store counts as untainted: not reachable through `store_metamath` (an unknown label is a parse error); noted, no change.
- `sophia-mm-trust` parses set.mm twice: performance only; no change.

## Confirmed by its probes

Event-driven return (a worker holding its pipes 500 ms returns at 502 ms with a 2 s grace); `deadline == None` waits for the answer; a garbage response is `Codec`; a repeated theorem's shared proof has one annotation, failed if either label failed; taint is transitive and the two queries partition the proofs; a `check-failed` statement does not taint; error-line edge cases; a re-check with other inputs keeps one annotation.
