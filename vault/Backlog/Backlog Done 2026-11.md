#overview #planning

> The backlog tasks finished in November 2026, moved here from [[Backlog]] when closed.

> Sources: [[Backlog]], [[Status]].

| id | task | closed | commit | verified |
|---|---|---|---|---|
| B-001 | Fill the backlog: one row per open item of [[Milestones]] (M2 onwards), the "waiting" items of [[Status]], open SP-nnn problems and unanswered SR-nn requirements | 2026-11-17 | not committed yet (branch `dev-danielb`) | rows B-002 to B-059 written from Milestones (M3b to M7, the order of work, P0 demonstration 3), [[Semantic Boundary Experiments]] (E-0 to E-10), the open problems of the [[Problem Log]], the requirements with status open, confirm or planned, and the open items of [[Review 2026-11-10 Fable (external)]]; every wikilink in [[Backlog]] checked to resolve. Found on the way: SR-89 said demonstration 2's Moonkale side was open, while [[Milestones]] records it passing (Moonkale `46a06b65`); SR-89 corrected |
| B-002 | Annotations move with the truth: `give_truth(…, exported_annotations)` refuses with `AnnotationsNotExported`; `take_truth(…, annotations)` stores them in the same transaction; `annotate_many` skips an annotation already stored | 2026-11-17 | not committed yet (branch `dev-danielb`) | `annotations_move_with_the_truth` in `crates/sophia-store/tests/folder.rs` on SQLite, redb, LMDB and DuckDB covers the three rules, a re-sync logging nothing (before and after `give_truth`), a refused switch storing nothing, and a changed status being stored and logged; `cargo test --workspace --all-features`: 342 passed, 0 failed; clippy `-D warnings` and `fmt --check` clean. Settled while building: "already stored" means stored exactly as given, so a status update is still written. Reviewed by Mistral ([[Review 2026-11-17 Mistral B-002 (external)]]): no defect, one missing test added. Moonkale told through `HANDOFF-from-sophia.md` |
