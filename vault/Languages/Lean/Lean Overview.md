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
- *α-equivalent* (equal up to binder names);
- *definitionally equal* (the kernel can show it by computation: needs the kernel);
- *propositionally equal or equivalent* (someone proved `A ↔ B`: a theorem, i.e. evidence, with a proof).

Only the first two can be identity in Sophia. Definitional equality is a check with a cost and a trusted kernel; propositional equivalence is a theorem like any other.

## What is verified, and by whom (2026-10-20)

- The Mathlib build cache (`.olean` files) was **downloaded**, built by Mathlib's CI. Their declarations were checked by Lean's kernel **on CI**, not here. Importing `.olean` files does not re-run the kernel.
- `lean4export` reads those `.olean` files. So **nothing in the export has been kernel-checked on this machine.**
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

**Next step:** E-10 in [[Semantic Boundary Experiments]]: inspect the repeated-statement groups, measure α-equivalence and the per-theorem dependency counts (statement vs proof), and re-check a part with `lean4checker`; only then a reader into the store.
