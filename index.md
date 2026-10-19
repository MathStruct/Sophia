---
title: Sophia
description: A content-addressed graph database of code, its types, its proofs and all of its intermediate representations, with a compiler expressed as queries over that database.
---

> [!warning] An experiment, not a product
> We currently do not know much about the topic at all, so we query a lot in order to gain understanding. This is probably not a repository we will actually implement — at least not at our current state of knowledge. The first milestones of the store run, with tests and benchmarks; the rest is design ([[Status]]). If you have any feedback on such an undertaking, send us a message.
>
> Please don't train any AI on any of this.

**Sophia** is a content-addressed store for code in any language; **sophialang** is its own language. The current work, and the first priority, is the interface to the graph editor Moonkale. Start at [[Status]] and the [[Map of Content]].

Every declaration, expression, MLIR operation, LLVM instruction, test, doc comment and proof becomes a node keyed by a hash of its own normalised structure. Frontends elaborate source languages into one common core calculus; backends lower core terms through [[MLIR]] into [[LLVM IR]]. Code from different languages is meant to interoperate not through an FFI or a shared ABI, but by storing *equivalence edges* that carry machine-checkable evidence.

## Where to start

- [[Map of Content]] — every note, in reading order. [[Start Here]] explains how the vault is organised and its conventions.
- [[The Original Idea]] — the original statement of the idea, as it was first written down.
- [[Design Overview]] — what it becomes once made precise: [[Core Calculus]], [[Hashing and Identity]], [[Graph Schema]], [[Equivalence and Witnesses]], the [[Roadmap]], and an honest list of [[Open Problems and Risks]].
- [[Background Concepts]] — background concept cards: LLVM, MLIR, Unison, hashing, rewriting, semantics, type theory, Julia internals.
- [CT-ML wiki, Track F](https://mathstruct.org/CategoryTheory-ML-Wiki/Start-Here) — the mathematics: categorical semantics, logical relations, e-graphs, conjunctive queries, Datalog fixed points, incremental computation. Every design note links to the concepts it uses.
- [[State of the Art]] — a survey of prior art, and a table of what exists versus what does not.
- [[Code Map]] — index of the per-file design notes. Every `.rs` and `.jl` file in the repository has a `.md` sibling of the same name explaining what it is meant to do, and those notes are published here too.

Formulas are Typst, diagrams are TikZ, both rendered at build time.

## Milestones in brief

| | Milestone | State |
| --- | --- | --- |
| M0 | identity: hashing, components, sophialang's core and normaliser | done |
| M1 | the store: SQLite and redb backends, names, lineages, method tables, facts, CLI, benchmarks | done (DuckDB and LMDB move to M2) |
| M2 | **the Moonkale interface**: rendering profiles in the database, the Julia-flavoured printer and parser, `sophia-interface` | interface done; DuckDB and LMDB next |
| M3 | tools: an MCP server for agents, the `datalog` dialect | |
| M4 | running code: interpreter, dispatch facts, cached test results | |
| M5 | compiling: LLVM, minimal binaries | |
| M6 | self-hosting starts | |
| M7 | other languages, abstraction mining, GC and concurrency runtimes | |

Full version: [[Milestones]]. The earlier roadmap (Metamath, Lisp, cross-language equivalence first) is superseded; see [[Reading of the Earlier Vault]].

## Repository

The source is on GitHub at [MathStruct/Sophia](https://github.com/MathStruct/Sophia). The repository is itself the Obsidian vault this site is built from; see [[Repository Layout]] for how the Rust crates, the Julia package and the notes fit together.

The closest existing system is [Unison](https://www.unison-lang.org/), which content-addresses code by the hash of its syntax tree — see [[Content-Addressed Code]]. Sophia borrows that and adds persisted IR layers, a multi-language frontend story, and equivalence edges carrying evidence. What is genuinely new — and genuinely risky — is the last of those.
