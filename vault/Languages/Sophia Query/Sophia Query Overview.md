#overview

> Sophia's query language, as a language: it is parsed, normalised, resolved, lowered and executed, which makes it the only language in the store at L5 today. 2026-10-20.
>
> Sources: [[SPEC-007 Sophia Query Language]]; [[S-ADR-0011 Sophia has its own query algebra]]; [[S-ADR-0012 The query IR is a separate IR on shared primitives]] (proposed); `crates/sophia-query`.

| Level | Status | Evidence |
|---|---|---|
| L1 Parsed | yes: Julia-flavoured syntax; printed in `julia` and `datalog` profiles | parser tests |
| L2 Normalised | yes: exact canonical form (minimum encoding over all valid traversals, budget 4 096); query ids by the store's component rule | property test: 2 000 random queries (20 000 in stress runs) under permutation, renaming, duplication |
| L3 Resolved | yes: checked (safety, arity, sorts), stratified | checker tests |
| L4 Lowered | yes: IR; magic-set rewrite (IR → IR); SQL translation (`WITH RECURSIVE`) | — |
| L5 Executable | yes: semi-naive native executor on four backends; SQL on SQLite | execution tests; B-17 |
| L6 Verified | no; magic sets and SQL **differentially tested** against plain evaluation on random programs and stores | `properties.rs` (719 programs per run, 9 561 in a stress run) |

**Materialisation** ([[Storage and Materialization]]): queries are authored objects (persisted, with ids); results are derived objects keyed by H(query, snapshot, params) and currently cached forever ([[SP-014 The query result cache is never evicted]]); working relations are transient and budgeted (bytes, steps, deadline, cancel).

**Interoperability:** queries read every language's records through eight base relations (`def`, `refs`, `name`, `member`, `nominal`, `uses`, `annotation`, `fact`), so one query spans Julia, Metamath and sophialang records alike.
