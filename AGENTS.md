# AGENTS.md

Instructions for AI coding agents working in this repository. Humans: see
[CONTRIBUTING.md](CONTRIBUTING.md); everything here applies to you too.

## Read this first

Sophia is a brainstorm turned into a working prototype, written fast and mostly by AI agents.
Not all of it has been reviewed by a person.

- Treat a claim in a note as unverified unless a test, a measurement or a check against an
  independent implementation in the repository backs it. Check before you build on it.
- When you find something wrong (a stale number, a note that overstates a result, a test that
  does not test what it says), fix it or say so plainly. Do not work around it silently.
- Do not add claims you have not checked. A number goes into a note with the command that
  produced it; anything else is labelled as a proposal, an expectation or a conjecture.
- State support for a language by level (parsed, normalised, resolved, lowered, executable,
  verified: `vault/Languages/Languages.md`), never as "percent of definitions with ids".

Nobody uses Sophia yet: large breaking changes are fine when they make the design right (SR-118).
Do not commit, push or merge unless you are asked to. `master` is the working branch.

## What Sophia is

A content-addressed store for code in every language, with its own language (sophialang) and its
own query language. Its first job is to be the code store of **Moonkale**, a graph editor that
was built as the editor for **Lenticulum.jl**; the three are designed together (SR-114). Start at
`vault/Start Here.md`, then `vault/Status.md`.

## The repository

| where | what |
|---|---|
| `crates/` | the Rust workspace: `sophia-hash` (identity), `sophia-store` (four backends behind one trait), `sophia-query` (the query language), `sophia-interface` (Moonkale's interface), `sophia-ingest` (Metamath, and Julia/Rust/Python through Moonkale's shared normaliser), `sophialang-core`, `sophialang-syntax`, `sophia-cli`, `sophia-bench` |
| `crates/*/src/*.md`, `examples/*.md`, `bin/*.md` | a note beside every source file: what it does, why, what it does not do |
| `vault/` | the Obsidian vault: requirements, specifications, decisions, problems, plans, theory, research, languages, storage |
| `experiments/` | measurement scripts in the language being measured (Julia for AD), each with a note |
| `attic/` | the earlier, comment-only design crates; kept for reference, not built |
| `site/` | the Quartz build of the vault (see the README) |
| `meta/` | the author's prompts and session scratchpads (`meta/scratchpad/`, persistent between agent sessions) |

## Tools and data

```sh
nix develop                 # Rust, sqlite3, zstd, metamath, Julia 1.13 (julia) and 1.12 (julia112)
nix develop .#lean          # + elan
sophia-corpora              # clones set.mm into $SOPHIA_CORPORA (~/.cache/sophia-corpora)
sophia-mathlib              # Mathlib, its build cache, lean4export (several GB)
cargo test --workspace --all-features
```

- Corpora and large stores live in `$SOPHIA_CORPORA`, never in the repository.
- `/tmp` on the author's machine is a 16 GB RAM disk: do not put builds, stores or corpora there.
- Sophia's code is Rust only (S-ADR-0004). Scripts that measure another language's tools
  (`experiments/`, `crates/sophia-ingest/tests/julia-semantics/methods.jl`) are in that language.

## Working conventions

**A note beside every source file.** A new `x.rs` gets `x.md`; a change updates it. List new files
in `vault/Design/Code Map.md`.

**Requirements come from Daniel.** Record what he asked, with the date, in
`vault/Requirements/Requirements.md` (SR-nn). External reviews he forwards are triaged in a
`vault/Research/Review … (external).md` note: each point, whether it was right, what was done.
Your own proposals are marked as proposals; decisions (`S-ADR-nnnn`) stay `proposed` until he
accepts them. Problems go in `vault/Problems/` (SP-nnn) and the Problem Log. Self-contained open
questions that come up while implementing or discussing the theory go in
`vault/Research Questions/` (RQ-nnn, from the template), stated so they can be worked on alone.
Keep `vault/Status.md` current.

**Vault notes.** A tag line, a summary in a blockquote, then `> Sources:` (papers, then `code:`).
Wikilinks resolve by note name, so note names are unique across folders (language pages carry
the language's name: `Julia Name Resolution`).

**Check against an oracle.** A claim about another language or tool is checked against that
tool: metamath.exe for Metamath (`sophia-mm-check`), Julia's own method table for Julia
(`examples/julia_conformance.rs`). Classify each fact as known, approximated, left open or
wrong; every "wrong" is a bug to fix or report.

**Identity is sacred.** Anything that changes a content id (an encoding, a normaliser, hashing)
is a migration (SPEC-006 §6) and needs a version bump. Physical storage may change freely.

**Measure before changing storage.** Document first (`vault/Storage and Materialization/`), then
change the layout with Daniel's agreement (SR-126).

## Moonkale

Julia, Rust and Python ids come from Moonkale's crate `moonkale-content-id`, pinned by git
revision in `crates/sophia-ingest/Cargo.toml`, so a definition has the same id in both tools
(SR-116). When the pin moves:

1. copy `packages/services/content-id/golden` from that Moonkale revision into
   `crates/sophia-ingest/tests/golden-moonkale/` and update `SOURCE.txt`;
2. `cargo test -p sophia-ingest` must reproduce every golden row;
3. re-ingest LinearAlgebra and compare the counts with Moonkale's.

Findings about the shared normaliser go to the Moonkale agent (or as an issue on
MathStruct/Moonkale), not into Sophia-side workarounds.
