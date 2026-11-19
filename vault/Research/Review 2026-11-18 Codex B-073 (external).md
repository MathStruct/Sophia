#reference

> A defect-only review of B-073 (part 4 of [[Julia Conformance]], world change) by Codex (GPT), 2026-11-18. Mistral was asked first and wrote nothing in 33 minutes, so it was stopped. Codex reported 3 verified findings, each with a probe: all 3 right, each shown first by a fixture case that failed, all fixed. Fixing the third found a fourth defect, in the oracle itself.
>
> Sources: the review's answer and probes, kept outside the repository at `~/.cache/sophia-corpora/review-b073/codex/`. code: `crates/sophia-ingest/src/conformance.rs`, `tests/julia-semantics/calls.jl`, `tests/world.rs`.

## How it was run

First `vibe -p --agent review --max-price 1 --max-turns 100` (Mistral Large 4, free) in `../Sophia-mistral` (branch `mistral/b073-review`, B-071 and B-073 applied uncommitted). It wrote no answer file in 33 minutes and was killed. Then `codex exec -C ../Sophia-mistral -s workspace-write` with the same brief: the requirement, defects only, an answer file kept probe by probe. It built, ran the tests (7 passed), wrote probe packages and ran Julia for them.

## Findings

| # | Claim | Right? | What was done |
|---|---|---|---|
| C1 | A change under `src/` that `include`s another file: Sophia appends only the change, not the file it includes, so a method added there is missed (`g → f` reported `MadeStale`) | **right** | fixture `changes/via_include.jl` → `changes/more/solo_str.jl`; the test failed (`MadeStale`, no id moved). The change is now read with `module::discover`, its files put in the package's module, and their own rows skipped |
| C2 | A relative package directory with an absolute change path changes the result (`call_sum`: `MadeStale` instead of `Followed`): the change's file name relative to `src/` no longer matched Julia's | **right** | test `the_result_does_not_depend_on_how_paths_are_written` failed; `world_ledger` and `dispatch_ledger` now canonicalise the paths |
| C3 | When another package replaces a method, Julia lists no use there and the row was counted `Followed`, though Sophia's claim did not change | **right** in the code (a missing side was `Followed`) | new outcome `Unchecked` for a pair with no Julia row on one side. Its fixture (`outside/redefine_call_solo.jl`) did not fail the way the probe did, because of C4 |
| C4 | (found while fixing C3) `calls.jl` listed a method another one had replaced: `Base.visit(Core.methodtable)` still visits it in Julia 1.13 | right | `calls.jl` now skips a method `m` with `which(m.sig) !== m` (checked by hand: the replaced method has `dispatch_status` 0 and is not in `methods(f)`); all truth tables regenerated, unchanged except the replacing case |

Also checked by the review and passed: a change path with `./`, no double counting in `dependants` (repeated uses, a diamond).
