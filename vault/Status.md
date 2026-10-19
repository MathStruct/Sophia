#overview

> What exists, what is being decided, and what is waiting. Check the date. Where the repository disagrees with this page, the repository wins, so fix this page.

**As of 2026-10-20.**

- **Direction:** Sophia is the **store**; sophialang is its **language** (S-ADR-0007). The first priority is the **Moonkale interface** ([[SPEC-004 Moonkale Interface]]). The plan is [[Milestones]] M0 to M7.
- **Code:** **M0 and M1 done** (2026-10-20):
  - `sophia-hash`, `sophialang-core` and `sophia-store` (SQLite and redb), plus `sophia-bench` and the `sophia` CLI;
  - 58 tests pass;
  - first benchmarks in [[Benchmark Results]].
  
  **Next: M2, the Moonkale interface**: rendering profiles in the database, the `julia` printer and parser, `sophia-interface`, then DuckDB and LMDB.
- **Specifications:** five drafts ([[Specifications]]).
- **Decisions:** [[Decisions]]. Accepted: S-ADR-0004 (Rust only), S-ADR-0007 (store vs language), S-ADR-0008 (sophialang first).
- **Requirements:** [[Requirements]]: SR-01 to SR-29 and SR-51 to SR-74 from Daniel; SR-30 to SR-50 from Moonkale.
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
- **Benchmarks:** [[Benchmark Plan]]; [[Benchmark Results]] (no runs yet).
- **Coordination:** the Moonkale agent has the 2026-10-17 interface. It is to be told about the split into SPEC-001 to SPEC-005.
