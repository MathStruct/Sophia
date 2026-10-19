#overview #open-problem

> The first version of this vault (`vault/Design/`, `Background/`, `State of the Art/`, the comment-only code) was written by an agent from Daniel's first prompts. Daniel said on 2026-10-17 that it "may have misunderstood" him, and that the repository is brainstorming, not a plan. This note lists where the earlier reading and Daniel's own statements differ, so each point can be confirmed or dropped. Until then, the `Design/` notes are **ideas, not requirements**. The requirements are in [[Requirements]].
>
> Sources: `meta/Prompts/Prompt.md`, `Prompt2.md`, `Prompt3.md`; [[The Original Idea]]; Moonkale `North Star.md` (Daniel's words, 2026-10-14/15); Daniel's message of 2026-10-17.

| # | The earlier vault says | Daniel said | To confirm |
|---|---|---|---|
| 1 | Rust and Julia split the work by concern: a Rust kernel and store, a Julia frontend and macros, joined by a C ABI ([[Repository Layout]]). | "Julia for rapid development … run Rust in production." Not the same thing implemented twice (Prompt3). | **Settled 2026-10-17: Rust only.** Julia maybe much later, for ML on the code graph ([[S-ADR-0004 Rust for the store, Julia for experiments]]). |
| 2 | "Julia + Lean" means "a proof layer *about* Julia code, not dependent types *inside* Julia" ([[Roadmap]] M5). | "A dependently typed Julia with Prolog-like elements … checked by several systems (dependent, cubical, algebraic)" (North Star, 2026-10-14). | **Settled 2026-10-20:** yes, sophialang, Julia-flavoured, with dependent types as an *exchangeable* checker that adds no run-time cost (SR-53, [[SPEC-002 Sophialang Principles]]). |
| 3 | The headline is cross-language interoperability through equivalence edges with evidence. Everything is ordered towards it (M6, M7). | That goal is in the Original Idea. But on 2026-10-17 the reason to continue is to specify Moonkale, and the North Star frames many languages as parsed into one graph, with proofs as annotations. | **Settled 2026-10-20:** the Moonkale interface first; then individual languages parsed into the store; reuse and interop are the long-term dream (SR-63, SR-73). |
| 4 | M0 is Metamath and M1 is a Lisp, chosen to test the plumbing. | Not mentioned by Daniel. | **Settled 2026-10-20:** sophialang first; Metamath or Lisp maybe later ([[S-ADR-0008 Sophialang before Metamath or Lisp]]). |
| 5 | The store is DuckDB first. | Lists six databases without a preference. | **Settled 2026-10-20:** try several and compare ([[Database Comparison]]); SQLite is the default meanwhile. |
| 6 | Precompilation of Julia is "where the project justifies itself" (M3). | "I wanted to improve precompilation for Julia" is one motivation among several. | **Settled 2026-10-20:** a goal, but not the first priority. It needs analytics on top of the store (SR-66). |
| 7 | The code files contain only comments. | "Please do not add any code yet" (Prompt2, at the time). | **Settled 2026-10-20:** yes, plan the milestones and implement ([[Milestones]]). |

## What the earlier vault still gives

- **Background cards and surveys** are useful reference and stay as they are: [[Background Concepts]], [[State of the Art]].
- **Design notes** that agree with SPEC-001 and can be read alongside it: [[Hashing and Identity]] (Merkle hashing, binders, cycles); [[Naming and Change Propagation]]; [[Tests and Documentation as Nodes]]; [[Equivalence and Witnesses]] (its ladder of evidence strengths).
- **Design notes** to read as ideas only, until confirmed: [[Core Calculus]] (its `Prf`/`Cmp` split is one option); [[Compilation as Query]]; [[Cross-Language Semantic Hazards]]; [[Content-Addressed Precompilation]]; [[Roadmap]].
