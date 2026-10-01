#overview

> How this vault is organised, the conventions every note follows, and where the mathematics lives. The notes themselves are indexed in [[Map of Content]]; the idea as first written down is [[The Original Idea]].

## What lives where

The vault is the whole repository. Its notes sit in four places:

| where | what | example |
|---|---|---|
| `vault/Design/` | **design notes**: what Sophia is supposed to be, made precise | `vault/Design/Hashing and Identity.md` |
| `vault/State of the Art/` | **surveys** of prior art, one area per note | `State of the Art - Equality Saturation and E-Graphs.md` |
| `vault/Background/` | **background cards** on engineering topics: LLVM, MLIR, Julia internals, hashing, Unison | `vault/Background/Hashing/Merkle DAG.md` |
| `crates/*/src/*.md`, `src/*.md` | **per-file design notes**, one beside each Rust or Julia file | `crates/sophia-hash/src/merkle.md` beside `merkle.rs` |
| `meta/` | working material: the prompts used while drafting (not published) | |

The **mathematics** — categorical semantics, logical relations, e-graphs as algebra, conjunctive queries, fixed points, incremental computation — is not repeated here. It lives in the [CT-ML wiki](https://mathstruct.org/CategoryTheory-ML-Wiki/), the root vault this one builds on, whose *Track F* in [Start Here](https://mathstruct.org/CategoryTheory-ML-Wiki/Start-Here) leads up to every concept Sophia uses. This vault keeps what is specific to Sophia: the design decisions, the engineering background, and the honest list of what may not work.

## Conventions

Every note opens the same way:

1. **a tag line** — one or more of `#overview`, `#design`, `#definition`, `#algorithm`, `#example`, `#comparison`, `#implementation`, `#open-problem`, `#reference`;
2. **a sources block**:
   - `> Sources:` — the papers and systems the note relies on, or "original to this vault" for design analysis, then `code:` the files that would implement it;
   - `> Theory (CT-ML wiki):` — links to the general concepts the note uses.

Formulas are written in **Typst** (the site renders Typst first and falls back to KaTeX for LaTeX); diagrams are ```` ```tikz ```` blocks in the Inline TikZ format. A note's title is its filename, so notes do not repeat it as a heading. Links between notes are `[[wikilinks]]`; links to general concepts go to the CT-ML wiki's published pages.

## Reading it

- **On the web**: <https://mathstruct.org/Sophia/>, built with Quartz from this repository (see `README.md`).
- **In Obsidian**: open the repository root as a vault; the *Inline TikZ* and *Wypst* plugins render diagrams and Typst.

## Adding to it

A new design decision gets a note in `vault/Design/` and a line in [[Map of Content]]. A new source file gets a `.md` sibling of the same name saying what it is meant to do. A new *general* concept — something true of programming languages or databases in general, not of Sophia — goes into the CT-ML wiki first, and the Sophia note links to it.
