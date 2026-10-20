#overview

> **One folder per language** Sophia stores, plans to store, or is: how it is parsed, normalised and resolved; what its semantics are and how much of them Sophia models; how it relates to sophialang; and what is known not to work. Started 2026-10-20 at Daniel's request (SR-122, SR-123).
>
> Sources: the language pages below; [[SPEC-008 Module-Level Julia Resolution]]; [[Real Code Corpora]]; [[Cross-Language Semantic Hazards]].

## Support is a ladder, not a percentage

"99 % of LinearAlgebra gets ids" says the first two rungs work. It says nothing about the rest. Every claim about a language names its rung and the test that backs it.

| Level | Meaning | Evidence that counts |
|---|---|---|
| **L1 Parsed** | the syntax is recognised without loss | round-trip or golden parse tests; refusal counts on real corpora |
| **L2 Normalised** | a stable canonical form and content id exist | ids invariant under renaming and formatting; identical ids in Moonkale and Sophia (SR-116) |
| **L3 Resolved** | names, bindings and references are identified as the language means them | agreement with the language's own resolver on real code ([[Julia Conformance]]) |
| **L4 Lowered** | represented in a shared semantic IR (sophialang core, or a stated IR layer) | elaboration tests; translation validation |
| **L5 Executable** | can be run with its intended semantics | differential runs against the language's own implementation |
| **L6 Verified** | translation or execution is proved, or checked per instance | certificates, checked proofs, or a verified toolchain |

Two rules:
- **A level is claimed per feature, not per language.** Julia is L3 for module-level names and generic-function ownership, L2 for macros (they are hashed as written, not expanded), and L0 for generated functions.
- **Unresolved is a valid answer.** A frontend that says "this I do not know" (a refusal, an approximation flag, a `dynamic` include) is correct. One that guesses silently is not.

## The matrix (2026-10-20)

| Language | Folder | L1 Parsed | L2 Normalised | L3 Resolved | L4 Lowered | L5 Executable | L6 Verified |
|---|---|---|---|---|---|---|---|
| **sophialang** | [[SophiaLang Overview]] | yes (Julia and Rust profiles round-trip) | yes (`sophialang-core@0`) | yes (elaboration against the store) | it *is* the core | no | no |
| **Sophia query** | [[Sophia Query Overview]] | yes | yes (exact canonical form) | yes (checked, stratified) | yes (IR; SQL translation) | **yes** (native and SQL, four backends) | differential tests only |
| **Julia** | [[Julia Overview]] | yes (tree-sitter) | yes (WP-66 v6, shared with Moonkale) | partly: module scope, imports, generic ownership 94.5 % agreement with Julia | no | no | no |
| **Rust** | [[Rust Overview]] | yes | yes (WP-66) | same-file names only | no | no | no |
| **Python** | [[Python Overview]] | yes | yes (WP-66) | same-file names only | no | no | no |
| **Metamath** | [[Metamath Overview]] | yes (full format, except file inclusion) | yes (`sophia-metamath@2`) | yes (labels are global) | stored as statements and proofs | no (proofs are not checked by Sophia) | **dependency analysis checked against metamath.exe** |
| **Lisp** | [[Lisp Overview]] | planned (S-ADR-0008) | — | — | — | — | — |
| **Lean 4** | [[Lean Overview]] | planned (rung 5) | — | — | — | — | — |

## What each language page contains

1. **Overview:** status by level, with tests and numbers.
2. **Parsing and normalisation:** the parser, the canonical form, what is refused and why.
3. **Name resolution:** what a name means in the language, what Sophia resolves, what it approximates, what it leaves open.
4. **Semantics:** types, dispatch, effects and runtime as far as they affect identity or dependencies.
5. **Interoperability with sophialang:** how a definition of this language relates to sophialang definitions (references, equivalence claims, lowering).
6. **Limitations:** known wrong answers and deliberate gaps.

Julia has a page per topic because it is the language that matters most (SR-114). Languages with less work have one page.

## Related

[[Storage and Materialization]] (what each frontend's output costs to keep) · [[Semantic Boundary Experiments]] (the experiments that move languages up the ladder) · [[Cross-Language Semantic Hazards]]
