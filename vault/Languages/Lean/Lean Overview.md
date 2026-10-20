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

**Next step:** run the export (`sophia-mathlib` prints the command), measure its size, and write the reader: hypotheses and failure criteria to go into [[Semantic Boundary Experiments]] first.
