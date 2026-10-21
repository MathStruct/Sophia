#open-problem

> **Status: open (2026-10-21).** Found while checking the Rust-profile fix against `sophia gen` stores. Related: [[SPEC-005 Rendering Profiles]] §5 (rule 4: text in an editable profile parses back), [[lexer]] (`Limits`), SR-50.

**Problem.** The parser refuses text nested deeper than `Limits::max_depth` (128, Moonkale WP-66's limit, SR-50), counted as nested `expr` calls: every `let` value and body, lambda body, argument and parenthesis adds one. Nothing bounds the depth of a term that enters the store by another path (`normalise_batch`, `sophia gen`, a future ingest). Such a definition prints, but its text does not parse back, so Moonkale can show it and not save it, even unchanged.

**Measured** (2026-10-21, release build): print each definition in the `julia` profile and save the text unchanged with `sophia edit`.
- `sophia gen 50` (seed 42): 7 of 50 refused with `nesting deeper than 128`, the same 7 in the `rust` profile; the build before 2026-10-21 refuses them too (checked on `gen.m0000036`).
- B-15's `gen1000` store, its first 300 definitions: 22 refused.
- B-15 edited `m0000001`, which parses, so its numbers are unaffected.

**Options.**
1. **Raise the limit** for Sophia's own texts (the parser runs on a 64 MiB stack, [[sophialang_syntax]]), keeping 128 only where WP-66 compatibility needs it. Moves the edge, does not remove it.
2. **Bound depth on entry**: refuse, or mark, records whose printed nesting exceeds the parse limit, so the store holds only editable definitions (or knows which are not).
3. **Make the generator realistic**: B-14/B-15's synthetic programs nest deeper than human code does; cap their depth. Fixes the benchmark data, not the invariant.

**Proposal.** 2 and 3: the invariant "an editable profile's text parses back" should hold for everything stored, and the generator should not produce code no person writes. Not implemented.
