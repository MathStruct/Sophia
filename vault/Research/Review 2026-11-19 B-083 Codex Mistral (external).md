#reference

> Defect-only reviews of B-083 (Julia conformance part 9, include order; `discover` reads a file once per module) by Codex (read-only, `codex exec -s read-only`) and Mistral Large 4 (`vibe`, agent `review-rust`), 2026-11-19. Codex: four findings, all right, fixed, each with a test that failed before the fix (two Codex had verified itself, two were guesses that reproduced). Mistral: its shell was refused in the headless run, so it ported the comparison to TypeScript and checked it there; three findings about edge lines of the truth, the first right (fixed with a failing test), the second already fixed by Codex's 3 and 4, the third a test gap now covered; a fourth (vault not updated) was out of date.
>
> Sources: the answers, `~/.cache/agent-usage/vibe/2026-11-19-1308-review-b083/ANSWER.md` (Mistral) and `REVIEW-from-codex.md` there (outside the repository). code: `crates/sophia-ingest/src/conformance.rs` (`include_ledger`), `src/module.rs` (`discover`), `tests/includes.rs`, `tests/julia-semantics/includes.jl`.

## How it was run

The diff and the requirement (not Claude's reasoning) went to both reviewers. Codex read the main checkout read-only, so it could run Julia expressions but not cargo (the Nix cache lock is on a read-only file system). Mistral worked in a fresh worktree `../Sophia-mistral-b083` holding the uncommitted state. Every shell command was refused there (the brief did not list the allowed command prefixes, as [[Review 2026-11-19 Mistral B-021 B-066 (external)]] had found necessary), so it checked a TypeScript port of the comparison instead.

## Findings

| # | Finding | Right? | What was done |
|---|---|---|---|
| Codex 1 | `includes.jl`'s `relfile` cut the prefix by `length` (characters) while Julia indexes strings by bytes: a non-ASCII SRCDIR gives wrong keys or an index error | **right** (Codex ran the expression) | `chopprefix`. Checked with a package under `…/é/src`: keys right; the fixture's and LinearAlgebra's truth unchanged (`diff`) |
| Codex 2 | Methods of a file Julia evaluated outside `src/` were dropped by the oracle | **right** | the oracle keeps methods of every evaluated file, keyed as in its include line. Checked on a package that includes `../shared.jl`: the method is placed (`Knows`), the file `ApproxPath` |
| Codex 3 | A method in an undiscovered file took the file's outcome, so a method in a module with a dynamic include was `WrongMissed` when another module evaluated the file without one | **right**: `a_method_is_open_by_its_own_module` failed (`WrongMissed`) | the method's own module decides |
| Codex 4 | One file reached by a symbolic link and by its name: Sophia reads it once, the ledger keyed the spellings apart, so the second was `WrongMissed` | **right**: `a_symbolic_link_is_the_same_file` failed (3 rows, not 2; the test's truth is Julia's own output for that package) | keys go through the canonical path where the file exists |
| Mistral 1 | A method in `Main` (top-level code of the root file outside `module`) has module `""`, rejected as malformed | **right**: `edge_lines_of_the_truth` failed | the oracle prints `-` for `Main`, as include lines do; the ledger reads `-` as the root module |
| Mistral 2 | A method line whose file is not in normal form, or that is in no include line, is misclassified | already fixed by Codex 3 and 4 (canonical keys; the method's own module); covered by `edge_lines_of_the_truth` | — |
| Mistral 3 | No test for a method at a line no definition covers (`methods_outside`), or for a duplicate `seq` | right (a test gap) | both in `edge_lines_of_the_truth` |
| Mistral 4 | The vault was not updated | not right: the vault notes were written after the diff was taken | — |

Mistral's other probes found nothing: the classification and order of every fixture row (on its port), panics, include cycles (a cycle that nests modules deeper each round stops at `MAX_MODULE_FILES` with `Limit`), `..` above `src/`, no change for a package that includes no file twice, and the other caller of `discover` (`store_module`).
