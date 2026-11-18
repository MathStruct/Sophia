#open-problem

> **Status: open; step 1 done (2026-11-18, B-016).** Related: [[SPEC-007 Sophia Query Language]] §4.3, SR-54, SR-94.

**Problem.** A query result is identified by H(query, snapshot, parameters). In v0 the snapshot is H(store instance, last log seq). So **any** logged change invalidates every cached result, even one the query cannot see:
- a profile switch;
- an annotation, when the query reads only `refs`.

That is the same "invalidate everything" pattern SR-54 forbids for tests.

**Fix, in steps.**
1. **Per-relation epochs. Done (2026-11-18, B-016).** The store keeps, per base relation, a seq at or after its last logged change (`relation-epoch/<relation>` in meta, written in the change's transaction; `LogOp::relations` maps ops to relations: `def`, `refs`, `uses` on adding or collecting a definition; `name` on name changes; `nominal` on a new nominal entity; `member` on method changes; `annotation`; `fact`; a setting changes none). A query's snapshot hashes only the epochs of the relations it reads (`Prepared::reads`), and the result cache evicts per read set. Tested on all four backends in `crates/sophia-query/tests/execution.rs` (`changes_elsewhere_keep_cached_results`, which fails when every op bumps every epoch, checked by that mutation; `stores_without_epochs_are_coarse_not_stale`). Cost: one meta read and up to three meta writes per logged entry, **not measured** (benchmarks deferred, 2026-11-18).
2. **Content-addressed snapshots.** Hash the name table and the world snapshot (SPEC-006 §2), so that two stores in the same state give the same snapshot. Results become shareable between stores.
3. **Finer still:** read sets. Record which keys a run looked up, and invalidate on change to those keys only. This is incremental view maintenance (SPEC-007 §11).
