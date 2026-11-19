#reference

> A defect-only review of B-076 (part 6 of [[Julia Conformance]], closures) by Mistral Large 4, 2026-11-18/19. Unlike B-074's run, this one executed its probes (`review-rust` agent: cargo allowed). Of its 4 verified findings, all 4 were right; 2 changed results (closure matching) and are fixed with a fixture case each that failed before the fix. Of 5 suspicions, 1 got a guard, 3 are documented limits, 1 (missing tests) is answered by the new cases. The LinearAlgebra counts did not change.
>
> Sources: the review's answer, `~/.cache/agent-usage/vibe/2026-11-18-2338-b076-review/ANSWER.md` (outside the repository; probes in `../Sophia-mistral/scratch/b076/pkgs/` and `../Sophia-mistral/crates/sophia-ingest/tests/zz_b076_*.rs`). code: `crates/sophia-ingest/src/conformance.rs` (`closure_ledger`), `tests/closures.rs`, `tests/julia-semantics/closures/src/Clo.jl`, `examples/julia_closures.rs`.

## How it was run

`vibe -p --agent review-rust --trust --max-price 1 --max-turns 100` (Mistral Large 4, cost $0.00) in `../Sophia-mistral` (branch `mistral/b073-review`, with B-071 to B-076 copied in uncommitted). The brief gave the two notes that specify part 6 as the requirement, the files, and asked for defects only, with an answer file kept probe by probe. It ran `cargo test` on hand-written packages and truth lines (Julia was not available to it). The answer file was written only at the end, but complete.

## Findings

| # | Claim | Right? | What was done |
|---|---|---|---|
| 1 | a `do` closure (Julia: at its body's line) is paired with another closure that starts on that body line, or left unmatched | **right**: fixture case `do_inner` (`Clo.jl:109`): before the fix both of Julia's closures merged into the arrow, and the `do` block's captured `b` was not compared | a `do` block's line is its body's first line when candidates are collected |
| 2 | two definitions with closures on one line: all closures are attributed to the wider definition | **right**: case `two_h(a) = …; two_k(b) = …` (`Clo.jl:113`): `two_k`'s closure was compared inside `two_h` | every outermost definition around the line is searched; when more than one has a closure there, Julia's line cannot tell them apart and the closure is counted as ambiguous (as parts 3 and 5 do) |
| 3 | `examples/julia_closures.rs` panics without its arguments | **right** (wrong usage, not wrong results) | usage message, exit code 2 |
| 4 | truth lines that are not 8 columns are dropped silently | **right** | counted (`ClosureLedger::malformed`) and printed; 0 on the fixture and LinearAlgebra |
| 5 | if a definition's text does not encode on its own, every local would be "own" (`None == None`) | not reproduced (no such definition found by it or in LinearAlgebra), but the path exists | guarded: such names are `Undecided` |
| 6 | closure names without `##N` would all pair with the first candidate | not a defect for `closures.jl`'s output (every closure name carries `##N`) | left; noted here |
| 7 | `closures_in` counts an anonymous function inside a quote as a closure | true, intended: an `@eval` template's closure is filed by Julia at the template's line | left; noted here |
| 8 | the `wider` fallback can pair a closure whose extra parameters are not Julia's | possible in principle; not exercised on the fixture or LinearAlgebra (0 unmatched, 0 ambiguous there before the fallback was needed) | left; documented in [[conformance]] ("else the only one whose parameters include them") |
| 9 | missing tests: findings 1 and 2, `Mixed`, several files, malformed truth | partly right | cases for 1 and 2 added; `the_comparison_reports_wrongs` covers the wrong outcomes; `Mixed` and several files not added |

Its "checked, no defect" list covered `##N` order, generators, keyword bodies, destructured arguments, the rename test (fresh-name collisions, multi-byte names, CRLF, boxing) and `spelled`.
