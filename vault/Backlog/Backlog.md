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
| B-060 | `Interface::accept_text(namespace, text, token)`: apply a `propose_text` text in one transaction: names added, retargeted, renamed (same content) and **removed** (SR-155), memberships, the token checked inside; plus the proposal's guard against concurrent writes (Mistral's F1) | planned | Daniel: build together with B-010? (proposed: yes) | M | SPEC-010 §8; SR-155; [[Review 2026-11-17 Mistral B-003 (external)]] F1, F5 | SPEC-010 §8's acceptance 1 to 4 pass on four backends |
| B-005 | B-15, the rest of the edit loop: the change feed (Moonkale polls every 500 ms), the workspace's re-fetch, paint on Daniel's desktop | planned | Moonkale | M | SR-83, SR-125; [[Milestones]] order item 2 | a measured edit-to-paint latency in [[Benchmark Results]], with Moonkale's revision |
| B-006 | X-1, X-2: where the shared normaliser lives (its own repository or Sophia), and a registry of foreign normalisers instead of the `moonkale-wp66` prefix | proposed | Daniel, Moonkale | L | [[Review 2026-11-10 Fable (external)]] X-1, X-2, X-3 | a decision (S-ADR) Daniel and Moonkale accept; then rows for the move |

### Open findings of Fable's review

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-007 | S-1: verify sophialang records on write (one hash per component); Moonkale records stay `Foreign` | proposed | Daniel | S | Fable S-1 / B-1 | a record whose bytes do not match its id is refused, tested on four backends; cost of the check measured on bulk ingest (B-03) |
| B-008 | S-4: a name bound twice in one file: refuse, or keep the last as Julia does | proposed | Daniel | S | Fable S-4 | Daniel's choice implemented and tested in `folder.rs` |
| B-009 | S-5: `path_from_moonkale` maps `a.b/c` and `a/b/c` to one path: escape `.` in segments, or forbid it | proposed | Daniel, Moonkale | S | Fable S-5 | the mapping is injective and round-trips, with a property test |
| B-010 | SP-008: an edit in one transaction, with the review's acceptance test (a backend that fails the n-th write) | proposed | Daniel | M | [[SP-008 Edits span several transactions]]; Fable | the failing-backend test passes for every n on four backends |
| B-013 | The cost points I-1 to I-9: measure the ones a benchmark can show, change nothing unmeasured | proposed | — | M | Fable 03; SR-126 | each point measured or marked "not measurable", in [[Benchmark Results]] |

### Query engine and agents

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-061 | A covering index for `all_kinds` on SQLite (`t_definition(kind, id)`, or `id` as the primary key): today the scan reads every whole row, 5 of the 7 ms of "count by kind" at 10⁴ | proposed | Daniel (SR-126: storage layout) | S | [[Benchmark Results]] 2026-11-18 (B-014) | count by kind and unreferenced definitions re-measured at 10⁴ and 10⁵; size on disk compared |
| B-015 | SP-014 option 3: query results cached in memory by default | proposed | Daniel | S | [[SP-014 The query result cache is never evicted]] | Daniel's choice; the cache is bounded and tested |
| B-018 | M3b: `sophia-mcp`, the agent server with the query API and edits | partial: split into B-062, B-063, B-064 (done) and B-065 | B-017 done | L (split) | [[Milestones]] M3b; SR-62 | an agent edits sophialang and runs stored queries through MCP; B-11 measured |
| B-067 | Query functions, the mechanism (SPEC-007 §13.2, §13.3, §13.5, §13.6, §13.9): `Literal::Call` and `FnRef::Builtin` replace `Literal::StartsWith`; a function registry with signature records (sorts, `partial`, cost class, `recursion`, SQL template); `query-v1` bytes with `functions: sophia-fn-v0` in the header, `query-v0` still decodes and loads (version-aware canonicity check), stored queries migrated with `migrated_to` (SPEC-006 §6); the prefix range and the parser keyword re-keyed on the builtin call; safety extended to call outputs; planner, magic sets, executor (cost charged to the step budget, undefined calls counted in `Stats`) and SQL translation go through the registry; `startswith` is its only function | proposed | Daniel accepts §13 and answers §13.10 questions 1 and 2 | M | SR-111; [[SPEC-007 Sophia Query Language]] §13 | every existing query test passes with only the ids changed; a stored v0 query loads and runs; the `name` prefix range still applies (no full scan, as tested today); the magic-set differential test passes with `startswith` as a call |
| B-068 | `sophia-fn-v0`, the first library (SPEC-007 §13.8): comparisons, text tests, `sizeof`, `parent`/`basename`, checked `+ - *`, the infix syntax; SQL templates only where exact | proposed | B-067; Daniel answers §13.10 question 3 | M | SPEC-007 §13.8 | per function a test of its stated meaning on edge cases (empty text, non-ASCII, the root path, `i64::MAX`); the differential test (plain, magic, SQL) generates calls and agrees |
| B-069 | Termination with functions (SPEC-007 §13.4): `check` refuses an `Inventive` call that reaches the head of a rule in its own recursive stratum; `Safe` declared per builtin with its reason | proposed | B-068; Daniel answers §13.10 question 4 | S | SPEC-007 §13.4 | `n(y) = n(x) && y == x + 1` refused; recursion through `parent` accepted and terminates on random inputs; a property test checks each `Safe` builtin's result is in its stated finite set |
| B-070 | sophialang functions as query functions, `FnRef::Def(id)` (SPEC-007 §13.7): type mapped to sorts, purity and partiality from an effect fact with evidence, cost and recursion safety from facts, evaluated by the interpreter inside the query budget; the evaluator's semantic version in the query header; the query `refs` the function (`canon::constants` extended to `FnRef::Def`) | planned (later) | B-038 (M4 interpreter), B-067 | M | SPEC-007 §13.7; SR-111 | a stored sophialang function called from a query on every backend; a function without the effect fact refused; the query is a dependant of the function |
| B-020 | SP-009: literal order inside a query body is part of its id | planned (later) | — | S | [[SP-009 Canonical form of queries]] | a canonical literal order, as a migration (SPEC-006 §6) |

### Semantic boundary experiments ([[Semantic Boundary Experiments]], SR-124, SR-139)

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-072 | Moonkale's answer to part 3's two wrongs (a content reference to one method of a function extended elsewhere; `$name` in an `@eval` template read as a use): when the normaliser changes, move the pin (AGENTS.md, "Moonkale") and re-run `tests/dispatch.rs` and the LinearAlgebra ledger | planned | Moonkale | S | B-071, B-073; [[Julia Conformance]] parts 3 and 4 | the fixture's three wrongs become knows or open; LinearAlgebra's 5 spliced uses gone; ids migrated per SPEC-006 §6; `tests/world.rs` re-run (part 4: Sophia proposed that a function name is never a content reference, which would also cure `solo_elsewhere`) |
| B-075 | Moonkale's answer to part 5's wrong (a macro around a definition is in no id: `@inline f`, `@generated f`, `@kwdef struct` and the plain form share one): when the normaliser changes, move the pin (AGENTS.md, "Moonkale") and re-run `tests/macros.rs` and the LinearAlgebra ledger (`julia_macros`) | planned | Moonkale | S | B-074; [[Julia Conformance]] part 5 | the fixture's six `IdIgnoresMacro` become `IdSeesMacro`, `@same` and `@eval` stay `IdNoChange` (or are explained); LinearAlgebra's 202 gone; ids migrated per SPEC-006 §6 |
| B-077 | Moonkale's answer to part 6's wrong (a call's keyword argument name is in no id: `f(; atol = x)` and `f(; rtol = x)` share one, because tree-sitter's `assignment` in a call's argument list is read as a local): when the normaliser changes, move the pin (AGENTS.md, "Moonkale") and re-run `tests/closures.rs` and the LinearAlgebra ledger (`julia_closures`) | planned | Moonkale | S | B-076; [[Julia Conformance]] part 6 | `keyword_names_are_in_no_id` inverted (the pairs get different ids, `kw_t` the same as `kw_a`); `keywords_unspelled` empty on the fixture and LinearAlgebra (127 now); the closure ledger unchanged except that the `Undecided` names passed as keyword arguments (`kwname`'s `atol`, LinearAlgebra's 3 in `isapprox`) become `KnowsCaptured`; ids migrated per SPEC-006 §6 |
| B-079 | Moonkale's answer to part 7's two wrongs (a typed global `x::T = v` at module level is read as a short function definition: it gets an id, a use gets a content reference to a value that changes, and the store files it as a generic function `jl.M.x`; the argument of a callable object's method `(I::T)(n) = …` is read as the module's binding `I`): when the normaliser changes, move the pin (AGENTS.md, "Moonkale") and re-run `tests/globals.rs` and the LinearAlgebra ledger (`julia_globals`) | planned | Moonkale | S | B-078; [[Julia Conformance]] part 7 | the fixture's `WrongContentOfMutable` (bindings `level`, `mode2`; uses at lines 25, 27), `WrongGenericOfMutable` (line 32) and `WrongLocalAsGlobal` (line 72) become `KnowsState` and `KnowsLocal`; `typed_globals_are_stored_as_functions` inverted; LinearAlgebra's 1 wrong (`uniformscaling.jl:84`) gone; ids migrated per SPEC-006 §6 |
| B-084 | Moonkale's answer to part 9's wrongs in the crate's `includes` (only a call `include(…)` with a bare identifier head is an include, anywhere in the tree): `M.include(…)` and `Base.include(M, …)` are missed (not even dynamic, so the module is stated complete without the file); includes in a function body, a `@static`/`if` branch or quoted code are followed as if evaluated at load. When the crate changes, move the pin and re-run `tests/includes.rs` and the LinearAlgebra ledger (`julia_includes`) | planned | Moonkale | S | B-083; [[Julia Conformance]] part 9 | the fixture's `qualified.jl`, `baseinc.jl` (`WrongMissed`) become at least `OpenDynamic`, `never_*.jl` (`WrongExtra`) open or gone; LinearAlgebra still 34 of 34 files known |
| B-085 | Moonkale's answer to E-2's finding: WP-66 v8 refuses Rust `for`, `match`, closures, `while let` and `let`-`else`, so no Rust function with a loop has an id (matrixmultiply: 0 of 18). When the crate changes, move the pin (AGENTS.md, Moonkale) and re-run `examples/gemm_structure.rs`: the in-store rows for matrixmultiply then become comparable | planned | Moonkale | S | B-023; [[Cross-Language Gemm Structure]] | matrixmultiply's labelled definitions have ids, and the in-store predicates are re-scored |
| B-086 | E-2b: compare the **lowered** loop nests of E-2's labelled definitions (rustc MIR or LLVM IR after macro expansion and inlining for Rust; `code_typed` or LLVM IR for Julia): loop trees and memory-access facts (reads, writes, multiply-accumulate), measured outside Sophia first (SR-126) | proposed | Daniel | M | B-023; SR-73 | whether packing, macro-kernel and micro-kernel correspond across the two languages at L4, written up beside E-2 |
| B-081 | Make a proof check's checker and status visible to queries (a base relation such as `checked_by(annotation, checker, version)`, or the status in `annotation`), so a policy "trust only these checkers" is a query; a change of the query schema `store-base-v0` | proposed | Daniel (query schema) | S | B-021 ([[trust]] limits); SPEC-006 §5 | a query keeps metamath.exe ≥ v and drops another checker's verdicts, tested |
| B-082 | `run_stored` describes each distinct id of an answer with its own `Interface::node` read: up to 13 ms on 1 000-row answers (SQLite 10⁵); describe them in one read transaction | proposed | — | S | B-065 ([[Benchmark Results]] 2026-11-19) | the same answers; `run_stored` within about 1 ms of the worker on the 1 000-row workload, measured |
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
