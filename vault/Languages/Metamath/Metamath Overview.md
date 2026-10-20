#overview

> Metamath in Sophia: the first real corpus (B-16 rung 1), and the first language whose analysis Sophia has **checked against an independent reference implementation**. 2026-10-20.
>
> Sources: Megill and Wheeler, *Metamath: A Computer Language for Mathematical Proofs* (2019); set.mm and iset.mm (github.com/metamath/set.mm); metamath-exe (github.com/metamath/metamath-exe, commit `9898f5d`, built from source); `crates/sophia-ingest/src/metamath.rs` ([[crates/sophia-ingest/src/metamath|metamath]]); [[Real Code Corpora]] §3–§4; `sophia-mm-check` ([[sophia-mm-check]]).

## Status by level

| Level | Status | Evidence |
|---|---|---|
| L1 Parsed | yes: the whole format (scoping blocks, `$c $v $f $e $d $a $p`, normal and compressed proofs), except file inclusion `$[ … $]` (refused) | set.mm (51 159 assertions) and iset.mm (17 398) parse |
| L2 Normalised | yes (`sophia-metamath@2`): a **statement** is its mandatory hypotheses in order, typecode, math string and disjoint-variable pairs, variables numbered by first occurrence; a **proof** is a separate record | renaming invariance tests; 12 927 of the 13 138 labels set.mm and iset.mm share are one statement |
| L3 Resolved | yes: labels are global and declared before use, so resolution is exact | — |
| L4 Lowered | stored as statements, proofs (evidence) and library collections (axioms, definitions, proofs); not translated into sophialang | — |
| L5 Executable | not applicable in the usual sense; **proof checking is not done by Sophia** | — |
| L6 Verified | **dependency analysis checked**: "which axioms and definitions does theorem T rest on" agrees with `metamath.exe` on every theorem checked | below |

## The cross-check (SR-128, 2026-10-20)

Sophia's answer is a query: the closure of a proof's references through proofs, filtered to the library's `axioms` (or `definitions`) collection. metamath.exe's answer is `show trace_back T /axioms /essential`. Compared by statement id.

| Library | Theorems checked | Agree | Dependencies compared |
|---|---|---|---|
| set.mm | 302 (seed 1, incl. `2p2e4`, `fta`) | 302 | 8 503 axiom, 24 471 definition |
| set.mm | 3 000 (seed 7) | 3 000 | 83 671 axiom, 242 975 definition |
| iset.mm | 1 001 (seed 3, incl. `2p2e4`) | 1 001 | 35 676 axiom, 48 995 definition |
| set.mm, iset.mm | **all theorems** | see [[Real Code Corpora]] §4 (run in progress when this page was written) | |

So the earlier claim "2 + 2 = 4 rests on 15 axioms in set.mm" is now checked: the same 15 (`ax-mp`, `ax-1` to `ax-9`, `ax-gen`, `ax-ext`, `ax-1cn`, `ax-addcl`, `ax-addass`) and 29 definitions.

## Storing is not checking

Sophia stores proofs and walks their references. It does **not** verify that each proof step is a valid substitution. A store that held an invalid proof would answer dependency queries about it just as confidently. Proof checking would be:
- **cheap** (metamath.exe verifies all 48 083 proofs of set.mm in 2.85 s, measured 2026-10-20), so it is a natural first **checker** for SPEC-006 §5's trust records: `checked by metamath.exe <version>`, or by a small Rust verifier in Sophia;
- the right place to test "evidence with trust levels" end to end.

Not done yet; listed in [[Semantic Boundary Experiments]] E-0.

## Limitations

- File inclusion (`$[ … $]`) is refused; set.mm and iset.mm do not use it.
- Being an axiom is a library's role, decided by the `$a` keyword and the label prefix (`df-` for definitions), as metamath.exe's own listings do.
- The store is about 11× the source ([[Current Inventory]]).
