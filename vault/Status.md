#overview

> What exists, what is being decided, and what is waiting. Check the date. Where the repository disagrees with this page, the repository wins, so fix this page.

**As of 2026-10-20.**

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
