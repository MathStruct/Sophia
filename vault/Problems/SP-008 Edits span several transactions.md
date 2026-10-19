#open-problem

> **Status: open (2026-10-20).** Related: `crates/sophia-interface/src/edit.md`, [[SPEC-004 Moonkale Interface]] §3.

**Problem.** One text edit through the interface runs several store transactions:
1. create the nominal entities of new relations;
2. add records and new names;
3. update existing names;
4. attach relation members.

If step 3 fails, for example because another writer moved a name in between, steps 1 and 2 have already happened.

**Why it is not dangerous.** Every intermediate state is consistent: records are immutable, unnamed records are just unused, and new names point at complete definitions. Nothing is ever half-written.

**Why it should still be fixed.** A failed edit should leave no trace (SPEC-001 §13, "failed writes roll back", holds per store call but not per edit).

**Fix.** A `Store::batch(|ops| …)` that runs the whole edit in one backend transaction. The logic in `logic.rs` is already written against `&mut dyn WriteTx`, so it needs exposing at that level.
