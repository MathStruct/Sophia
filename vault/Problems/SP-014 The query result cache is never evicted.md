#open-problem

> **Status: open (2026-10-20).** Found while writing [[Current Inventory]]. Related: [[SPEC-007 Sophia Query Language]] §4.3, [[Materialization Policy]], [[Garbage Collection]].

**Problem.** `run_cached` stores every complete query result in `t_meta` under `query-result/<result id>`, where the result id is H(query, snapshot, params). Nothing ever removes one. Every write makes a new snapshot, so on a store being edited (Moonkale), every repeated query adds a new entry and the old ones can never be hit again. The cache only grows.

This violates the [[Materialization Policy]]: query results are derived objects (category 5), to be **cached**, not persisted.

**Options.**
1. **A size budget with LRU eviction**, results keyed as now. Simple; needs an access time per entry (a write on every hit, or an in-memory LRU over the persisted entries).
2. **Evict non-current snapshots first**: an entry for a snapshot that is neither current nor cited by a fact cannot be hit by a query about the present.
3. **Do not persist results at all** by default: keep an in-memory cache per process, and persist only results a fact or an agent session cites.

**Proposal.** Option 3 as the default, with option 2 for persisted ones. It matches the policy: provenance (query id, snapshot, params) is enough to rebuild a result, and most results are used once ([[Storage Measurements]] M-3 will check that). Not implemented yet: SR-126, measure first.
