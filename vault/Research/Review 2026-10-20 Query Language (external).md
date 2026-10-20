#reference #design

> A second external review of 2026-10-20, written by ChatGPT in reply to Daniel's proposal for a native query language, and passed on by Daniel. Daniel had proposed that Sophia should have its own query language rather than SQL, and asked whether SQL or LMDB is the better foundation. The review agreed, and ended with a "suggested message to Claude", which Daniel forwarded as his direction. This note records each point, **whether it was right**, and what was done. The requirements it led to are SR-90 to SR-106 in [[Requirements]]. The decisions are [[S-ADR-0010 Primary backend and an analytics replica]] (amended), [[S-ADR-0011 Sophia has its own query algebra]] and [[S-ADR-0012 The query IR is a separate IR on shared primitives]]. The specification is [[SPEC-007 Sophia Query Language]].
>
> Sources: the review text (in the conversation of 2026-10-20); the code and benchmarks named below.

## Verdicts

| § | Point | Verdict | What was done |
|---|---|---|---|
| 1 | "SQLite versus LMDB" is not the architectural question; Sophia needs its own query IR, and backends expose access paths and capabilities | **Right**, and the most useful point | The IR is defined over base relations and access paths (`ReadTx` methods); it runs on all four backends (SPEC-007 §2, §6) |
| 1 | LMDB has no query language, so choosing it now would remove Moonkale's queries | **Right, then made moot.** With Sophia's own language on every backend, LMDB loses nothing | S-ADR-0010 amended: SQLite default, LMDB experimental, choice after B-15, B-16, B-17 |
| 2 | Study TypeQL (typed relations, roles) and CQL (Σ/Δ/Π migrations); adopt neither wholesale | **Right.** It missed the closer prior art: CodeQL, Glean (Datalog over a key-value store, for code), Datomic (Datalog over index access paths), Soufflé, egglog | [[Query Languages for Sophia]] |
| 2 | CQL schema migration is not semantic equivalence between languages | **Right** | Kept as two separate relations (SPEC-007 §10; SR-84) |
| 3 | Queries are Sophia objects: canonical, content-addressed, with proofs, plans, measurements attached | **Right** | Queries are records of the language `sophia-query`; stored, named, queryable, verifiable (SPEC-007 §4, §8) |
| 3 | A query's id and its result's id differ: ResultId = H(Q, snapshot, parameters) | **Right**, and necessary for caching | `canon::result_id`, `run_cached`. The v0 snapshot is coarse ([[SP-010 Query snapshots are coarse]]) |
| 4 | "Storing SophiaLang without serialisation" is conceptually yes, physically no: bytes are still needed, but no JSON or SQL layer | **Right, and already true.** Canonical bytes go straight into LMDB and redb values. The framing can be walked in place | Measured in [[Zero-Copy Storage of Canonical Graphs]] |
| 5 | Do not make the query language identical to SophiaLang: `x + y == 10` is a test in one and a constraint in the other; queries are a typed DSL `Query{S, A, E}` | **Right** | SPEC-007 §3.4; S-ADR-0012 (a separate IR now, a quoted core node later) |
| 6 | Order of work: query IR, SQLite execution, LMDB experiment, planner, Moonkale integration, benchmarks, frontend, research | **Mostly right; partly outdated.** LMDB was already implemented (M2). The order was followed: IR, native executor on all backends, SQL translation, planner and magic sets, Moonkale `query` dialect, B-17, Julia syntax | SPEC-007 §12 |
| 6 | Include Datalog early | **Right** | The IR *is* stratified Datalog |
| 6 | Measure LMDB's single writer before using it for a multi-agent server | **Right** | SR-104 |
| 7 | Do not build a full new database engine | **Right** | The executor is ~600 lines over existing backends; no storage engine was written |
| end | Program optimisation and query optimisation converge | **Right as a direction** | SR-106, after the small algebra works on two backends (it now works on four) |

## What the review missed

- **Goal-directed evaluation is the first-order issue.** The review listed a planner as step 4 of 8. B-17 shows that without magic sets, "transitive dependants of X" computes the closure of the whole store: about 6 s at 10⁴ definitions, against about 0.15 ms with them. That is four orders of magnitude, more than any backend difference. The magic-set rewrite is also the first optimisation written as IR → IR, so it shows the architecture the review proposed.
- **SQL cannot be the semantic definition even on SQLite.** SQLite's recursive CTEs cannot express mutual recursion. The translation refuses such queries, and the native executor runs them (SPEC-007 §7).
- **Canonical form of queries is harder than of terms.** Conjunction and union commute, so full canonisation is graph canonisation. v0 normalises names, predicate and rule order, but not literal order ([[SP-009 Canonical form of queries]]).
- **Result caching needs snapshots that do not move when the cache is written.** Cached rows live in unlogged meta rows for this reason.
