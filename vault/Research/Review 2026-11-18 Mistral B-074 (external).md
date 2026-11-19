#reference

> A defect-only review of B-074 (part 5 of [[Julia Conformance]], macros) by Mistral Large 4, 2026-11-18. The run could not execute anything (its approval callback refused every command), so all 12 points were traced from the code, with probe fixtures written but not run. Claude turned each into a fixture case and ran it: 6 reproduced and are fixed (each case failed before its fix and is kept in `tests/macros.rs`), 2 were not reproduced, 3 are limitations now documented, 1 was not a defect. Its "missing notes" point was out of date. The LinearAlgebra counts did not change.
>
> Sources: the review's answer, `~/.cache/agent-usage/vibe/2026-11-18-2138-review/ANSWER.md` (outside the repository; probes in `../Sophia-mistral/scratch/b074/`). code: `crates/sophia-ingest/src/conformance.rs` (`macro_ledger`), `tests/julia-semantics/macros.jl`, `tests/macros.rs`, `tests/julia-semantics/macros/src/Mac.jl`.

## How it was run

`vibe -p --agent review --trust --max-price 1 --max-turns 100` (Mistral Large 4, free on the EDU plan, cost $0.00) in `../Sophia-mistral` (branch `mistral/b073-review`, with B-071, B-073 and B-074 copied in uncommitted). The brief gave the requirement, the files and the commands, asked for defects only, and asked for an answer file kept probe by probe. It wrote that file (211 lines) but ran nothing: `cargo`, `julia`, even `echo` were refused. The probes were rebuilt by Claude as cases at the end of the fixture `Mac.jl` (lines 70 to 96), and the oracle's output was regenerated (`truth.tsv`).

## Findings

| # | Claim | Right? | What was done |
|---|---|---|---|
| 1 | a relative package directory compares no form (`src` canonical, file paths not) | **right**: `julia_macros tests/julia-semantics/macros …` printed "0 forms compared, 24 not found" | `macro_ledger` canonicalises `dir` first, as `world_ledger` does; test `relative_package_dir` |
| 2 | two forms on one line merge: the second form's macro overwrites the first's, its definition shows as `Open` | **right**: `@inline same_a(x) = 1; @noinline same_b(x) = 2` gave `same_b` `Open` under `@noinline` | such lines are not compared (`MacroLedger::ambiguous`), as part 3 does |
| 3 | `baremodule` bodies are never compared | **not reproduced**: Julia parses `baremodule` as `Expr(:module, false, …)`, which the oracle follows; the fixture's `Bm` compares | case kept (line 93) |
| 4 | the oracle drops a method whose first parameter is `Type{T}` with `T` a type variable; the definition shows as `NotMade` | **right**: `@inline (::Type{T})(c::Char) where {T<:Box} = T()` had no method line, ledger `NotMade` | the oracle prints such a method with owner `(::Type)` |
| 5 | methods of a literal `include` nested inside a form count for the enclosing form and show as `Open` | plausible, not built: no such form in the fixture or LinearAlgebra (0 unmatched forms, no `include` inside a macro call) | documented as a limitation in [[conformance]] and [[macros]] |
| 6 | blanking a non-ASCII macro argument writes one space per character, so later offsets in the file move and the id comparison reads wrong keys | **right**: after `@same2 "αβ" uni(x) = …`, `uni` became `IdSplits` and every later wrapped definition in the file a false `IdSeesMacro` | one space per byte |
| 7 | `@m(def)` blanks to `… def)`, a syntax error in the re-ingest | **not reproduced**: a parenthesised call has no `macro_argument_list`, so it is not blanked and not compared for its id; no error | case kept (line 80); the limitation is documented |
| 8 | a form that throws aborts the oracle; if caught, its methods would be dropped | right as stated, **not a defect**: the oracle fails loudly with Julia's error, so nothing is silently miscounted | documented |
| 9 | a docstring before a `module` moves the start of the next form, which is then unmatched | **right**: `after_dm` (line 89) was not found in Sophia's parse | a module consumes the docstring before it |
| 10 | a method replaced by a later form is still listed for its own form, so one current method counts twice | not a defect: each form did make its method; part 5 counts what forms made, not the final table (part 3's `current` filter answers a different question) | none |
| 11 | `@eval` with a splice outside a loop is compared for its id (`changed` against a bare definition that does not lower) | **right**: `@eval $(Symbol("direct"))(x) = 4` gave an id row | the oracle says `-` for `@eval` with `$`: a template, no bare definition to compare |
| 12 | a definition without a name matches methods of every function at its lines | right as stated; by design, since an unnamed definition (`$n() = …`, `(::Type{T})(…)`) has no owner to match; a wrong pairing needs a method of another function, made by the same form, at the unnamed definition's lines. Not seen in the fixture; not checked in LinearAlgebra | documented |
| — | `macros.md`, `julia_macros.md`, the part 5 sections of [[conformance]] and [[Julia Conformance]], the Code Map row are missing | out of date: written after the copy the review read | none |

After the fixes: `cargo test -p sophia-ingest`: 46 passed, 0 failed; clippy `-D warnings` on sophia-ingest clean. LinearAlgebra re-run with the fixed oracle: the same counts as before (834 forms, 1 753 methods, 202 `IdIgnoresMacro`; no line with two forms).
