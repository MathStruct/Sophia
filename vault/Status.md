#overview

> What exists, what is being decided, and what is waiting. Check the date. Where the repository disagrees with this page, the repository wins, so fix this page.

**As of 2026-10-28.**

- **Direction:** Sophia is the **store**; sophialang is its **language** (S-ADR-0007). The first priority is the **Moonkale interface** ([[SPEC-004 Moonkale Interface]]). The plan is [[Milestones]] M0 to M7.
- **Code:** **M0 and M1 done** (2026-10-20):
  - `sophia-hash`, `sophialang-core` and `sophia-store` (SQLite and redb), plus `sophia-bench` and the `sophia` CLI;
  - 58 tests pass;
  - first benchmarks in [[Benchmark Results]].
  
  **M2's interface is done (2026-10-20):**
  - `sophialang-syntax` (the Julia and Rust profiles round-trip);
  - rendering profiles in the database;
  - `sophia-interface` (SPEC-004 §9 on both backends);
  - Moonkale has been sent the revision.

  **M2 is complete:** four backends compared ([[Database Comparison]]).

  **M3a, Sophia's own query language, is done in its first form (2026-10-20),** after Daniel's query-language proposal ([[Review 2026-10-20 Query Language (external)]]):
  - `sophia-query`: stratified Datalog over base relations, a Julia-flavoured syntax, content ids, a native executor on all four backends, magic sets, SQL translation ([[SPEC-007 Sophia Query Language]]);
  - Moonkale's `query` dialect, and queries stored, printed and edited as definitions;
  - B-17 measured ([[Benchmark Results]]);
  - S-ADR-0010 accepted in amended form (SQLite default, LMDB experimental, no permanent choice yet); S-ADR-0011 accepted; S-ADR-0012 proposed.

  **M3a stabilised (2026-10-20)** after its review ([[Review 2026-10-20 M3a (external)]]): cache race fixed, exact canonical form, resource budget, property and differential tests.

  **B-16 rung 1 done (2026-10-20):** Metamath set.mm and iset.mm in Sophia, with real queries (the axioms behind 2 + 2 = 4, per library) and four design corrections learned from real data ([[Real Code Corpora]] §4). Daniel: Moonkale, Lenticulum and Sophia are to be thought of together; Lenticulum running in Sophia is a later goal (SR-114, SR-115).

  **B-16 rung 2 started (2026-10-20):** Julia LinearAlgebra through the normaliser shared with Moonkale (identical ids on Moonkale's fixtures). 69 % of real definitions got an id under v2; Moonkale's v4 (loops, scopes, varargs, destructuring) raised it to **99 %** ([[Real Code Corpora]] §5).

  **v6 in both tools (2026-10-20):** Sophia and Moonkale give identical ids on LinearAlgebra (3 435), with module-level references (2 469, 948 across files).

  **After the real-code review (2026-10-20,** [[Review 2026-10-20 Real Code (external)]]**):**
  - **Metamath checked:** Sophia's "which axioms does T rest on" agrees with metamath.exe on every theorem of set.mm and iset.mm (64 566) ([[Metamath Overview]]);
  - **generic functions are (module, name):** a real bug, fixed; agreement with Julia's own method table on LinearAlgebra 53.7 % → 95.1 %; the check found a bug in the shared normaliser, fixed by Moonkale in **v7**, which Sophia now uses (3 416 identical ids) ([[Julia Conformance]]);
  - **record kinds enforced** (SP-012);
  - **new sections:** [[Storage and Materialization]] (finding: the store is about 10× its source and mostly derivable) and [[Languages]] (support levels per language);
  - **AD measured** (E-AD-0): producing a derivative costs 10² to 10⁶ runs of it ([[AD as a Materialization Case]]).

  **After the second real-code review (2026-10-20,** [[Review 2026-10-20 Real Code 2 (external)]]**):**
  - **reconstruction before compaction:** sources and an ingest manifest are stored; every LinearAlgebra (3 284) and set.mm (98 136) record rebuilds identically from the store alone, tested on four backends;
  - **a compiled derivative restored in a fresh process** (Reactant, bit-identical); Enzyme via Julia's pkgimage segfaults;
  - **Mathlib exported and measured:** 71 % of its term graph is needed only by proofs; nothing has been kernel-checked locally yet;
  - **S-ADR-0012 accepted.**
  - **B-15 measured with Moonkale** (its `edit_loop`, Moonkale `26752448` on Sophia `202e137a`): the whole edit loop's p99 is 2.4–3.5 ms on 10⁵ definitions (target < 50 ms); Moonkale's 500 ms change-feed polling is what a user would wait on ([[Benchmark Results]]).

  **After Moonkale's desktop check (2026-10-21),** four findings on Sophia's side, all fixed: Julia records and ingest manifests now print (the source text a definition was read from, through a new `source-span` fact; re-ingest needed for old stores; Moonkale re-ingested LinearAlgebra and all 945 names print, pinned at `4c2aaab3`); listing a namespace is a skip scan (root 32 → 0.1 ms on 10⁵ names); parse errors name tokens as written; `let` and `match` follow the Rust profile's braces. Found on the way: 7 % of `sophia gen` definitions print but do not parse back ([[SP-015 Stored definitions deeper than the parse limit cannot be edited]]).

  **Moonkale ADR-0029 (accepted by Daniel, 2026-10-23):** Sophia becomes Moonkale's code-graph store, one derived store per folder, files the truth. Sophia's answer is [[SPEC-009 Moonkale Folder Store]], **accepted by Daniel on 2026-10-24** with SP-015 (revised the same day: no depth limit, depth must cost memory not stack, SR-145) and a `v0.1.0` tag after the gates; **implemented on 2026-10-24**: `replace_files`, the derived mode, `text_source`; SP-015 step 1a (any depth on native targets); sophia-ingest on Moonkale's shared naming (`09ce6d9c`). SP-015 resolved: no pass recurses on the call stack (parser and elaborator included), any depth on native and wasm (Node, Firefox), printing and parsing linear; `stacker` removed. **`v0.1.0` tagged on 2026-10-24** at `c3c48e02` (after the fact-provenance fix found by Moonkale's Codex) (finished records from Moonkale, per-file replacement in one transaction, an enforced derived mode, SQLite for wasm, a first tag). The whole workspace except the ingest compiles for `wasm32-unknown-unknown`; nothing has run in a browser yet. **After `v0.1.0`, Moonkale's history replay (P-231)** found generic memberships drifting from a fresh build: repeated members were subtracted more than once, and names changing between generic function and definition left orphan memberships or lost live ones. Both are fixed and tested on all four backends, and each step is compared with a fresh build (`folder.md`). Moonkale reports 0 differences at every checkpoint after the fix. **`v0.1.1` tagged on 2026-10-24.** **Folder store garbage measured** ([[Folder Store Garbage]]): about 60 % of a replayed store is unreachable, mostly old source texts. **Collection implemented (SR-146):** the source-of-truth setting decides. In derived stores, where the files are the truth, `replace_files` collects what no current file reaches, and `collect_folder` sweeps old stores. On Moonkale's replay the sweep leaves exactly the fresh build's records: 39.2 MB against 91.2 MB. Where the store is the truth, history is kept. Moonkale's replays with collection: 0 differences, no measurable cost per batch, 41.5 MB against 92.6 MB. `Interface::all` on many namespaces: 2.1 s → 0.12 s (namespace children were listed and discarded; two SQLite prefix reads scanned their tables, Moonkale P-232). **`v0.1.2` tagged on 2026-10-24** (collection by source of truth, the `all` fixes). A Codex review of the changes since `v0.1.0` found three things, all checked and fixed: facts kept records that a fresh build would not have, which made a dropped fact leave garbage; and one file listing a generic function twice could retire another file's method. **`v0.1.3` tagged on 2026-10-25** with these fixes; Moonkale pinned it (`48c188c9`), both replays clean. **Research questions section added (2026-10-25, SR-147):** [[Research Questions]], RQ-001 to RQ-003. Writing them found two things: a recursive group's ids respect automorphisms but not bisimilarity (RQ-001, now a test), and collection keeps a record a file provided but nothing reaches, so the store can differ from a fresh build (RQ-002). Daniel chose to reject them: `replace_files` refuses such a file with `UnreachedRecord` (2026-10-26). Moonkale's replays on `dcf8660`: 0 errors, 0 differences, record counts unchanged (5 940 and 1 770), its 75 tests pass (Moonkale's report). **`v0.1.4` tagged on 2026-10-26.** **Where the truth lives (Moonkale ADR-0030, Daniel, 2026-10-26; SR-148):** legacy folders keep files as the truth, everything else has the store as the truth. Sophia's part is [[SPEC-010 Store-Truth Folders and Generated Files]], accepted by Daniel on 2026-10-26, **implemented on 2026-10-28** except `propose_text` and the SR-149 text export: file tokens on `replace_files`, an in-place switch of the truth, generated files and outside edits as proposals; Moonkale normalises foreign code, as in SPEC-009. **Acceptance 2 passes (Moonkale's report, 2026-10-28):** Moonkale's history replay on `08e6ffb`, switched to store truth halfway, checks the replay against a twin store built fresh and switched at the same commit. At every checkpoint after the switch it also compares the replay with a fresh build: 0 differences for both Lenticulum.jl (130 commits) and Moonkale (471), 0 errors, and no token conflicts in 65 and 236 commits written with `replace_files_checked`. Keeping history, the stores grow to 2 473 and 7 840 records. `give_truth` removes 703 and 1 900 of them, leaving exactly a fresh build's 1 770 and 5 940 records. Command and raw reports are on Moonkale's branch `replay-truth` (`08e23152`), `packages/services/index/measurements/history-2026-10-28-*-spec010.jsonl`. **`v0.1.5` tagged on 2026-10-28** (SPEC-010 acceptance 1 and 2; acceptance 3 waits for `propose_text`). **Codex's review of SPEC-010 (2026-10-28)** found six bugs, all checked and fixed, each with a test that fails before the fix except the export race: generic names and a group's other names escaped `LaidOut`; `give_truth` kept names no file has (now `NotInFiles`); export could pair a new token with old text; a recursive member's second name got a wrong span (now `aliases`); proposals did not list generic functions created or removed. `fetch_text` had the same token race and now reads the token first. **`v0.1.6` tagged on 2026-10-28** with these fixes. Moonkale's replay on `v0.1.6`: the same counts and 0 differences, 0 errors, no token conflicts; Moonkale master `6d1b1eb3` pins `v0.1.6` (Moonkale's report, 2026-10-29; reports `history-2026-10-28-*-v0.1.6.jsonl`). Sharing a store-truth folder: a text export for now (SR-149), its own spec to come. Also fixed for Moonkale: `Interface::all` on large stores (P-229: no term recursion, no quadratic edge check).

  **Next (SR-124 to SR-126):** the [[Semantic Boundary Experiments]] (small, each with an oracle), and B-15 (Moonkale edits) as soon as Moonkale is on the current revision. LuxCore is dropped as a step; full Lux.jl after the AD and Reactant experiments, Lenticulum after that. No storage-layout change without Daniel's agreement.
- **Specifications:** eight drafts ([[Specifications]]).
- **Decisions:** [[Decisions]]. Accepted: S-ADR-0004 (Rust only), S-ADR-0007 (store vs language), S-ADR-0008 (sophialang first), S-ADR-0009 (relations tracked), S-ADR-0010 (amended), S-ADR-0011 (own query algebra).
- **Requirements:** [[Requirements]]: SR-01 to SR-29 and SR-51 to SR-74 from Daniel; SR-30 to SR-50 from Moonkale; SR-75 to SR-89 from the first external review; SR-90 to SR-106 from Daniel's query-language proposal; SR-107 to SR-113 from the review of M3a; SR-114 to SR-118 (the three projects); SR-119 to SR-132 from Daniel's storage and language requests and the real-code review.
- **Daniel's answers of 2026-10-20 (second message):** push; delete the Julia skeleton (done); M2 starts with the interface crate. Relations: S-ADR-0009 proposed (tracked), awaiting confirmation.
- **Daniel's answers so far:**
  - Q1: keep both Unison and Sophia.
  - Q2: Julia-flavoured, display as settings stored in the database.
  - Q3: Rust only.
  - 2026-10-20: the language principles (SR-51 to SR-61), research questions (SR-62 to SR-74), and the go-ahead to implement.
- **Research notes answering the questions of 2026-10-20:**
  - [[Compression and Canonical Form]]
  - [[Type Taxonomy]]
  - [[Computational Trinitarianism]]
  - [[Operating Systems and Sophia]]
  - [[Hardware for Sophia]]
  - [[Block Size and Hash Overhead]]
  - [[Adopting from Unison]]
  - [[Minimal Binaries]]
  - [[LLMs and Mixed-Language Code]]
  - [[Database Comparison]]
- **Benchmarks:** [[Benchmark Plan]]; [[Benchmark Results]].
- **External review (2026-10-20):** triaged in [[Review 2026-10-20 (external)]]. It found one real bug (symmetric recursion), which is fixed; and led to SR-75 to SR-89, SPEC-006, corrections to SPEC-001 to SPEC-004 and the theory notes, edit tokens, world snapshots, and the **P0 gate** in [[Milestones]].
- **Coordination:** the Moonkale agent has the 2026-10-17 interface. It is to be told about the split into SPEC-001 to SPEC-005.
