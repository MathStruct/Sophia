#overview #implementation

> Sources: index of the per-file design notes

Every source file in the repository has a markdown sibling of the same name describing what it is for. Because the vault root is the repository root, those siblings are ordinary vault notes and link into the rest of the design. This note is the index of them.

**All code files are comments only.** Nothing is implemented, nothing is meant to run. See [[Repository Layout]] for the tree and the Rust/Julia split, and [[Roadmap]] for the order in which these would actually be written.

## Rust — `crates/` (implemented, 2026-10-20)

| Crate | File | Note | Role |
| --- | --- | --- | --- |
| `sophia-hash` | `sophia_hash.rs` | [[sophia_hash]] | identity: framing, digests, components; language-agnostic |
| `sophialang-core` | `debug.rs` | [[debug]] | `Debug` in derive's layout without recursion (SP-015) |
| `sophialang-syntax` | `tests/deep.rs` | [[parser]] | every pass at depth 100 000 on a 256 KiB stack |
| | `canonical.rs` | [[canonical]] | the tag-length-value framing shared with Moonkale |
| | `component.rs` | [[component]] | SCCs and the canonical order of recursive groups |
| | `merkle.rs` | [[merkle]] | component, member and nominal digests; `Record` |
| | `digest.rs` | [[digest]] | the 256-bit digest type |
| `sophialang-core` | `sophialang_core.rs` | [[sophialang_core]] | sophialang's core calculus and normaliser |
| | `term.rs` | [[term]] | core terms |
| | `definition.rs` | [[definition]] | definition kinds and bodies |
| | `encode.rs` | [[encode]] | canonical encoding, decoding, display data |
| | `normalise.rs` | [[normalise]] | batches of drafts → records |
| | `samples.rs` | [[samples]] | the 16-definition sample program |
| `sophia-store` | `sophia_store.rs` | [[sophia_store]] | the store over interchangeable backends |
| `sophia-store` | `folder.rs` | [[folder]] | folder stores (SPEC-009): `replace_files`, file manifests, the derived mode, `text_source` |
| `sophia-store` | `tests/folder.rs` | [[folder]] | `replace_files` on four backends |
| `sophia-store` | `examples/folder_garbage.rs` | [[folder_garbage]] | how much of a folder store no current file reaches (before garbage collection) |
| | `backend.rs` | [[backend]] | the backend traits |
| | `logic.rs` | [[logic]] | the store's rules |
| | `types.rs` | [[types]] | names, log, nominal entities, lineages, annotations, facts |
| | `codec.rs`, `query.rs`, `error.rs` | [[codec]], [[query]], [[error]] | value encoding, query results, errors |
| | `sqlite.rs` | [[sqlite]] | SQLite backend with the read-only guard |
| | `redb_store.rs`, `lmdb_store.rs`, `kv.rs` | [[redb_store]], [[lmdb_store]], [[kv]] | key-value backends and their shared logic |
| | `duckdb_store.rs` | [[duckdb_store]] | DuckDB backend (analytics) |
| `sophialang-syntax` | `sophialang_syntax.rs` | [[sophialang_syntax]] | profiles, printer, parser; the round-trip guarantee |
| | `lexer.rs`, `parser.rs`, `ast.rs` | [[lexer]], [[parser]], [[ast]] | text → surface tree |
| | `elab.rs`, `resolve.rs` | [[elab]], [[resolve]] | surface tree → core; shared name resolution |
| | `print.rs`, `profile.rs`, `diagnostic.rs` | [[print]], [[profile]], [[diagnostic]] | core → text; profiles; diagnostics |
| `sophia-interface` | `sophia_interface.rs` | [[sophia_interface]] | what Moonkale's adapter wraps (SPEC-004) |
| | `edit.rs`, `profiles.rs` | [[edit]], [[profiles]] | edits through text; profiles in the database |
| | `propose.rs` | [[propose]] | an outside edit of an exported sophialang file as a proposal (SPEC-010 §5) |
| | `tests/propose_text.rs` | [[propose]] | `propose_text` against the specification, compared with real edits (Mistral's review probes) |
| `sophia-query` | `sophia_query.rs` | [[sophia_query]] | Sophia's own query language (SPEC-007): prepare, run, SQL, cache, store |
| `sophia-query` | `bounded.rs` | [[bounded]] | a hard memory bound for untrusted queries: a worker process under `RLIMIT_DATA` (B-017) |
| `sophia-query` | `bin/sophia-query-worker.rs` | [[sophia-query-worker]] | the worker binary of `bounded` |
| `sophia-mcp` | `sophia_mcp.rs` | [[sophia_mcp]] | the agent server: MCP tools on stdio, built on `rmcp`, in permission classes behind startup flags; queries through `bounded` (B-062, B-063) |
| `sophia-mcp` | `backends.rs` | [[backends]] | the store on any of the four backends behind one `Backend` enum; which backends serve writes (B-064) |
| `sophia-mcp` | `bin/sophia-mcp.rs` | [[sophia-mcp]] | the `sophia-mcp` command |
| `sophia-mcp` | `bin/sophia-mcp-worker.rs` | [[sophia-mcp-worker]] | the server's copy of the query worker |
| | `ir.rs`, `syntax.rs`, `canon.rs` | [[ir]], [[syntax]], [[canon]] | the IR; Julia-flavoured syntax and printers; canonical form and ids |
| | `check.rs`, `plan.rs`, `magic.rs` | [[check]], [[plan]], [[magic]] | sorts, safety, strata; literal ordering; the magic-set rewrite |
| | `eval.rs`, `sql.rs` | [[eval]], [[sql]] | the native executor; the SQL translation |
| | `tests/set_oriented.rs` | [[eval]] | the prefix path and anti-joins against tuple at a time (every backend) and SQL (B-014) |
| | `tests/review_mistral_b016.rs` | [[sophia_query]] | every op invalidates the cached results that read what it changes, and only those (SP-010 step 1, B-016; Mistral's review probes) |
| `sophia-ingest` | `sophia_ingest.rs`, `metamath.rs`, `code.rs`, `module.rs` | [[sophia_ingest]], [[crates/sophia-ingest/src/metamath\|metamath]], [[code]], [[module]] | frontends for real code (B-16): Metamath into statements, proofs and library collections; Julia, Rust and Python through Moonkale's shared WP-66 normal form; generic functions keyed by (module, name) |
| | `source.rs` | [[source]] | source files as records and the ingest manifest; `reconstruct` rebuilds and compares every record |
| | `examples/lean_export_stats.rs` | [[lean_export_stats]] | measures a lean4export file (statements, programs, proofs) |
| | `examples/lean_statements.rs` | [[lean_statements]] | E-10: repeated Lean statements by level of equality and cause; statement vs proof dependencies; axioms behind theorems |
| | `examples/lean_layout.rs` | [[lean_layout]] | M-1, M-2: lean copies of a SQLite store, verified by rebuilding from their sources; rebuild costs per file and definition |
| | `examples/fact_cache.rs` | [[fact_cache]] | B-035: fact cache hit rates over git histories and fresh checkouts; cache store size and lookup time |
| | `examples/lean_elab_costs.rs` | [[lean_elab_costs]] | B-035 part 2: Lean elaboration cost per command and declaration on a Mathlib sample, through [[elab_costs]] |
| | `examples/gemm_structure.rs`, `examples/gemm_structure.tsv` (role labels) | [[gemm_structure]] | E-2: name-free loop predicates on Rust and Julia gemm code, on the source and in the store, against role labels ([[Cross-Language Gemm Structure]]) |
| | `examples/julia_conformance.rs`, `tests/julia-semantics/methods.jl` | [[julia_conformance]], [[methods]] | Sophia's generic-function grouping against Julia's own method table ([[Julia Conformance]]) |
| | `src/conformance.rs`, `examples/julia_dispatch.rs`, `tests/dispatch.rs`, `tests/julia-semantics/calls.jl`, `tests/julia-semantics/dispatch/` | [[conformance]], [[julia_dispatch]], [[calls]] | part 3 of the conformance suite: what Sophia says a name in a body refers to, against Julia's lowered code ([[Julia Conformance]]) |
| | `tests/world.rs`, `tests/julia-semantics/world/` (`world_ledger` in `src/conformance.rs`) | [[conformance]], [[calls]] | part 4: what a method or an import added after loading does to Sophia's facts |
| | `tests/macros.rs`, `tests/julia-semantics/macros.jl`, `tests/julia-semantics/macros/`, `examples/julia_macros.rs` (`macro_ledger` in `src/conformance.rs`) | [[conformance]], [[macros]], [[julia_macros]] | part 5: what a module-level macro call makes in Julia against Sophia's definitions, and whether the id sees the macro |
| | `tests/closures.rs`, `tests/julia-semantics/closures.jl`, `tests/julia-semantics/closures/`, `examples/julia_closures.rs` (`closure_ledger` in `src/conformance.rs`) | [[conformance]], [[closures]], [[julia_closures]] | part 6: how Sophia's encoding binds each name in a closure (captured, the closure's own, a module name) against Julia's closure; keyword names the encoding loses |
| | `tests/globals.rs`, `tests/julia-semantics/globals.jl`, `tests/julia-semantics/globals/`, `examples/julia_globals.rs` (`global_ledger` in `src/conformance.rs`) | [[conformance]], [[globals]], [[julia_globals]] | part 7: whether Sophia treats each global binding that holds data as content (a `const`) or state (a mutable global), against Julia's binding partition; locals spelled like a binding |
| | `tests/parse.rs` (+ `parse.md`), `tests/julia-semantics/parse.jl` (+ `parse.md`), `tests/julia-semantics/parse/Project.toml`, `parse/src/Pars.jl`, `parse/src/More.jl` (+ their notes), `parse/truth.tsv`, `examples/julia_parse.rs` (+ `julia_parse.md`); `parse_ledger` in `src/conformance.rs` | [[conformance]], [[parse]], [[julia_parse]] | part 8: per-definition JuliaSyntax/tree-sitter parser agreement, refusals, silent differences and recovery diagnostics |
| | `tests/includes.rs` (+ `includes.md`), `tests/julia-semantics/includes.jl` (+ `includes.md`), `tests/julia-semantics/includes/Project.toml`, `includes/src/Inc.jl` (+ `Inc.md`) and the files it includes, `includes/truth.tsv`, `examples/julia_includes.rs` (+ `julia_includes.md`); `include_ledger` in `src/conformance.rs` | [[conformance]], [[includes]], [[julia_includes]] | part 9: include order, files and methods placed by Sophia's include graph against Julia's `include_callbacks` |
| | `src/expansion.rs`, `tests/expansions.rs`, `tests/julia-semantics/expansions.jl`, `tests/julia-semantics/expansions/` (`src/Exp.jl` + `Exp.md`, `truth.tsv`, `macros.tsv`), `examples/julia_expansions.rs` | [[expansion]], [[expansions]], [[julia_expansions]] | E-4: Julia's expansions of module-level macro calls, spliced in and ingested; methods recovered, transformation records with input addresses ([[Semantic Boundary Experiments]] §E-4) |
| | `src/derivative.rs`, `tests/derivatives.rs`, `tests/julia-semantics/derivatives.jl`, `tests/julia-semantics/rules/` (`Project.toml`, `Manifest.toml`, `Rules.md`, `variants/*/`, `truth.tsv`), `examples/julia_derivatives.rs` | [[derivative]], [[derivatives]], [[Rules]], [[julia_derivatives]] | E-5: three keys for Zygote's derived programs, computed without running it, scored against what Zygote builds |
| | `src/layers.rs`, `tests/layers.rs`, `tests/julia-semantics/layer_programs.jl`, `tests/julia-semantics/layers/` (`Project.toml`, `Manifest.toml`, `truth.tsv`, `reverse.tsv`), `examples/julia_layers.rs` | [[layers]], [[layer_programs]], [[julia_layers]] | E-6: one derivative at two layers (Zygote at Julia IR, Enzyme at LLVM IR) stored as records with a fact and a `tested` equivalence annotation; Enzyme's module text with addresses and session numbers normalised |
| | `src/invalidation.rs` (+ `invalidation.md`), `tests/invalidation.rs`, `tests/julia-semantics/dispatch_facts.jl` (+ `dispatch_facts.md`), `tests/julia-semantics/invalidation/` (`Inval.jl`, `setup.jl`, `changes/`, `truth/`; `linalg/setup.jl` and `linalg/changes/` for LinearAlgebra), `examples/julia_invalidation.rs` (+ `julia_invalidation.md`) | [[invalidation]], [[dispatch_facts]], [[julia_invalidation]] | E-3: dispatch facts recorded from Julia; Sophia's stale set for an added method by signature overlap (and specificity) against what changed and Julia's invalidation log |
| | `src/trust.rs`, `tests/trust.rs` | [[trust]] | proof trust records (SPEC-006 §5, E-0): a checker's verdict per proof as annotations; the queries "rests only on checked proofs" and its complement |
| `sophia-interface` | `examples/edit_timing.rs` | [[edit_timing]] | B-15's steps on `Interface` directly, to separate Sophia's share |
| `sophia-interface` | `examples/listing_timing.rs` | [[listing_timing]] | `children` and `all` on a large store (Moonkale's Explorer calls) |
| `sophia-interface` | `tests/wasm_read.rs` | [[sophia_interface]] | the read path on wasm32 under Node, from a store's bytes (SPEC-009 §4) |
| `sophia-interface` | `tests/wasm_deep.rs` | [[sophia_interface]] | every pass at depth 100 000 in wasm (Node, Firefox) |
| `sophia-interface` | `tests/wasm_fixture.rs`, `tests/fixtures/web.sophia` | [[sophia_interface]] | the fixture store, and `from_bytes` natively |
| `experiments/b15-edit-loop` | `run.sh` | [[run]] | B-15: Moonkale's `edit_loop` on synthetic stores of 10³–10⁵ definitions |
| `sophia-bench` | `sophia_bench.rs`, `bin/sophia-bench-report.rs`, `bin/sophia-bench-mcp.rs` | [[sophia_bench]], [[sophia-bench-report]], [[sophia-bench-mcp]] | the benchmark suite |
| | `bin/sophia-mm-check.rs` | [[sophia-mm-check]] | Metamath dependency answers against metamath.exe |
| | `bin/sophia-mm-trust.rs` | [[sophia-mm-trust]] | E-0: checks set.mm with metamath.exe, records trust annotations, times the trusted-closure query |
| `experiments/ad-materialization` | `ad_costs.jl` | [[ad_costs]] | E-AD-0: cost of producing vs running a derivative (Julia) |
| | `reactant_persist.jl` | [[reactant_persist]] | E-AD-1: split the first call; restore a compiled derivative in a fresh process |
| | `pkgimage_reuse.jl`, `pkgimage_control.jl`, `ADCache/` | [[pkgimage_reuse]], [[pkgimage_control]], [[ADCache]] | E-AD-1: the same through Julia's package image (fails) |
| `experiments/staged-compilation` | `lux_train_step.jl` | [[lux_train_step]] | E-7: a Lux training step compiled by Reactant: compile vs step time, sizes, reproducibility, restore |
| | `executable_objects.py` | [[executable_objects]] | E-7: compares XLA CPU executables after sorting their object files |
| `experiments/lean-recheck` | `recheck.sh` | [[recheck]] | E-10: re-checks every Mathlib module with Lean's kernel (`leanchecker`), per module |
| `experiments/lean-elab-costs` | `elab_costs.lean` | [[elab_costs]] | B-035 part 2: times each source command of one Lean file through Lean's frontend, synchronously |
| `sophia-cli` | `main.rs` | [[main]] | the `sophia` command |

The comment-only crates of the first vault (`sophia-core`, `sophia-equiv`, `sophia-emit`, `sophia-frontend-metamath`) are kept as ideas in `attic/`: [[sophia_core]], [[elaborate]], [[effects]], [[sophia_equiv]], [[egraph]], [[witness]], [[sophia_emit]], [[metamath]].

## Julia

The comment-only Julia skeleton (`src/*.jl`, `Project.toml`) was deleted on 2026-10-20 (S-ADR-0004: Rust only). Julia may come back much later, for analytics on the code graph (SR-24).

## Reading order for someone new

1. [[Status]] and [[SPEC-001 Sophia Store v0]]: what the system is now
2. [[sophia_hash]] → [[canonical]] → [[component]] → [[merkle]]: how identity works, and what can go wrong with it
3. [[sophialang_core]] → [[term]] → [[encode]] → [[normalise]]: the first language
4. [[sophia_store]] → [[backend]] → [[logic]] → [[sqlite]] / [[redb_store]]: how it is stored, and the measurements ([[Benchmark Results]])

## Where the risk is concentrated

Three files, for three different reasons:

- [[canonical]] and [[encode]] — the only silent failure mode in the system. An over-coarse canonical form identifies two different programs and nothing downstream detects it.
- [[sophia_store]] — carries the empirical questions of [[Database Comparison]] and that could invalidate the architecture: whether querying IR back out is competitive with regenerating it ([[Open Problems and Risks]]).
- [[sophia_equiv]] — where a careless default turns the project into a miscompilation generator, via the congruence problem in [[Equivalence and Witnesses]].

## Related

- [[Repository Layout]] · [[Design Overview]] · [[Roadmap]]
- [[Tests and Documentation as Nodes]] — these paired files are a manual, filesystem-level version of the `DOCUMENTS` edge
| `sophia-ingest` | `tests/bindings.rs` | [[code]] | Julia names equal Moonkale's golden `bindings.tsv` (SPEC-009 §8) |
