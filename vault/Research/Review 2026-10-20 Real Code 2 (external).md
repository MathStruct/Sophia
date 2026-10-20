#reference #open-problem

> The fifth external review of 2026-10-20: ChatGPT on Sophia at `85189737` (Metamath fully checked, generic ownership, the storage and language sections, Reactant and Mathlib set up), passed on by Daniel. It answered the three open decisions and asked for stricter evidence on storage, AD artifacts and Lean. This note records each point and what was done. Requirements: SR-133 to SR-139 in [[Requirements]].
>
> Sources: the review text (conversation of 2026-10-20); the code and measurements named below.

## Decisions

| Decision | Review's answer | Done |
|---|---|---|
| Download Reactant | yes (already done) | — |
| Reduce storage overhead | yes, incrementally, with conformance and **reconstruction** tests | precondition built first (below); nothing dropped yet |
| S-ADR-0012, separate query IR | accept; share identity, types and evidence infrastructure with sophialang | **accepted** |

## Verdicts

| § | Point | Verdict | What was done |
|---|---|---|---|
| 1 | Do not drop the reference lists in records until the same logical record can be rebuilt and its identity verified; a regression test across all four backends | **Correct** | Nothing dropped. Reconstruction built and tested first: `source.rs`, `tests/reconstruct.rs` on four backends |
| 3 | Sophia does not keep the source of ingested files; before discarding canonical records it must keep an immutable source plus a **manifest** (hashes, paths, frontend and normaliser versions, environment) | **Correct, and the most important point** | Source files are content-addressed records; an ingest manifest records frontend, normaliser, package, and per file path, source id and include module. `sophia reconstruct` rebuilds **all 3 284 LinearAlgebra and all 98 136 set.mm records identically** from the store alone ([[Current Inventory]]). Environment beyond versions (Base bindings) is not in the manifest yet (SP-013) |
| 2 | E-AD-0's first call mixes transformation, compilation, execution and overhead; measure them apart | **Correct** | E-AD-1 splits a Reactant derivative's first call: session start-up 12.1 s, Julia compiling the tracer 1.89 s, the transformation itself 55 ms, XLA 34 ms, first run 15 ms ([[AD as a Materialization Case]]) |
| 2 | A fast second call in one process is not persistence; restore and run an artifact in a **fresh process** | **Correct** | Done. Reactant: executable + call glue + argument types saved, restored in a new process in 23 ms, **bit-identical gradient**, versus 12.6 s to compile there. Julia pkgimage route: an Enzyme gradient precompiled into a package image **segfaults** in a fresh process (the control works) |
| 2 | An artifact's identity depends on compiler versions, world, method tables, CPU features, rule registries, options | **Correct** | Confirmed in practice: the restored executable is bound to Julia, Reactant and Reactant_jll versions (checked on load) and to the host CPU; the glue is Reactant-internal. All belong in A (SPEC-006 §3) |
| 4 | For Mathlib, distinguish source syntax, elaborated terms, kernel declarations and proof objects; say what is verified and by which checker | **Correct** | [[Lean Overview]]: the four objects; four kinds of equality; **nothing in the export has been kernel-checked on this machine** (the `.olean` cache was built and checked on Mathlib CI) |
| 4 | Measure declarations, dependencies and proof terms first; definitional vs propositional equality | Agreed | Measured: 519 639 theorems, 187 268 definitions, 106 M shared expression nodes, **71 % needed only by proofs**, 54 138 theorems sharing a statement expression. Dependency counts and α-equivalence are next (E-10) |
| 4 | Order: finish Mathlib and Reactant; reconstructability; Moonkale's edit benchmark; cross-session artifacts; compact edges; then other compilers or the agent server | Agreed | [[Semantic Boundary Experiments]] and [[Milestones]] follow it |
| — | MIT permits proprietary reuse, which should be explicit | **Correct** | stated in the README's license section (as a fact about MIT, not as Daniel's reason) |

## What the review got right that the measurements sharpened

- **Where the time goes decides what to keep.** For this Reactant derivative the transformation is 55 ms and XLA 34 ms; Julia compiling code is 14 s of the session. So persisting the derived *program* would save little; persisting the *executable* saves almost everything, at the price of being host- and version-bound.
- **"Executable artifacts are hard to restore" is now an observation**, not a caution: one route works only by rebuilding Reactant's internal call glue, the other crashes.
- **Proofs dominate theorem libraries** (71 % of Mathlib's term graph), which makes statement-as-identity and proof-as-evidence, learned from Metamath, the central storage decision for Lean too.
