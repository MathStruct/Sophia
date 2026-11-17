#overview #planning

> The open tasks of Sophia in one list, how a task is picked and closed, and where finished tasks go. One task per agent session; the handoff to the next session is the top section of [[Status]].

> Sources: original to this repository; [[Milestones]], [[Status]].

**Rows.** `| id | task | status | needs | size | source | done when |`. Status: *planned* (specified or asked for), *proposed* (an idea with a reason, not yet accepted by Daniel), *partial*, *done*. Size: **S** about one session, **M** two to four, **L** many (split before starting). Ids are `B-nnn`, never reused. Priorities are proposals until Daniel sets them.

**Picking a task.** Take the task named under "Next session" in [[Status]]; otherwise the first *planned* row whose needs are done. A specification, requirement (SR-nn), problem (SP-nnn) or review that asks for work gets a row here.

**Closing a task.** When a task is finished: its gates pass; its row moves to `Backlog Done <yyyy-mm>.md` in this folder (one note per month) with the date, the commit and what was verified and how; the "Next session" section of [[Status]] is **overwritten** (date, what was finished, the next task's id, which notes and files it needs, what the next session must know); if the change reaches Moonkale, an entry goes to Moonkale's `HANDOFF-from-sophia.md` (AGENTS.md, "Moonkale"); then the agent reports to Daniel and suggests a fresh session.

**Unattended runs.** When a task's design is written in its row (files, acceptance, nothing left for Daniel to decide), the maintainer's session may queue it for Codex or Mistral: one line `ready  B-002  codex` (or `mistral`, `glm`) in the untracked `QUEUE.md` of the main checkout, then `dispatch-ready sophia` (`--dry-run` first; `--max N`, `--hours H`). The script runs the queued tasks one after another, each on a fresh branch in the agent's worktree, and leaves `HANDOFF-from-<agent>-<id>.md` and a summary in `HANDOFF-from-dispatch.md` in the main checkout. It never merges or edits Status or the backlog: the next Opus session reviews each branch (and has the other model review it), merges, and closes the row as above.

## Open

The order below is a proposal (2026-11-17, B-001), following SR-125 (Moonkale first) and the order of [[Milestones]]; Daniel sets the priorities. Rows marked *proposed* wait for his decision before work starts.

### Moonkale and the folder store (SR-125)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-002 | Annotations move with the truth: `give_truth(…, exported_annotations)` refuses with `AnnotationsNotExported`; `take_truth(…, annotations)` stores them in the same transaction; `annotate_many` skips an annotation already stored | planned | — | M | SR-151; [[SPEC-010 Store-Truth Folders and Generated Files]] §7 (accepted 2026-10-30) | tests on all four backends for each of the three rules; a re-sync logs nothing; Moonkale told the calls exist |
| B-003 | `Interface::propose_text(path, text, token)` for sophialang files, returning a `Proposal` | planned | — | M | SPEC-010 §5, acceptance 3 | SPEC-010 acceptance 3 passes on all four backends |
| B-004 | Specify sharing a store-truth folder: the text export committed beside the generated files and the rebuild from it (option b); the pointer-file idea as a later option | planned | — | S | SR-149 | a SPEC-011 draft Daniel can accept; SR-149's status points to it |
| B-005 | B-15, the rest of the edit loop: the change feed (Moonkale polls every 500 ms), the workspace's re-fetch, paint on Daniel's desktop | planned | Moonkale | M | SR-83, SR-125; [[Milestones]] order item 2 | a measured edit-to-paint latency in [[Benchmark Results]], with Moonkale's revision |
| B-006 | X-1, X-2: where the shared normaliser lives (its own repository or Sophia), and a registry of foreign normalisers instead of the `moonkale-wp66` prefix | proposed | Daniel, Moonkale | L | [[Review 2026-11-10 Fable (external)]] X-1, X-2, X-3 | a decision (S-ADR) Daniel and Moonkale accept; then rows for the move |

### Open findings of Fable's review

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-007 | S-1: verify sophialang records on write (one hash per component); Moonkale records stay `Foreign` | proposed | Daniel | S | Fable S-1 / B-1 | a record whose bytes do not match its id is refused, tested on four backends; cost of the check measured on bulk ingest (B-03) |
| B-008 | S-4: a name bound twice in one file: refuse, or keep the last as Julia does | proposed | Daniel | S | Fable S-4 | Daniel's choice implemented and tested in `folder.rs` |
| B-009 | S-5: `path_from_moonkale` maps `a.b/c` and `a/b/c` to one path: escape `.` in segments, or forbid it | proposed | Daniel, Moonkale | S | Fable S-5 | the mapping is injective and round-trips, with a property test |
| B-010 | SP-008: an edit in one transaction, with the review's acceptance test (a backend that fails the n-th write) | proposed | Daniel | M | [[SP-008 Edits span several transactions]]; Fable | the failing-backend test passes for every n on four backends |
| B-011 | Check the unchecked findings S-9 (deleting a namespace leaves sub-namespace markers) and S-11 (a removed fact reaches subscribers with no keys); fix what is right | planned | — | S | Fable S-9, S-11 | each one has a test that fails before its fix, or is recorded as not reproduced in the triage note |
| B-012 | wasm tests in CI (today run by hand under Node) | planned | — | S | Fable "not done" | the CI workflow runs the wasm tests and passes |
| B-013 | The cost points I-1 to I-9: measure the ones a benchmark can show, change nothing unmeasured | proposed | — | M | Fable 03; SR-126 | each point measured or marked "not measurable", in [[Benchmark Results]] |

### Query engine and agents

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-014 | The set-oriented native executor: a prefix access path, anti-joins for negation over scans, borrowed field reads | planned | — | M | [[Milestones]] order item 3 | B-17 re-run, differential tests unchanged, results in [[Benchmark Results]] |
| B-015 | SP-014 option 3: query results cached in memory by default | proposed | Daniel | S | [[SP-014 The query result cache is never evicted]] | Daniel's choice; the cache is bounded and tested |
| B-016 | SP-010: finer query snapshots, so a logged change invalidates only the results that read it | planned | — | M | [[SP-010 Query snapshots are coarse]]; SR-98 (incremental maintenance) | a change outside a query's relations keeps its cached result, tested |
| B-017 | A hard memory bound for untrusted queries, from outside the executor (SQLite's `sqlite3_hard_heap_limit64` is process-wide; a subprocess or allocator limit) | planned | — | M | SR-108; Fable S-3 remainder | a query that exceeds the bound fails cleanly on every backend, tested |
| B-018 | M3b: `sophia-mcp`, the agent server with the query API and edits | planned | B-017 | L (split) | [[Milestones]] M3b; SR-62 | an agent edits sophialang and runs stored queries through MCP; B-11 measured |
| B-019 | Typed, pure, cost-annotated functions called from queries through one mechanism | planned | — | M | SR-111; [[SPEC-007 Sophia Query Language]] §11 | a written design in SPEC-007, then rows for the build |
| B-020 | SP-009: literal order inside a query body is part of its id | planned (later) | — | S | [[SP-009 Canonical form of queries]] | a canonical literal order, as a migration (SPEC-006 §6) |

### Semantic boundary experiments ([[Semantic Boundary Experiments]], SR-124, SR-139)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-021 | E-0: Metamath proof checking as SPEC-006 §5 trust records; "which theorems rest only on checked proofs" as a query | planned | — | S | E-0 | its measures recorded, pass or fail stated |
| B-022 | E-1: Julia conformance, parts 3–9 (dispatch, world change, macros, closures, mutable globals, parser agreement, include order) | planned | — | L (split per part) | E-1; SR-129 | each part classified known / approximated / open / wrong in [[Julia Conformance]], no unexplained "wrong" |
| B-023 | E-2: cross-language structure, Rust `matrixmultiply` vs Julia matmul | planned | — | M | E-2; SR-73 | the structural query's result written up, whatever it is |
| B-024 | E-3: dispatch facts from Julia, Sophia's stale set vs Julia's invalidations | planned | — | M | E-3; SP-006, SR-85 | precision and recall recorded |
| B-025 | E-4: macro expansions as transformation records (LinearAlgebra's 1 059 unseen methods) | planned | — | M | E-4 | methods recovered and costs recorded |
| B-026 | E-5: rules vs derived derivatives; id(D(f)) before running D | planned | — | M | E-5 | equal keys give equal outputs, or the hidden input is named |
| B-027 | E-6: one derivative at two layers (Zygote vs Enzyme) | planned | B-026 | M | E-6 | stored at both layers with a `tested` equivalence, or a specification result |
| B-028 | E-7: staged compilation of a small Lux training step with Reactant | planned | — | S | E-7 | compile time, artifact size and reproducibility recorded |
| B-029 | E-10: Mathlib repeated statements and a local `lean4checker` re-check | planned | — | M | E-10; SR-137 | repeat groups classified; re-check time recorded |
| B-030 | SP-013: a binding table from Julia as an environment artifact (86 wrong owners in LinearAlgebra) | proposed | — | M | [[SP-013 Julia bindings of modules Sophia has not ingested]] | the 86 owners are right, checked against Julia's method table |
| B-031 | E-8: full Lux.jl | planned | B-026, B-027, B-028 | L | E-8 | E-1 to E-7's facts checked on Lux |
| B-032 | E-9: Lenticulum.jl runs in Sophia | planned (later) | B-031 | L | E-9; SR-115 | — (split when started) |

### Storage (measure first, layout changes need Daniel, SR-126)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-033 | M-1, M-2: a smaller layout and rebuild costs, measured | planned | — | M | [[Storage Measurements]]; SR-120 | measurements in the note; a layout proposal for Daniel |
| B-034 | Compact local keys for reference edges (42 % of set.mm's store) | proposed | B-033, Daniel | M | SR-132; [[SP-011 Reference edges dominate storage on real code]] | measured gain; changed only with Daniel's agreement |
| B-035 | The fact cache experiment: what enters it (every content-only fact, a cost threshold, off), default chosen by measurement | planned | — | M | SR-152; [[SPEC-001 Sophia Store v0]] §8a | the experiment's result and a default proposed to Daniel |
| B-036 | B-02's flat-layout comparison | planned | a flat layout | S | [[Milestones]] M0 | B-02 run on both layouts |

### Language: sophialang (M4 to M6, SR-154)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-037 | Specify the third part of sophialang, a language for defining type systems | planned | — | L | SR-154; [[RQ-004 A Language for Defining Type Systems]] | a draft specification Daniel can accept |
| B-038 | M4: interpreter over core terms, dispatch resolution as facts, tests as cached facts, a simple checker; P0 demonstration 3 end to end | planned | — | L (split) | [[Milestones]] M4; SR-54, SR-85, SR-89 | M4's "done when" and the P0 gate's demonstration 3 with dispatch |
| B-039 | SP-006: generic functions nominal, dispatch as derived facts, invalidation by signature overlap | planned | B-024 | M | [[SP-006 Open method tables versus content addressing]]; SR-85 | B-09 invalidation precision measured |
| B-040 | Macro hygiene, phases and quotation, specified | planned | — | M | SR-86 | a specification section accepted by Daniel |
| B-041 | SP-001: comments inside a definition body survive printing back | planned | — | M | [[SP-001 Comments inside a definition body]] | a body comment round-trips through both profiles |
| B-042 | SP-007: an `Index` type, so 0- or 1-based indexing can be shown | planned | — | M | [[SP-007 Index base needs an Index type]] | the Julia and Rust profiles show the same index correctly |
| B-043 | M5: lowering to LLVM, minimal binaries, a JIT cache keyed by content id and argument types | planned | B-038 | L | [[Milestones]] M5; SR-11, SR-60 | M5's "done when" |
| B-044 | M6: self-hosting starts (the printer in sophialang, the `compile` DSL, the dependent checker; later the query DSL in sophialang) | planned | B-038 | L | [[Milestones]] M6; SR-55, SR-97 | M6's "done when" |
| B-045 | WebAssembly: (a) sophialang compiled to wasm; (b) Sophia's crates running in a browser (they pass under Node; nothing has run in a browser) | planned (later) | (a) B-043 | M / L | SR-140 | (b) a store opened and queried in a browser; (a) after M5 |

### Store semantics, later

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-046 | SP-002: propagating an edit of `f` to its dependants | planned (later) | — | M | [[SP-002 Propagating an update to dependants]] | a written design, then rows |
| B-047 | SP-004: operations that are not file-shaped (annotations, structural edits, rename as one entry, Fable S-15) | planned (later) | — | M | [[SP-004 Moonkale operations are file-shaped]] | an `Op` for each, with Moonkale |
| B-048 | SP-005: lineages under merge and fork | planned (later) | branches | M | [[SP-005 Lineages under merge and fork]] | merge rules specified |
| B-049 | Two identities declared equivalent with a proof, instead of merged | planned (later) | — | M | SR-02 | specified |
| B-050 | Attach proofs, optimisations, compiled code and benchmarks to queries (possible through annotations; nothing produces them) | planned (later) | B-018 | S | SR-93 | one producer exists and is tested |

### Vault and documentation

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-051 | Restructure [[Status]] into a current state, moving the history to a dated log | proposed | Daniel | S | Fable D-3 | Status fits on one screen and links the log |
| B-052 | README: describe `sophia-ingest`'s Julia, Rust and Python support by level | planned | — | S | Fable D-4 (not rechecked) | the README states the levels of [[Languages]] |
| B-053 | SR-17: confirm or drop the original database candidates (LadybugDB, HelixDB, TypeDB, FalkorDB, TursoDB); four backends are compared | proposed | Daniel | S | SR-17 | SR-17 marked done or reworded |

### Long-term directions (not planned; listed so they have a row)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-054 | Many languages in one graph, beyond Moonkale's ingest (C/C++, Lean, …) | proposed | — | L | SR-03 | — |
| B-055 | E-graphs, IRs and DSLs inside the graph | proposed | — | L | SR-10 | — |
| B-056 | Code graph meets a knowledge graph (notes, papers, concepts) | proposed | — | L | SR-20 | — |
| B-057 | Analytics and ML on the code graph; precompilation analytics; improving Julia's precompilation | proposed | — | L | SR-14, SR-24, SR-66 | — |
| B-058 | Old software usable; Sophia as Moonkale's main parsing engine | proposed | — | L | SR-15, SR-74 | — |
| B-059 | Abstraction mining, a GC and scheduler runtime, memory and concurrency DSLs, a Lisp or Metamath frontend | proposed | — | L | [[Milestones]] M7; SR-73 | each an experiment with a written result |
