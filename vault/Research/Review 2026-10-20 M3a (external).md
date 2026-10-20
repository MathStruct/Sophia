#reference #open-problem

> The third external review of 2026-10-20: ChatGPT's critical review of the M3a push (`d0c08b4`), passed on by Daniel, who added that **nobody uses Moonkale, Lenticulum or Sophia yet, so big alterations are fine**. It worked from static reading. This note records each point, whether it was right, and what was done. Requirements: SR-107 to SR-113 in [[Requirements]].
>
> Sources: the review text (conversation of 2026-10-20); the code and tests named below.

## Verdicts

| § | Point | Verdict | What was done |
|---|---|---|---|
| 3 | **Cache race (P0):** `run_cached` computed the snapshot in one read transaction and evaluated in another, so a concurrent write could store rows of a newer state under an older snapshot's id | **Correct.** A real violation of ResultId = H(Q, S, p) | Fixed: snapshot, lookup and evaluation in one read transaction; the write follows (SPEC-007 §4.3) |
| 4 | **Cache ignores limits:** a cached complete result was returned whole to a caller with a smaller row cap | **Correct** | Fixed: the caller's row cap is applied on the way out, with `truncated` set. The result id stays semantic; only complete results are cached. An execution id would be added only if a policy ever changed the answer. Test: `results_are_cached_per_snapshot` |
| 5 | **`max_derived` is not a memory bound** | **Correct.** I had called it "the memory bound" | Renamed in the docs to a logical limit. Added an **estimated byte budget** (lists, sets, indexes, memoised and scanned base tuples, select rows), a **wall-clock deadline** and a **cancel flag**. The docs say plainly that the estimate is not a guarantee; a hard bound needs a counting allocator or a separate process for untrusted agent queries (SPEC-007 §6.0). Tested |
| 6 | Expressiveness: add typed, pure, cost-annotated **query functions** rather than many built-ins | **Agreed** | SPEC-007 §11 (design only) |
| 6 | Keep queries a separate DSL now; let sophialang build and run them later | **Agreed** (it is S-ADR-0012) | — |
| 7 | **Canonicalisation:** "repeat at most 8 rounds" does not guarantee canon(πQ) = canon(Q) | **Correct** | Replaced by an **exact definition**: the minimum encoding over all valid traversals, trying every tie, refused beyond a budget, as for recursive groups. Property test on 2000 random queries (20 000 in a stress run): permutations of predicates and rules, duplicates, unreachable predicates, renaming, decode(encode), idempotence (SPEC-007 §4.1, SP-009) |
| 8 | **Optimiser correctness:** test magic sets differentially on random programs and databases | **Correct**, and it paid off at once | `magic_sets_preserve_meaning_on_random_programs`: random safe programs with repeated variables, mutual recursion, stratified negation, several bound arguments and `count`, over a random store. Plain, magic and SQL evaluation agree (719 valid programs per default run; 9561 in a stress run). It found **one real bug**: SQLite reports a read of a CTE that a rule uses without reading a column of it, and the compiled-SQL guard refused it. Fixed in `Sqlite::query_compiled` |
| 9 | Next: real-code benchmarks and Moonkale integration **before** the agent server | **Agreed** | [[Milestones]]: the order is now B-16 (real code) and B-15 (Moonkale edits), then the set-oriented executor, then M3b |
| 10 | A **common interface for transformations and their evidence** (T: (Q, E) → (Q′, E′)), defined now and built later | **Agreed** | SPEC-006 §6a: one transformation record (by, input, output, relation, obligation, evidence, inputs); magic sets will be its first producer |

## What the review got wrong or missed

- **"A truncated first run poisons the cache"** was framed as an issue but was already handled: truncated and failed runs were never cached. The real problem was the row cap on hits (row 2).
- **The CTE-read bug** was not in the review. Its recommended test found it, which is the best argument for the recommendation.
- **The byte budget cannot be exact inside one process.** The review's `max_memory_bytes` reads as if it could be. SPEC-007 says what the estimate covers, and where a hard bound must come from.
