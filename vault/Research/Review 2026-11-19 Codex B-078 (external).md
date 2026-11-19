#reference

> A defect-only review of B-078 (part 7 of [[Julia Conformance]], mutable globals) by Codex (GPT), 2026-11-19, after a Mistral run that produced no answer. Of 8 findings, 6 were right and are fixed, each with a fixture case or a test that failed before the fix (5 in `global_ledger`, 1 in the oracle `globals.jl`); 1 was wrong (the escapes it read in the test); 1 was a mismatch between the spec text and the intended scope, answered by correcting the text. The LinearAlgebra counts did not change.
>
> Sources: Codex's answer, `~/.cache/agent-usage/codex/2026-11-19-b078-review.md` (outside the repository). code: `crates/sophia-ingest/src/conformance.rs` (`global_ledger`), `tests/globals.rs`, `tests/julia-semantics/globals.jl`, `tests/julia-semantics/globals/src/Glob.jl`.

## How it was run

First Mistral Large 4 (`vibe -p --agent review-rust`, `../Sophia-mistral`, 100 turns, $0.00): 57 of its tool calls were refused in headless mode (commands outside the agent's allowlist), it hit the turn limit and wrote no answer file. Then `codex exec -s read-only` in the main checkout, with the requirement (the part-7 section of [[conformance]] and [[globals]]), the diff and the LinearAlgebra truth; no build, so every finding was checked here by a test.

## Findings

| # | Finding | Right? | What was done |
|---|---|---|---|
| 1 | `tests/globals.rs:145`: the falsified-truth helper joins with a literal `\t` | **wrong**: the file has real tab and newline escapes; `the_comparison_reports_wrongs` would fail otherwise | nothing |
| 2 | a use at a line two definitions share is `ambiguous`, but both definitions' content references then come out as `NoJuliaUse` | right: `amb_a() = LIMIT; amb_b() = LIMIT` gave two `NoJuliaUse` | the definitions of an ambiguous use count as matched for that binding; case at `Glob.jl:96` |
| 3 | "innermost" by shortest line range picks one of two definitions that start on one line without nesting (`amb_f() = LIMIT; function amb_g() … end`) | right: `amb_f` got `KnowsConstant`, `amb_g` `NoJuliaUse` | innermost = the candidates containing no other (by bytes); several are `ambiguous`; case at `Glob.jl:98` |
| 4 | a generic use is matched to a definition by lines, so it leaks into a definition that ends on the line where the next starts | right: `function gen_f(mode2) mode2 end; gen_g() = mode2` gave `WrongLocalAsGlobal` for `gen_f` | matched by bytes inside the definition; `gen_f` is now `KnowsLocal`; case at `Glob.jl:102` |
| 5 | the binding table counts refused definitions | right: `const BAD = set_counter(1)` was `KnowsConstant`, `badlevel::Int = set_counter(1)` `WrongContentOfMutable`, though neither has an id | only identified definitions count: `OpenConstant` and `KnowsState`; cases at `Glob.jl:106-107` |
| 6 | truth lines are checked by column count only: an unknown access, a non-numeric line, a use whose kind contradicts its binding's, a binding listed with two kinds were accepted | right (shown by the new test) | each is `malformed` and not compared; `malformed_truth_is_not_compared` |
| 7 | `globals.jl` reports mutable globals bound to functions, types or modules, which the spec excluded | the code was intended, the text wrong: a mutable global's value is state whatever it holds (LinearAlgebra's `ClosedReal` holds a type) | the spec text in `globals.jl`, [[globals]], [[conformance]] and `BindingRow` now says: constants whose value is not a function, type or module, and every mutable global |
| 8 | `mutable_object` called every `AbstractString` immutable | right: a `mutable struct Text <: AbstractString` held by a constant printed `immutable` | only `String` is excepted; case at `Glob.jl:109-113` (`ApproxMutableObject`) |

Gates after the fixes: `cargo test -p sophia-ingest --test globals` 4 passed; LinearAlgebra's ledger unchanged (45 bindings, 114 uses, 1 wrong).
