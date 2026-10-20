#open-problem

> **Status: open (2026-10-20).** Related: [[SPEC-007 Sophia Query Language]] §4.3, SR-54, SR-94.

**Problem.** A query result is identified by H(query, snapshot, parameters). In v0 the snapshot is H(store instance, last log seq). So **any** logged change invalidates every cached result, even one the query cannot see:
- a profile switch;
- an annotation, when the query reads only `refs`.

That is the same "invalidate everything" pattern SR-54 forbids for tests.

**Fix, in steps.**
1. **Per-relation epochs.** The store keeps the last seq that changed each base relation (`def`/`refs` on add-definition; `name` on name changes; `member` and `nominal` on method changes; `annotation`; `fact`). A query's snapshot hashes only the epochs of the relations it reads. Cost: one meta write per logged change, measured in B-03.
2. **Content-addressed snapshots.** Hash the name table and the world snapshot (SPEC-006 §2), so that two stores in the same state give the same snapshot. Results become shareable between stores.
3. **Finer still:** read sets. Record which keys a run looked up, and invalidate on change to those keys only. This is incremental view maintenance (SPEC-007 §11).
