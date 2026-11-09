#open-problem

> **Status: option 2 implemented (2026-11-09); option 3 still open.** Found while writing [[Current Inventory]] (2026-10-20). Related: [[SPEC-007 Sophia Query Language]] §4.3, [[Materialization Policy]], [[Garbage Collection]].

**Problem.** `run_cached` stores every complete query result in `t_meta` under `query-result/<result id>`, where the result id is H(query, snapshot, params). Nothing ever removes one. Every write makes a new snapshot, so on a store being edited (Moonkale), every repeated query adds a new entry and the old ones can never be hit again. The cache only grows.

This violates the [[Materialization Policy]]: query results are derived objects (category 5), to be **cached**, not persisted.

**Options.**
1. **A size budget with LRU eviction**, results keyed as now. Simple; needs an access time per entry (a write on every hit, or an in-memory LRU over the persisted entries).
2. **Evict non-current snapshots first**: an entry for a snapshot that is neither current nor cited by a fact cannot be hit by a query about the present.
3. **Do not persist results at all** by default: keep an in-memory cache per process, and persist only results a fact or an agent session cites.

**Proposal.** Option 3 as the default, with option 2 for persisted ones. It matches the policy: provenance (query id, snapshot, params) is enough to rebuild a result, and most results are used once ([[Storage Measurements]] M-3 will check that). Option 3 is not implemented: SR-126, measure first.

**Implemented (2026-11-09): option 2.** `run_cached` now keys each entry `query-result/<snapshot hex>/<result id hex>`; on a miss that writes a result it removes, in the same write transaction, every `query-result/` entry whose snapshot is not the current one — old-layout `query-result/<result id>` keys, which have no snapshot segment, are removed too. A hit on the current snapshot still returns `Cache::Hit`, and truncated results are still not stored. Daniel said to hand SP-014 to Mistral (2026-11-09). Mistral wrote option 2 through `vibe` and Claude (Sophia) checked it. Test: `results_are_cached_per_snapshot` (`crates/sophia-query/tests/execution.rs`) on all backends; it fails when the removal is undone. Two limits: a miss reads every cached entry (`meta_prefix` returns values too) to find the stale ones; and if a write lands between a miss's read and its store, that miss removes the newer snapshot's entries and stores a stale one, which the next miss removes. Option 3 (an in-memory cache by default, persisting only cited results) is still open, and the proposal above stands.
