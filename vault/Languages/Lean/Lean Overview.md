#overview

> Lean 4 in Sophia: planned as B-16 rung 5 (Mathlib). **Set up 2026-10-20** (`sophia-mathlib`, flake `.#lean`): Mathlib `98a293f` with its prebuilt cache (7.2 GB) and lean4export built at Mathlib's toolchain `v4.36.0-rc1`, in `$SOPHIA_CORPORA`. No export or ingest yet. Daniel named "Metamath and then Lean for theorem libraries". 2026-10-20.
>
> Sources: [[Real Code Corpora]] §2; `lean4export` (the kernel-term export format used by independent checkers such as lean4lean and nanoda); de Moura and Ullrich, *The Lean 4 Theorem Prover and Programming Language*, CADE 2021; [[Proof Assistant]]; [[Dependent Types]].

| Level | Plan |
|---|---|
| L1 Parsed | read **kernel terms** from `lean4export`, not Lean source: elaboration (tactics, type classes, coercions, macros) is Lean's business, and the kernel term is the meaning |
| L2 Normalised | hash kernel terms (de Bruijn binders, universe levels, constants by name → by id), Merkle over the constant dependency graph |
| L3 Resolved | exact at the kernel level: every constant is declared |
| L4 Lowered | close to sophialang's dependent core; a translation of a fragment is plausible |
| L5 Executable | not the point; Lean's compiler is |
| L6 Verified | checking kernel terms with an independent checker is the established practice; Sophia would record the checker as trust (SPEC-006 §5) |

**What it would test:** dependent types at Mathlib's scale (hundreds of thousands of constants); proof terms as evidence separate from statements, as Metamath taught (theorem statements are identity, proofs evidence); definitional unfolding and its cost; the gap between source (tactics) and kernel term (a derived program, input-addressed by the elaborator and its version: the same pattern as AD, [[AD as a Materialization Case]]).

## Four objects, kept apart

Lean, like Metamath, separates what is stated from how it is justified, and adds elaboration between source and kernel. Sophia must not merge any two of these:

| Object | What it is | Where it comes from | Checked by | In Sophia |
|---|---|---|---|---|
| **source syntax** | what a person wrote: tactics, notation, implicit arguments, `simp` calls | `.lean` files | nothing (it is authored) | not read; the authored object ([[Object Categories]] 1) |
| **elaborated term** | the source after elaboration: implicit arguments filled, type classes resolved, tactics run, coercions inserted | the elaborator, a program with a version (the toolchain) | nobody checks the elaborator; its output is checked by the kernel | a **derived program**, input-addressed by (source, elaborator version, imports), like an AD derivative ([[AD as a Materialization Case]]) |
| **kernel declaration** | name, universe parameters, type (the **statement**), and for a definition its value | the elaborated term, as sent to the kernel | Lean's kernel, when the declaration was added | what `lean4export` exports; the identity of a theorem should be its **type**, as for Metamath |
| **proof object** | a theorem's value: a term of its type | elaboration of the proof | the kernel, once; afterwards irrelevant to the statement's meaning (proof irrelevance) | **evidence** (SPEC-002 §3), separate from the statement |

**Equalities, kept apart too.** Two statements can be:
- *syntactically equal* (the same expression; what the measurement below counts);
- *α-equivalent* (equal up to binder names; lean4export also ignores binder annotations, so its node identity is this level or coarser, see §E-10);
- *definitionally equal* (the kernel can show it by computation: needs the kernel);
- *propositionally equal or equivalent* (someone proved `A ↔ B`: a theorem, i.e. evidence, with a proof).

Only the first two can be identity in Sophia. Definitional equality is a check with a cost and a trusted kernel; propositional equivalence is a theorem like any other.

## What is verified, and by whom (2026-10-20)

- The Mathlib build cache (`.olean` files) was **downloaded**, built by Mathlib's CI. Their declarations were checked by Lean's kernel **on CI**, not here. Importing `.olean` files does not re-run the kernel.
- `lean4export` reads those `.olean` files. So **nothing in the export has been kernel-checked on this machine.**
- **Re-checked here on 2026-11-20** (E-10, below). Before that:
- Re-checking is possible and is the established practice: `lean4checker` replays declarations through Lean's own kernel; independent kernels (lean4lean, nanoda) check an export. Until one of them runs, the right trust record for Mathlib in Sophia is `checked by Lean kernel v4.36.0-rc1, on Mathlib CI, not re-checked` (SPEC-006 §5). This is experiment E-0's Lean analogue.

## Measured: the Mathlib export (2026-10-20)

`lean4export` 3.1.0 on Mathlib `98a293f` (Lean v4.36.0-rc1), all of `Mathlib` and its dependencies (Batteries, Aesop, Lean's core): **401 s, 5.9 GB, 111 M lines.** Measured by `examples/lean_export_stats.rs` ([[lean_export_stats]]) in 35 s, 1.6 GB peak:

| | Count |
|---|---|
| theorems | 519 639 |
| definitions | 187 268 |
| inductive groups | 7 107 |
| opaque constants | 2 978 |
| quotient declarations | 4 |
| axioms | 4: `propext`, `Quot.sound`, `Classical.choice`, `sorryAx` (declared in core; whether any proof uses `sorryAx` is not checked yet) |
| expression nodes (shared DAG) | 106 260 410 (85 % applications) |

| Part of the expression graph | Nodes | Share |
|---|---|---|
| reachable from statements (all declaration types) | 22.8 M | 21.5 % |
| reachable from definition values (programs) | 12.2 M | 11.4 % |
| reachable from proofs | 89.3 M | 84.0 % |
| **needed only by proofs** | **75.7 M** | **71.2 %** |
| needed only by definition values | 7.3 M | 6.8 % |

**What it says.**
- **Proofs are most of a theorem library**: 71 % of Mathlib's term graph exists only to justify statements. That is the Metamath lesson (proofs are evidence, not identity) at 50 times the scale, and it is the materialisation question in its sharpest form: the proofs were checked once (on CI); a store that keeps statements and programs needs under a third of the graph, and the proofs can be kept as evidence or re-fetched and re-checked.
- **Repeated statements:** 54 138 theorems share their exact statement expression with another theorem (16 423 groups). Syntactic equality including binder names, so a lower bound on α-equivalent repeats; mostly, presumably, aliases and generated lemmas (not yet inspected). Content addressing by statement would make each group one identity with several proofs, as in Metamath.
- The export (5.9 GB of text) is a derived form: Mathlib's own `.lean` sources are 99 MB (its dependencies' sources not counted), so the export is about 60× larger. The same source-versus-derived question as [[Current Inventory]].

## Measured: E-10, repeated statements, dependencies and a local re-check (2026-11-20)

[[Semantic Boundary Experiments]] §E-10, B-029. The same export (Mathlib `98a293f`, Lean v4.36.0-rc1, lean4export 3.1.0). Statements and dependencies come from `examples/lean_statements.rs` ([[lean_statements]]: 55 s, 5.1 GB peak; reviewed by Codex, [[Review 2026-11-20 Codex B-029 (external)]]). The re-check is `experiments/lean-recheck/recheck.sh` ([[recheck]]). Outputs are in `$SOPHIA_CORPORA/e10/`.

**The export is already α-quotiented.** lean4export shares subterms through a hash map keyed by Lean's `BEq Expr`, which is `Expr.eqv`: α-equivalence with binder annotations ignored. Two statements that differ only in bound-variable names, or only in which arguments are implicit, are one node, printed with the first one's names and annotations. Hashing the 19 855 398 statement nodes with everything, without binder names, and without annotations gives the same 19 855 398 hashes. So the 16 423 groups of the export measurement were α-equivalence classes (modulo annotations) all along, and the export cannot show the finer levels. Which of them should belong to a theorem's id is [[RQ-005 What a Lean Statement's Identity Includes]].

| Statements equal up to | groups | theorems in groups | beyond one per group |
|---|---|---|---|
| the export's node (as measured on 2026-10-20) | 16 423 | 70 561 | 54 138 (10.4 %) |
| everything the export keeps, the universe parameter list included (one pair differs only there) | 16 422 | 70 559 | 54 137 |
| … without binder names / annotations | the same (see above) | | |
| … and universe parameters renamed by position | **17 372** | 72 554 | **55 182 (10.6 %)** |

**Causes** (at the last level; groups, theorems, theorems beyond one per group):

| Cause | groups | theorems | beyond one | share of repeats |
|---|---|---|---|---|
| **generated**: at most one member has a hand-written name; the rest are made by Lean: `simp`'s private `_simp_N_M` copies of a lemma (most), `_proof_N` proofs abstracted out of definitions, equation lemmas | 12 123 | 60 905 | 48 782 | 88.4 % |
| **alias**: the hand-named members are one proof under several names (a proof is another member applied to its arguments: `alias`, deprecations, `LT.lt.le` for `le_of_lt`) | 4 410 | 9 351 | 4 941 | 9.0 % |
| **duplicate**: two or more hand-named members whose proofs are not such references | 839 | 2 298 | 1 459 | 2.6 % |

Of the 839 duplicate groups, 314 have two hand-named members with the same (α-equal) proof term. In 70 more, one member's proof mentions another member, though not as a bare reference. The other 455 have independent proofs: true duplicates, in part between Lean's core and Mathlib (`Nat.zero_sub` and `Lean.Grind.zero_sub`, `heq_of_eq` and `Eq.heq`, `Int.ofNat_dvd` and `Int.natCast_dvd_natCast`). Samples are in `groups.tsv`; the generated-name patterns are a rule of thumb ([[lean_statements]]).

**The failure condition did not occur.** The repeats are not export artefacts; if anything, the export merges more than it should. They are real repeated kernel statements, but nine in ten come from the elaborator, not from authors. With statements as identity they collapse as Metamath's do. A store that keeps only hand-named theorems would still have about 6 400 repeats (aliases and duplicates).

**Dependencies** (distinct constants mentioned directly; mean, median, max over 519 639 theorems):

| | mean | median | max |
|---|---|---|---|
| by the statement | 17.6 | 14 | 228 |
| by the proof | 29.5 | 19 | 692 |
| by the proof, not by the statement | 14.5 | 4 | 647 |
| theorems cited by the proof | 6.2 | 2 | 283 |

Over the library, closing over types, definition bodies (not proofs) and inductive groups:

| Kind | all | reached from statements | only from proofs | from neither |
|---|---|---|---|---|
| theorem | 519 639 | 94 641 | 273 530 | 151 468 |
| definition | 187 268 | 78 428 | 35 330 | 73 510 |
| inductive types, constructors, recursors | 24 403 | 23 337 | 3 | 1 063 |
| opaque | 2 978 | 521 | 2 | 2 455 |

So 35 330 definitions (19 %) exist for proofs only: they are evidence, like the 71 % of the term graph above. 151 468 theorems (29 %) are cited by no statement and no proof in the library: leaves: results for users, not used inside the library. A theorem reached from a statement is one whose proof term sits inside another declaration's type.

**Axioms behind theorems** (through types, values and inductive groups, proofs included, to a fixpoint): `propext` 89.0 %, `Quot.sound` 79.9 %, `Classical.choice` 66.0 %, **`sorryAx` none**, no axiom at all 9.3 % (48 261).

**Re-checked here (SR-137).** Lean's own kernel replays Mathlib's declarations on this machine: `leanchecker` from the toolchain (lean4checker, now shipped with Lean), one process per module, 6 in parallel.
- **8 531 of 8 650 modules pass; none is rejected.** 77.7 min wall, 7.2 h of summed module time, the slowest module 28.3 s, peak memory under 5 GB per process.
- The other **119 crash** (segfault in `leanchecker`'s import, before any declaration is replayed; [[recheck]]). Re-checked with `--fresh`, which replays each module and everything it imports from nothing: **all 119 pass** (in three runs because of the session crashes; 10 331 s summed, the slowest `Mathlib.NumberTheory.ModularForms.LevelOne` at 264 s, peak 3.7 GB). So every one of Mathlib's 8 650 modules has been checked here, and none is rejected.
- **It does catch bad proofs:** a declaration `bad : False := True.intro` added with the kernel check switched off builds, and `leanchecker` rejects it.

The trust record for Mathlib `98a293f` can now be `checked by Lean kernel v4.36.0-rc1 (leanchecker), on this machine, 2026-11-20`. Its scope is all of Mathlib's own declarations, plus the import closures of the `--fresh` modules, which include Lean's core. An independent kernel (nanoda, lean4lean, both in the toolchain) has not been run on Mathlib. `nanoda` 0.4.17 was tried on the 520 MB export of `Nat`'s basics (`nat-basic.ndjson`, 2026-11-20, `$SOPHIA_CORPORA/e10/nanoda-nat.json`). The kernel's OOM killer stopped it twice, at 26 and 28 GB resident, without a result: at least 50 times the size of its input. The whole export (5.9 GB) is out of reach for it on this 30 GB machine. Running it alongside other jobs also ended the agent session: run memory-hungry checkers in their own `systemd-run --user -p MemoryMax=…` unit.

**Next:** a Lean reader into the store needs RQ-005 decided first, and an export that keeps binder annotations if they are to count.
