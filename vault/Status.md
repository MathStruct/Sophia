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

  **Next:** M3b (an MCP server for agents over the query API), B-15/B-16 (edit latency, real code), or the query follow-ups (prefix access path, precise snapshots, SP-010).
- **Specifications:** seven drafts ([[Specifications]]).
- **Decisions:** [[Decisions]]. Accepted: S-ADR-0004 (Rust only), S-ADR-0007 (store vs language), S-ADR-0008 (sophialang first), S-ADR-0009 (relations tracked), S-ADR-0010 (amended), S-ADR-0011 (own query algebra).
- **Requirements:** [[Requirements]]: SR-01 to SR-29 and SR-51 to SR-74 from Daniel; SR-30 to SR-50 from Moonkale; SR-75 to SR-89 from the first external review; SR-90 to SR-106 from Daniel's query-language proposal.
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
