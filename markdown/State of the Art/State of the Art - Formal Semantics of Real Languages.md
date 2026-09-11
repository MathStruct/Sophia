# State of the Art - Formal Semantics of Real Languages

How much is actually known, formally, about the languages this project cares about. This note exists because [[Equivalence and Witnesses|every equivalence claim]] presupposes a semantics, and the honest answer differs wildly by language.

| Language | Formal semantics? | Notes |
| --- | --- | --- |
| **C** | Yes, several | CompCert's Clight/CompCert C (in Coq); Ellison–Roșu's K semantics of C11, executable and used to find compiler and standard bugs; Cerberus, which specifically models the contested parts (pointer provenance, effective types) |
| **C++** | Essentially no | Far too large. Fragments formalised (the C++11 memory model by Batty et al. is the major success, and it changed the standard). No whole-language semantics exists or is likely to. |
| **Rust** | Partially | RustBelt gives a semantic model of the type system in Iris and verifies `unsafe` standard-library code. Stacked Borrows / Tree Borrows model aliasing and are **still being revised**; Miri operationalises them. There is no complete semantics, and the community treats the question as open. |
| **Julia** | **No** | Subtyping has been formalised (Zappa Nardelli et al., "Julia subtyping: a rational reconstruction" — and it is genuinely intricate). Dynamic semantics: nothing. |
| **Lean 4** | Yes, by construction | Its kernel *is* the specification; independent type checkers exist |
| **JavaScript** | Yes | JSCert (Coq), KJS (K), and the spec itself is unusually operational |
| **LLVM IR** | Partially | Semantics of `undef`/`poison` and UB were clarified largely *because* of Alive2's need for them; still not fully settled. Vellvm formalises a subset in Coq. |
| **WebAssembly** | Yes | Formally specified in the standard itself, with a mechanised proof of type soundness. The one mainstream language designed this way. |
| **x86-64 / ARM / RISC-V** | Yes | Sail specifications; ARM publishes machine-readable semantics |

## What this table means for Sophia

**The two languages [[Start Here]] most wants to unify — Julia and C++ — are the two with the weakest formal foundations.** That is not a coincidence: both are large, dynamic (in different senses) and defined by their implementations. It is the central practical obstacle to the cross-language goal, more than any database or hashing concern.

Three consequences:

1. **Do not attempt a full semantics for either.** Specify [[Core Calculus|SC]] properly, and let each frontend cover an explicitly bounded subset. The subset boundary must be *enforced* — ingestion of anything outside it should fail loudly rather than silently producing a term whose meaning is guessed.
2. **The elaboration function is the de facto semantics** of the ingested subset, and it is unverified. Differential testing (run the original, run the extracted core term, compare) is the only realistic validation. That makes a good conformance-suite investment the highest-value engineering after [[Roadmap|M0]].
3. **WebAssembly and LLVM IR are the well-founded meeting points.** If a cross-language claim must rest on something with a real semantics, it will rest on one of those, not on Julia or C++ directly. Worth remembering when choosing where in the layer cake to state an equivalence.

## The encouraging half

Formalising real languages has repeatedly turned out to be *possible* and *useful*: the C++11 memory model changed the standard, K's C semantics found bugs in GCC and Clang test suites, RustBelt found soundness bugs in the Rust standard library, and Alive2 found miscompilations in LLVM. None of these were considered feasible before someone did them. The pattern is always the same — start with a fragment, mechanise it, and let the tooling find the discrepancies.

## Related

- [[State of the Art - Language Semantics Frameworks]]
- [[State of the Art - Program Equivalence Checking]]
- [[Cross-Language Semantic Hazards]]
- [[Operational Semantics]]
- [[Open Problems and Risks]]
