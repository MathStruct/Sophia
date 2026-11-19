#reference

> A defect-only review of B-071 (part 3 of [[Julia Conformance]], dispatch) by Mistral Large 4 through the `vibe` CLI (`review` agent, 2026-11-18). Every shell call it tried was refused, so the review is from reading only. 6 findings and 4 suspicions: 2 right and fixed, each shown first by a fixture case that failed; 1 right as a mislabel, fixed; the rest not reachable or by design.
>
> Sources: the review's answer and its probe fixtures, kept outside the repository at `~/.cache/sophia-corpora/vibe/review-b071/`. code: `crates/sophia-ingest/src/conformance.rs`, `tests/julia-semantics/calls.jl`, `tests/dispatch.rs`.

## How it was run

`vibe -p --agent review --max-price 1 --max-turns 100` in `../Sophia-mistral` (branch `mistral/b022-review`, the diff applied, uncommitted), with a brief giving the requirement and asking for defects only. Free model (cost 0.0). The agent's shell was refused in this session, so it could not run Julia, the test or its probes; Claude ran the cases below.

## Findings

| # | Claim | Right? | What was done |
|---|---|---|---|
| V1 | Two definitions on one line: a Julia method (which has only a line) is given to the first, the other loses its claims | **right** | fixture `same_a(x) = 1; same_b(x) = solo(x)`: `same_a` got `same_b`'s use (test failed). Such a line is now not attributed and counted (`julia_lines_ambiguous`; 3 in LinearAlgebra) |
| V2 | Claims are matched to Julia's uses by name only: `h(x) + Inner.h(x)` can turn a right claim into `WrongTarget` | **right** | fixture `both(x) = h(x) + Inner.h(x)`: `both → h` was `WrongTarget` (test failed). Claims now match by binding, then by name; open uses are filtered by binding |
| V3 | `calls.jl` skips `none:0` closures, against its comments | wrong | `relfile(…) === nothing \|\| continue` skips only closures that have a package line (they are printed on their own); the generator case `call_in_generator` passes and needs exactly this path |
| V4 | `julia_methods_outside` counts lines, not methods | right (a label) | renamed `julia_method_lines_outside`, documented |
| V5 | A content reference to an unnamed (qualified-head) definition is dropped | not reachable | the crate's `scope` holds only named units, so no reference targets an unnamed one |
| V6 | Missing cases: multi-line definitions, same-line, name collision | right | fixture cases `multi`, `same_a`/`same_b`, `both` added to `tests/dispatch.rs` |
| S1 | A generic use of an imported, extended Base function would be `WrongTarget` | not reachable | imported names are `blocked` in the crate, never a use |
| S2 | `nmethods` counts methods of other loaded packages | by design | a method another loaded package adds does make a single-definition reference name one of several; the LinearAlgebra run loads only LinearAlgebra |
| S3 | Module, constant and undefined uses are not reported as open | by design | the ledger reports open functions and types (the dispatch question); documented in `conformance.md` |
| S4 | `ReturnNode` and similar statements are not walked | not reachable as far as seen | Julia 1.13's lowering loads every global into a statement of its own (`%1 = Disp.Inner.h`, then `return %2`), which the script reads; seen in every lowered body printed while building, not proven for all forms |
