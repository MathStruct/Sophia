#measurement #plan

> The measurements that decide [[Storage and Materialization]]'s open questions. Each has a hypothesis, a method and a failure criterion. They extend [[Benchmark Plan]] (B-06 compact encoding, B-09 snapshots, B-16 real code, B-17 queries). 2026-10-20.
>
> Sources: [[Current Inventory]] (the baseline numbers); [[Materialization Policy]].

| # | Question | Hypothesis | Method | Fails if | Status |
|---|---|---|---|---|---|
| M-1 | How much smaller is a store that keeps only authority, ids, kinds and one copy of the edges? | set.mm under 70 MB (from 567 MiB); LinearAlgebra under 2 MB (from 17.7 MB) | a second SQLite layout behind the same `Backend` trait; ingest both corpora; compare `dbstat` | the saving is under 3× | baseline measured ([[Current Inventory]]) |
| M-2 | What does rebuilding canonical bytes cost, per definition and per file? | below 1 ms per definition for Julia (parse the file once, normalise one definition) | time `module::ingest` per file and per definition; time the WP-66 encoding alone | rebuild for one Moonkale "show canonical form" request exceeds 50 ms (an interactive budget) | open |
| M-3 | Which derived objects are requested how often, at what cost? | a few query shapes dominate; most results are used once | log every `run_cached` call: query id, snapshot, rows, steps, bytes, time, hit/miss | — (it is a measurement) | open; needs a Moonkale workload (B-15) |
| M-4 | For evidence, how does finding cost compare to checking cost? | Metamath: checking is seconds, finding is human work | time `verify proof *`; later the same for an SMT certificate | — | **Metamath measured:** metamath.exe checks all 48 083 set.mm proofs in 2.85 s; open for SMT |
| M-5 | How fast does the log grow under editing, and what does compaction save? | ingest dominates the log today (5 % of set.mm's store); edits are small | B-15 edit loop; count log bytes per edit | log growth per edit above 1 KB | open |
| M-6 | When is a compiled derivative worth keeping? | small functions: compile ≫ run but both are milliseconds, so cache only; compiled models (Reactant/XLA): compile is seconds to minutes, persist | E-AD-0 and its successors ([[AD as a Materialization Case]]) | compile time does not dominate run time for any backend | **E-AD-0 done, hypothesis partly refuted:** even for toy functions producing D(f) takes 0.1–6.6 s (not milliseconds), 10²–10⁶ runs' worth; so persisting across sessions pays for small functions too. Reactant not yet measured |

A measurement that refutes its hypothesis is a result, not a failure: it goes into [[Benchmark Results]] and the policy changes.
