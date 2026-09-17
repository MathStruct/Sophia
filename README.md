# Sophia

**This repository is an experiment: We currently do not know much about the topic at all, so we query  a lot in order to gain understanding. This is probably not a repository we will actually implement. (At least not at our current state of knowledge)**

Regardless of that if you have any feed back w.r.t. such an undertaking send us a message.

Please don't train any AI on any of this.

**Under development — design stage. No code here runs yet.**

A content-addressed graph database of code, its types, its proofs and all of its intermediate representations, with a compiler expressed as queries over that database.

Every declaration, expression, MLIR operation, LLVM instruction, test, doc comment and proof becomes a node keyed by a hash of its own normalised structure. Frontends elaborate source languages into one common core calculus; backends lower core terms through MLIR into LLVM IR. Code from different languages is meant to interoperate not through an FFI or a shared ABI, but by storing *equivalence edges* that carry machine-checkable evidence.

## Documentation

**Read it online: <https://mathstruct.org/Sophia/>**

This repository is also an Obsidian vault (the vault root is the repository root, so every `.md` file in it is a note and `[[wikilinks]]` resolve across code and prose alike). The website is built from the vault with [Quartz](https://quartz.jzhao.xyz); see [Website](#website) below.

- **[markdown/Start Here.md](markdown/Start%20Here.md)** — the original statement of the idea
- **[markdown/Design/](markdown/Design/)** — what it becomes once made precise: core calculus, hashing scheme, graph schema, equivalence ladder, roadmap, and an honest risk list. Start at `Design Overview.md`.
- **[markdown/Wiki/](markdown/Wiki/)** — background concept cards (LLVM, MLIR, Unison, hashing, rewriting, semantics, type theory, category theory, Julia internals)
- **[markdown/State of the Art/](markdown/State%20of%20the%20Art/)** — survey of prior art, and a table of what exists versus what does not
- **`Code Map.md`** in `markdown/Design/` — index of the per-file design notes

Formulas are Typst (`wypst`), diagrams are TikZ (`inline-tikz`).

## Layout

```
Cargo.toml          Rust workspace manifest
Project.toml        Julia package manifest
crates/             Rust: hashing, core calculus, store, equivalence, emission, CLI
src/                Julia: frontend, annotation macros, precompilation experiment
markdown/           the vault
index.md            landing page of the website
site/               vendored Quartz + plugins that build the website from the vault
```

Julia requires `src/Sophia.jl`, so `src/` belongs to Julia and Rust lives in `crates/`. Every `.rs` and `.jl` file has a `.md` sibling of the same name explaining what it is meant to do. See `markdown/Design/Repository Layout.md`.

**All source files currently contain comments only.** They exist to fix module boundaries while the design settles.

## Website

`site/` is a vendored copy of Quartz 4.5.2 taken from
[MathStruct/MathStruct.github.io](https://github.com/MathStruct/MathStruct.github.io),
whose build-time plugins match the vault's Obsidian plugins:

- `quartz/plugins/transformers/tikz.ts` — ` ```tikz ` fences in the **Inline TikZ** format (`\usepackage` lines, then a full `\begin{document} … \end{document}`) are compiled to SVG with `node-tikzjax` and cached by content hash in `site/.tikz-cache` (committed, so CI does not recompile every diagram). TikZJax ships a small TeX distribution: a macro it lacks (`\llbracket` from `stmaryrd`, say) has to be defined in the preamble.
- `quartz/plugins/transformers/latex.ts` — every formula is offered to Typst first (**Wypst** syntax), and anything Typst rejects falls through to KaTeX, so a stray LaTeX formula still renders. Pin a page with `math: typst` or `math: latex` frontmatter to forbid the other syntax. The build prints a `Mixed math in …` line for every note that used both.
- `quartz/plugins/transformers/tabs.ts` — **Markdown Tabs** blocks.
- `quartz/plugins/transformers/titleHeading.ts` — drops the `# Note Name` heading each note opens with, since Quartz renders the note name as the title.

The content directory is the repository root, so `src/*.md` and `crates/**/*.md` are published next to `markdown/` and the `[[wikilinks]]` in `Code Map.md` resolve. `site/quartz.config.ts` lists what is *not* published: `site/`, `.obsidian/`, `markdown/Prompts/`, this README (the site has its own `index.md`), and the `.rs`/`.jl`/`.toml` sources themselves.

Local preview:

```bash
cd site
npm ci                          # first time only; needs Node 22+
npx quartz build --serve -d ..
```

Then open <http://localhost:8080>. `npm run check` typechecks and checks formatting.

Pushing to `master` runs `.github/workflows/deploy.yml`, which builds the site and publishes it to GitHub Pages (the workflow enables Pages with source "GitHub Actions" on first run). The site is served as a project page of the organisation, under its custom domain: `baseUrl` in `site/quartz.config.ts` is `mathstruct.org/Sophia`.

To pull in upstream changes, merge them into `site/` and keep the four transformer files above plus `quartz.config.ts` and `quartz.layout.ts`.

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

Full version with rationale: `markdown/Design/Roadmap.md`.

Note the ordering: **M3 is where the project justifies itself**, and it needs none of the cross-language machinery. Even total failure at M6 leaves a content-addressed, queryable code store with exact dependency tracking and exact test and compilation invalidation.

## Databases under evaluation

Graph: LadybugDB, HelixDB, TypeDB, FalkorDB. Relational: DuckDB, TursoDB.

Current leaning is DuckDB first — the schema is two tables wide and the hot queries are hash lookups and bounded traversals, which a columnar engine handles well without a server. The store sits behind a trait so the choice stays reversible; see `markdown/State of the Art/State of the Art - Graph Databases for Code.md`.

## On disagreeing with any of this

The store is content-addressed, so it is not trusted: any node can be re-verified by rehashing it. And migration is re-elaboration from core terms rather than a text rewrite. So if you dislike the schema, replace `crates/sophia-store/src/schema.rs` and migrate — that is a supported operation, not a fork.

## Prior art

The closest existing system is [Unison](https://www.unison-lang.org/), which content-addresses code by the hash of its syntax tree. Sophia borrows that and adds persisted IR layers, a multi-language frontend story, and equivalence edges carrying evidence. What is genuinely new — and genuinely risky — is the last of those; see `markdown/Design/Open Problems and Risks.md`.
