# Contributing to Sophia

Thank you for your interest in Sophia. It is an open-source research prototype: a
content-addressed store for code in every language, with its own language and query language,
designed together with the graph editor Moonkale and the Julia library Lenticulum.jl.

Contributions of every size are welcome, from a corrected number to a new language frontend,
and so are contributions that are not code: a reference, a review, a counterexample, or a
pointed question.

## The state of things, plainly

The project moved fast. Much of the code and most of the notes were written with AI agents, and
not everything has been checked by a person. What is tested says so; a claim in a note that no
test, measurement or independent check backs is a claim, not a result. If you find something
wrong, that is a contribution, and a welcome one: open an issue or fix it.

There are no users yet, so the design can still change a lot. Proposals that make it right are
preferred over ones that keep compatibility.

## Getting started

```sh
nix develop                              # all tools (or install Rust ≥ 1.85 yourself)
cargo test --workspace --all-features    # identity, store conformance on four backends, queries, ingestion
cargo run -p sophia-cli -- sample        # a small sample store
```

The README lists more commands; `vault/Start Here.md` explains how the vault is organised, and
`vault/Status.md` says where things stand.

## Ways to contribute

### Code

- bug fixes, tests, and performance work (with a benchmark in `crates/sophia-bench`);
- language frontends and their conformance checks against the language's own implementation
  (`vault/Languages/`);
- the experiments in `vault/Plans/Semantic Boundary Experiments.md`, each small and checked
  against an oracle;
- storage work: measurements first (`vault/Storage and Materialization/`).

For larger changes, open an issue first so that the design can be agreed on. The open problems
are in `vault/Problems/` (the Problem Log is the index).

### Notes and specifications

The vault is part of the project, not an afterthought. You can correct errors, add references,
point out where a note overstates what is known, or review a specification. A contribution to
the notes counts on its own; it does not need code with it.

## Conventions

- **Rust** for Sophia itself. Measurement scripts are in the language they measure.
- **A note beside every source file** (`x.md` beside `x.rs`), listed in `vault/Design/Code Map.md`.
- **Numbers come with the command that produced them.**
- **Content ids are stable.** Changing what a hash covers is a migration with a version bump.
- **Large data stays outside the repository** (`$SOPHIA_CORPORA`, default
  `~/.cache/sophia-corpora`).

[AGENTS.md](AGENTS.md) has the details; it applies to people as much as to agents.

## Pull requests

Work on a branch of your own (or a fork) and open a pull request into `master`. Please make sure
`cargo test --workspace --all-features` passes and that the notes you touched are still true.
AI assistance is fine; say so in the pull request, and check what the assistant wrote.

## License

By contributing you agree that your contribution is licensed under the [MIT License](LICENSE).
