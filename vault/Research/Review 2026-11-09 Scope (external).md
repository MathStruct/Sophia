#reference

> Three external reviews of the repository, by ChatGPT, Gemini and Mistral. Daniel forwarded them on 2026-11-09 together with his question: is sophialang as **one language with three parts** (programming, queries, a type-theory language for arbitrary type systems) still in scope, or has the project settled on a smaller goal? His statement is SR-154. This note records each checkable claim, whether it is right on the current repository (`cc9e426`), and what was done. Triaged by Claude (Sophia) on 2026-11-09.
>
> Sources: the three reviews, uploaded by Daniel (not in the repository). code: the notes named below.

## Where all three agree, and are right

- **The ambition was sequenced, not reduced.** Milestones M0–M7 still end with self-hosting, a dependent checker and other languages ([[Milestones]]). Everything built so far is the store, the query language and Moonkale's interface. The language that executes (M4) is design only.
- **The query language is planned to become part of sophialang.** [[S-ADR-0012 The query IR is a separate IR on shared primitives]]: a separate IR now, and with M4 a typed `Query{S, A}` value that sophialang code builds and runs. The query language is to be rewritten in sophialang at M6. It keeps its own evaluator because `x == y` is a test in one and a constraint in the other.
- **Moving from SQL to Sophia's query language over a key-value store is done.** [[S-ADR-0011 Sophia has its own query algebra]] (Daniel's proposal, 2026-10-20): stratified Datalog, executed natively on all four backends; SQL is only a translation target.
- **Storing information does not slow execution, by design:** SR-52/53, [[Computational Trinitarianism]] §2.2 (proofs live in an erasable universe).

## Verdicts

| source | claim | verdict | evidence / done |
|---|---|---|---|
| ChatGPT §2C | a language for **describing arbitrary type systems** (judgments, rules, checkers as objects) is not specified; SPEC-002 has exchangeable checkers, which is a different thing | **right** | No note in the vault specifies one (searched for logical framework, judgment rules, user-defined checker). Lifted as [[RQ-004 A Language for Defining Type Systems]]. Gemini's table presents checkers as already fulfilling this part, and Mistral's point 4 does too. That overstates it |
| ChatGPT §6 | record the three-part vision as a requirement, so the parts do not stay separate products by inertia | **right** | SR-154 |
| ChatGPT §3, Mistral 5 | trinitarianism is a family of precise correspondences, not "everything is one category"; Datalog is the regular-logic row | **right** | [[Computational Trinitarianism]] §0, §2.5 |
| ChatGPT §5 | `Status.md` says "As of 2026-10-28" but has entries to 2026-11-08 | **right** | Date updated |
| ChatGPT §5 | open: SP-014 (cache never cleared), SP-010 (coarse snapshots), heuristic planner, literal order not canonical | **right, known** | All are in the Problem Log or SPEC-007 |
| Gemini §3 | "LMDB is the fastest on every single query, SQLite last" | **wrong** | [[Database Comparison]]: LMDB is fastest of the *native* executors, but SQLite's SQL engine wins set-at-a-time queries by 2–9× (unreferenced definitions at 10⁵: SQL 32 ms against LMDB 99 ms). ChatGPT states this correctly |
| Gemini §1 | 64 566 theorems agree with metamath.exe | not rechecked here | the number is from the vault's own Metamath notes |
| Gemini (parallelism) | readers run in parallel with one writer; `run_cancellable` exists; SR-104 says LMDB's single writer is measured before a multi-agent server | **right** | `sophia_query.rs:231`; Requirements SR-104 |
| Gemini §4 | MLIR lowering is M3–M6 | **wrong** | M3 is the query language and the MCP server; compiling is M5 ([[Milestones]]) |
| Mistral 2 | [[Multi-AST Layering]]: terms, types and proofs are one node kind with three roles | **right** | that note |
| Mistral | RocksDB is not a backend; the `Backend` trait and conformance suite make adding one cheap | **right** about the trait; "cheap" is an expectation | `kv.rs` serves redb and LMDB |
| Mistral | "95.1 % agreement with Julia's method table" | not rechecked here | |

## Done

- SR-154 records Daniel's three-part statement, as open and in scope.
- [[RQ-004 A Language for Defining Type Systems]]: the part nothing specifies yet, stated so it can be worked on alone (logical frameworks: LF, Twelf, λΠ-modulo/Dedukti).
- `Status.md`'s date.
