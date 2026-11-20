Review a new measurement program for defects only. Do not edit any file. Do not run cargo builds or the program (the machine is busy); read the code.

Repository: /home/daniel/Code/CategoryTheory/Sophia (Rust workspace).
Files under review (new, untracked):
- crates/sophia-ingest/examples/fact_cache.rs  (the program)
- crates/sophia-ingest/examples/fact_cache.md  (its specification: what it measures and how)
- vault/Storage and Materialization/Fact Cache Experiment.md (the results note; check that its claims follow from the program and the numbers it quotes)

Requirement behind it: vault/Design/... no — read section "8a" of the note `SPEC-001 Sophia Store v0.md` (find it under vault/), the paragraph "The experiment (planned, not run)": measure size, hit rate across folders, and lookup cost of a per-user cache of content-only facts.

APIs it uses: crates/sophia-ingest/src/code.rs (`code::file`), crates/sophia-store/src/logic.rs (`Store::record_facts`, `Store::read`), crates/sophia-store/src/types.rs (`Fact`, `Input`), crates/sophia-store/src/folder.rs (how a folder store keeps facts per file).

Report only defects: wrong counts (e.g. a definition counted in the wrong category, the live multiset updated wrongly, first commit handling, files deleted/renamed, duplicate ids within a file or event), wrong timing or size measurement, panics on plausible input (non-UTF-8 files, submodules, symlinks, empty repos, git output parsing), a check that does not check what it claims, a claim in the notes not supported by the program. For each: file:line, the concrete failing scenario, and why it is wrong. Mark what you verified by reading versus what you suspect. No style remarks. If you find nothing, say so.
