#review #external

> **Codex's review of B-029 (E-10, `lean_statements.rs`), 2026-11-20, defects only.** Thirteen findings, each shown by Codex with a probe (small Lean exports or edited lines). Seven are fixed. Two of them changed published numbers: axioms reached only through an inductive group were missed (the theorems resting on `Classical.choice` rise from 64.1 % to 66.0 %), and the duplicate sub-counts overlapped (they are now exclusive: 314 same proof, 70 derived, 455 independent). The group counts did not change. The other six concern inputs lean4export does not write, a stated rule of thumb, or a design choice; they are recorded as limitations.
>
> Sources: `codex exec -s read-only` with a brief naming the requirement and the files, no reasoning sent (`meta/scratchpad/review-brief-B-029.md`); its answer is `meta/scratchpad/REVIEW-from-codex-B-029.md`. code: `crates/sophia-ingest/examples/lean_statements.rs`, [[lean_statements]].

| # | finding | right? | what was done |
|---|---|---|---|
| 1 | a numeric name component `1` and a string component `"#1"` hash alike | **right** | the name hash tags string and numeric components. Mathlib's groups are unchanged |
| 2 | a theorem's key includes its universe parameter list, so equal types with lists `[]` and `[u]` split | right about the behaviour; **intended** | a declaration with an extra universe parameter is applied with one more level. The note now says so; it explains the one pair where `node` and `exact` differ |
| 3 | whitespace after a colon (`{"sort": 0,…}`) panics the reader | right for JSON in general | not fixed: lean4export writes compact lines. Stated in [[lean_statements]] |
| 4 | with `--export-mdata`, an `"expr"` key inside the metadata is read as the child | **right** | the child is now the last `"expr":` key. Mathlib's export has no `mdata` |
| 5 | the hash pass assumes children are numbered below their parents | right that it was assumed | now asserted; it holds on Mathlib (the run passes the assertion) |
| 6 | `natVal` `"01"` and `"1"` differ | right in principle | not fixed: lean4export prints canonical decimals. Stated |
| 7 | a chain of 150 000 `succ` levels overflows the stack | right | not fixed: Mathlib's levels are short. Stated |
| 8 | hand-written names such as `_manual` or `eq_17` count as generated | right; the rule was already called a rule of thumb | the note now names the misclassified cases |
| 9 | an alias chain through a generated member (`b → _g → a`) is not seen | **right** | the union now runs over all members. Checked on a small Lean export (`$SOPHIA_CORPORA/e10/negtest`, `Neg/Ind.lean`): `a`, `_g` and `b` are joined. Mathlib's counts are unchanged |
| 10 | the duplicate sub-counts overlap, "mentions another member" includes bare aliases, and there is no independent count | **right** | exclusive sub-classes, bare aliases excluded, an independent count: 314 / 70 / 455 (before: 314 and 113, overlapping) |
| 11 | axioms reached only through an inductive group (a constructor's type) are missed, because group edges entered only the fixpoint, which ran only on forward references | **right**, and it changed results | the propagation always iterates to a fixpoint with group edges (13 rounds on Mathlib). Checked on `Neg/Ind.lean`: `t : I → True` now rests on `X`. Mathlib: `propext` 88.0 → 89.0 %, `Quot.sound` 78.6 → 79.9 %, `Classical.choice` 64.1 → 66.0 %, no axiom 10.2 → 9.3 % |
| 12 | axioms after the 64th are dropped | **right** | now an assertion (Mathlib has 4) |
| 13 | no per-theorem attribution of axioms, and no total for the proof closure | a request, not a defect | not done: the tool reports counts. `groups.tsv` is the only per-item output |

Not added as regression tests: the tool is a measurement example, not library code, and has no test target. The probes for #9 and #11 are in `negtest/Neg/Ind.lean` under `$SOPHIA_CORPORA/e10/`.
