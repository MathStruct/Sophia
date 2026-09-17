---
title: Sophia
description: A content-addressed graph database of code, its types, its proofs and all of its intermediate representations, with a compiler expressed as queries over that database.
---

> [!warning] An experiment, not a product
> We currently do not know much about the topic at all, so we query a lot in order to gain understanding. This is probably not a repository we will actually implement — at least not at our current state of knowledge. **No code here runs yet.** If you have any feedback on such an undertaking, send us a message.
>
> Please don't train any AI on any of this.

**Sophia** is a content-addressed graph database of code, its types, its proofs and all of its intermediate representations, with a compiler expressed as queries over that database.

Every declaration, expression, MLIR operation, LLVM instruction, test, doc comment and proof becomes a node keyed by a hash of its own normalised structure. Frontends elaborate source languages into one common core calculus; backends lower core terms through [[MLIR]] into [[LLVM IR]]. Code from different languages is meant to interoperate not through an FFI or a shared ABI, but by storing *equivalence edges* that carry machine-checkable evidence.

## Where to start

- [[Start Here]] — the original statement of the idea, as it was first written down.
- [[Design Overview]] — what it becomes once made precise: [[Core Calculus]], [[Hashing and Identity]], [[Graph Schema]], [[Equivalence and Witnesses]], the [[Roadmap]], and an honest list of [[Open Problems and Risks]].
- [[Wiki Home]] — background concept cards: LLVM, MLIR, Unison, hashing, rewriting, semantics, type theory, category theory, Julia internals.
- [[State of the Art]] — a survey of prior art, and a table of what exists versus what does not.
- [[Code Map]] — index of the per-file design notes. Every `.rs` and `.jl` file in the repository has a `.md` sibling of the same name explaining what it is meant to do, and those notes are published here too.

Formulas are Typst, diagrams are TikZ, both rendered at build time.

## Roadmap in brief

| | Milestone | Tests the assumption |
| --- | --- | --- |
| M0 | Metamath ingestion, hashing, schema | canonicalisation is deterministic and total |
| M1 | a tiny Lisp, end to end | binders, elaboration, frontend functoriality |
| M2 | emit MLIR → LLVM and run it | querying IR out of the store is fast enough |
| M3 | Julia ingestion + content-addressed code cache | invalidation precision beats Julia's current scheme |
| M4 | equivalence within one language (e-graph) | a rewrite rule set over the core is useful |
| M5 | the proof fragment; specs attached to Julia code | "Julia + Lean", in the tractable form |
| M6 | one witnessed Julia ↔ C++ vertical slice | a cross-language relation can be written at all |
| M7 | unified execution | — |

Note the ordering: **M3 is where the project justifies itself**, and it needs none of the cross-language machinery. Even total failure at M6 leaves a content-addressed, queryable code store with exact dependency tracking and exact test and compilation invalidation. Full version with rationale: [[Roadmap]].

## Repository

The source is on GitHub at [MathStruct/Sophia](https://github.com/MathStruct/Sophia). The repository is itself the Obsidian vault this site is built from; see [[Repository Layout]] for how the Rust crates, the Julia package and the notes fit together.

The closest existing system is [Unison](https://www.unison-lang.org/), which content-addresses code by the hash of its syntax tree — see [[Content-Addressed Code]]. Sophia borrows that and adds persisted IR layers, a multi-language frontend story, and equivalence edges carrying evidence. What is genuinely new — and genuinely risky — is the last of those.
