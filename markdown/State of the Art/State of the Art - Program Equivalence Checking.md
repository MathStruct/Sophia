# State of the Art - Program Equivalence Checking

[[Start Here]] wants to "insert equivalency proofs" relating two pieces of code. This note surveys the tools that actually decide, or check, program equivalence today — the single most relevant body of engineering work for the project's central claim.

## Translation validation, per-instance

Rather than proving a compiler correct once and for all (the [[State of the Art - Proof-Carrying Code and Verified Compilation|CompCert]] approach), check each compilation.

- **Alive2** — the most important entry. It checks [[LLVM IR]] transformations for refinement using an SMT solver, modelling LLVM's `undef`, `poison` and UB semantics faithfully. It has found many real miscompilation bugs in LLVM and now runs against the LLVM test suite continuously. Crucially it checks **refinement**, not equality — the optimised program must have *no more* behaviours — which is exactly the `REFINES` relation of [[Equivalence and Witnesses]] and confirms that refinement, not equivalence, is the right primitive for a lowering.
- **Alive / Alive-NJ** — the predecessor, which additionally *generates* verified peephole optimisations from a DSL.
- **Necula's translation validation for GCC**, and **Pnueli, Siegel and Singerman's** original formulation — the origin of the idea.
- **CoVaC** — equivalence checking of programs related by a simulation relation, handling loop transformations.
- **SymDiff** (Microsoft Research) — differential program verification over Boogie, for showing two versions of a program equivalent or characterising the difference.
- **LLVM-MD, Peggy** — equivalence checking built on [[E-Graph|e-graph]]-like representations (Peggy's PEG, "Program Expression Graphs", is essentially equality saturation applied to equivalence checking).

## Full-program and regression equivalence

- **RVT (Regression Verification Tool)** — proves equivalence of two versions of a C program by decomposing over matching function pairs and using uninterpreted functions for recursive calls. The decomposition strategy is directly reusable: it is how you check equivalence *compositionally* rather than by symbolically executing whole programs.
- **KLEE-based differential symbolic execution / UC-KLEE** — bounded, finds counterexamples rather than proving equivalence. Evidence, not proof, in [[Equivalence and Witnesses]] terms.
- **SMT-based bounded equivalence** for hardware (equivalence checking is *routine* in EDA — Cadence Conformal, Synopsys Formality — for RTL vs. gate-level netlists). Worth noting that an entire industry depends on automated equivalence checking; the difference is that hardware is finite-state.

## Cross-language equivalence

Much thinner, which is the honest summary of the risk in [[Open Problems and Risks]].

- **Multi-language semantics** (Matthews and Findler's boundaries; Patterson and Ahmed's "linking types" and *compositional compiler correctness*) — the theory of what it even means for terms in two languages to interoperate soundly. This is where [[Logical Relations|cross-language logical relations]] are developed, and it is the closest formal foundation for what Sophia wants.
- **Verified compilers with multi-language targets** — CakeML, CompCert's linking work, and SepCompCert address *separate compilation* correctness, which is a special case of cross-language equivalence where one side is the compiled form of the other.
- **Binary lifting and decompilation** (rev.ng, Ghidra P-Code, McSema, angr) — recovering a higher-level representation from machine code, with equivalence to the original binary as the correctness criterion. Relevant because it is the one place where "unify code from many languages" is routinely attempted, albeit by lowering everything to machine code first, which is exactly the loss of semantic information Sophia is trying to avoid.

## What to take from this

1. **Refinement, not equality, is the practical relation.** Alive2 settled this empirically.
2. **Per-instance checking is tractable; general verification is not.** Translation validation should be the default mechanism for `Term → Op → Instr` edges ([[Trusted Computing Base]]).
3. **SMT certificates are a realistic witness format** for decidable fragments (bitvectors, arrays, linear arithmetic), and cover a surprising amount of real code.
4. **Compositional decomposition (RVT-style) is how equivalence checking scales** — match up functions and use uninterpreted functions at the boundaries. That maps perfectly onto a graph of hash-identified definitions, where the matching is already recorded.
5. **Nobody has done proof-carrying cross-language equivalence at scale.** The theory exists (multi-language semantics); the engineering does not.

## Related

- [[Equivalence and Witnesses]]
- [[Contextual Equivalence]]
- [[Logical Relations]]
- [[Trusted Computing Base]]
- [[State of the Art - Proof-Carrying Code and Verified Compilation]]
- [[State of the Art - Equality Saturation and E-Graphs]]
