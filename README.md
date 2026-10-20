# Sophia

**This repository is an experiment: We currently do not know much about the topic at all, so we query  a lot in order to gain understanding. This is probably not a repository we will actually implement. (At least not at our current state of knowledge)**

Regardless of that if you have any feed back w.r.t. such an undertaking send us a message.

Please don't train any AI on any of this.

**Under development.** The store's first milestones (M0 identity, M1 storage) run, with tests and benchmarks; everything else is design. See `vault/Status.md` and `vault/Plans/Milestones.md`.

**Sophia** is a content-addressed store for code in any language: definitions keyed by the hash of their normalised structure, names as movable labels, proofs and test results as annotations and derived facts. **sophialang** is the store's own language: Julia-flavoured, multiple dispatch, dependent types as an exchangeable checker, and a principle of *autoresolve → unsafe → safe*. Its first job is to be the code store that the graph editor [Moonkale](https://github.com/MathStruct/Moonkale) edits.

Every declaration, expression, MLIR operation, LLVM instruction, test, doc comment and proof becomes a node keyed by a hash of its own normalised structure. Frontends elaborate source languages into one common core calculus; backends lower core terms through MLIR into LLVM IR. Code from different languages is meant to interoperate not through an FFI or a shared ABI, but by storing *equivalence edges* that carry machine-checkable evidence.

## Documentation

**Read it online: <https://mathstruct.org/Sophia/>**

This repository is also an Obsidian vault (the vault root is the repository root, so every `.md` file in it is a note and `[[wikilinks]]` resolve across code and prose alike). The website is built from the vault with [Quartz](https://quartz.jzhao.xyz); see [Website](#website) below.

- **[vault/Start Here.md](vault/Start%20Here.md)** — how the vault is organised and its conventions; **[vault/Map of Content.md](vault/Map%20of%20Content.md)** — every note in reading order
- **[vault/The Original Idea.md](vault/The%20Original%20Idea.md)** — the original statement of the idea
- **[vault/Design/](vault/Design/)** — what it becomes once made precise: core calculus, hashing scheme, graph schema, equivalence ladder, roadmap, and an honest risk list. Start at `Design Overview.md`.
- **[vault/Background/](vault/Background/)** — background concept cards (LLVM, MLIR, Unison, hashing, rewriting, semantics, type theory, Julia internals)
- **[vault/State of the Art/](vault/State%20of%20the%20Art/)** — survey of prior art, and a table of what exists versus what does not
- **`Code Map.md`** in `vault/Design/` — index of the per-file design notes
- **The mathematics** — categorical semantics, logical relations, e-graphs, conjunctive queries, Datalog, incremental computation — is in the [CT-ML wiki](https://mathstruct.org/CategoryTheory-ML-Wiki/) (Track F), which every design note links to.

Formulas are Typst (`wypst`), diagrams are TikZ (`inline-tikz`).

## Layout

```
Cargo.toml          Rust workspace (Rust only: vault/Decisions/S-ADR-0004)
crates/
  sophia-hash/      identity: canonical framing, BLAKE3 digests, recursive groups
  sophialang-core/  sophialang's core calculus and normaliser
  sophia-store/     the store, with SQLite, redb, LMDB and DuckDB backends and a conformance suite
  sophialang-syntax/ rendering profiles: printer and parser (Julia- and Rust-flavoured)
  sophia-interface/ what Moonkale wraps: nodes, edges, text, edits, profiles, the `query` dialect
  sophia-query/     Sophia's own query language: Datalog IR, Julia-flavoured syntax, content ids, executors
  sophia-ingest/    real code into Sophia: Metamath first (set.mm, iset.mm)
  sophia-bench/     the benchmark suite (criterion + a report binary)
  sophia-cli/       the `sophia` command
attic/              comment-only crates from the first design, kept as ideas
vault/              the vault: requirements, specifications, decisions, research, plans
meta/               working material and agent scratchpads (not published)
index.md, site/     the website (Quartz)
```

Every `.rs` file has a `.md` sibling of the same name explaining it. The index is `vault/Design/Code Map.md`.

**Tools** come from `flake.nix`:
- `nix develop` gives Rust, sqlite3, zstd, metamath, Julia 1.13 (`julia`) and 1.12 (`julia112`);
- `nix develop .#lean` adds elan;
- `sophia-corpora` clones set.mm into `$SOPHIA_CORPORA` (default `~/.cache/sophia-corpora`);
- `sophia-mathlib` fetches Mathlib with its build cache and builds lean4export (several GB).

```sh
cargo test --workspace                       # identity properties, store conformance and interface acceptance on both backends, syntax round trips
cargo run -p sophia-cli -- sample            # load the sample program into .sophia/store.sqlite
cargo run -p sophia-cli -- ls sample
cargo run -p sophia-cli -- text sample.add      # Julia-flavoured by default
cargo run -p sophia-cli -- profile rust         # the same code, Rust-flavoured
cargo run -p sophia-cli -- sql "SELECT kind, count(*) FROM definitions GROUP BY kind"
cargo run -p sophia-cli -- query 'select k, count(d) where def(d, k)'   # Sophia's own query language, any backend
cargo run --release -p sophia-bench --bin sophia-bench-report -- --sizes 1000,10000,100000
cargo run --release -p sophia-bench --bin sophia-mm-check -- --exe "$(command -v metamath)" "$SOPHIA_CORPORA/set.mm/set.mm" --sample 300
```

## Website

`site/` is a vendored copy of Quartz 4.5.2 taken from
[MathStruct/MathStruct.github.io](https://github.com/MathStruct/MathStruct.github.io),
whose build-time plugins match the vault's Obsidian plugins:

- `quartz/plugins/transformers/tikz.ts` — ` ```tikz ` fences in the **Inline TikZ** format (`\usepackage` lines, then a full `\begin{document} … \end{document}`) are compiled to SVG with `node-tikzjax` and cached by content hash in `site/.tikz-cache` (committed, so CI does not recompile every diagram). TikZJax ships a small TeX distribution: a macro it lacks (`\llbracket` from `stmaryrd`, say) has to be defined in the preamble.
- `quartz/plugins/transformers/latex.ts` — every formula is offered to Typst first (**Wypst** syntax), and anything Typst rejects falls through to KaTeX, so a stray LaTeX formula still renders. Pin a page with `math: typst` or `math: latex` frontmatter to forbid the other syntax. The build prints a `Mixed math in …` line for every note that used both.
- `quartz/plugins/transformers/tabs.ts` — **Markdown Tabs** blocks.
- `quartz/plugins/transformers/titleHeading.ts` — drops the `# Note Name` heading each note opens with, since Quartz renders the note name as the title.

The content directory is the repository root, so `crates/**/*.md` and `attic/**/*.md` are published next to `vault/` and the `[[wikilinks]]` in `Code Map.md` resolve. `site/quartz.config.ts` lists what is *not* published: `site/`, `.obsidian/`, `meta/`, this README (the site has its own `index.md`), and the `.rs`/`.jl`/`.toml` sources themselves.

Local preview:

```bash
cd site
npm ci                          # first time only; needs Node 22+
npx quartz build --serve -d ..
```

Then open <http://localhost:8080>. `npm run check` typechecks and checks formatting.

Pushing to `master` runs `.github/workflows/deploy.yml`, which builds the site and publishes it to GitHub Pages (the workflow enables Pages with source "GitHub Actions" on first run). The site is served as a project page of the organisation, under its custom domain: `baseUrl` in `site/quartz.config.ts` is `mathstruct.org/Sophia`.

To pull in upstream changes, merge them into `site/` and keep the four transformer files above plus `quartz.config.ts` and `quartz.layout.ts`.

## Milestones in brief

| | Milestone | State |
| --- | --- | --- |
| M0 | identity: hashing, components, sophialang's core and normaliser | done |
| M1 | the store: SQLite and redb backends, names, lineages, method tables, facts, CLI, benchmarks | done (DuckDB and LMDB move to M2) |
| M2 | **the Moonkale interface**: rendering profiles in the database, the Julia-flavoured printer and parser, `sophia-interface`; four backends compared | done |
| M3a | **Sophia's own query language** (stratified Datalog, Julia-flavoured syntax, content ids, all four backends, magic sets, SQL translation, the `query` dialect for Moonkale) | done, first form |
| M3b | tools: an MCP server for agents over the query API | |
| M4 | running code: interpreter, dispatch facts, cached test results | |
| M5 | compiling: LLVM, minimal binaries | |
| M6 | self-hosting starts | |
| M7 | other languages, abstraction mining, GC and concurrency runtimes | |

Full version: `vault/Plans/Milestones.md`. The earlier roadmap (Metamath, Lisp, cross-language equivalence first) is superseded; see `vault/Reading of the Earlier Vault.md`.

## Databases under evaluation

Four backends sit behind one trait and pass the same conformance suite: SQLite (default), redb, LMDB and DuckDB (`--features lmdb,duckdb`). Measured on synthetic code, LMDB is the fastest, SQLite the smallest and queryable, and DuckDB only suits analytics (`vault/Research/Database Comparison.md`, `vault/Plans/Benchmark Results.md`). LadybugDB, HelixDB, TypeDB, FalkorDB and TursoDB remain candidates.

## On disagreeing with any of this

The store is content-addressed, so it is not trusted: any node can be re-verified by rehashing it. And migration is re-elaboration from core terms rather than a text rewrite. So if you dislike the schema, write a different normaliser (a new hash version) and migrate (SPEC-001 §4.5) — that is a supported operation, not a fork.

## Prior art

The closest existing system is [Unison](https://www.unison-lang.org/), which content-addresses code by the hash of its syntax tree. Sophia borrows that and adds persisted IR layers, a multi-language frontend story, and equivalence edges carrying evidence. What is genuinely new — and genuinely risky — is the last of those; see `vault/Design/Open Problems and Risks.md`.

## License and contributing

MIT ([LICENSE](LICENSE)). How to contribute: [CONTRIBUTING.md](CONTRIBUTING.md); instructions for AI agents: [AGENTS.md](AGENTS.md).
