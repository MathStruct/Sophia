#implementation

> A folder store ([[SPEC-009 Moonkale Folder Store]]): Sophia as the derived code-graph cache of a folder whose files are the truth. Moonkale's index hands over finished records per file; `Store::replace_files` replaces everything a set of files contributed, in one transaction. Source: `folder.rs`.
>
> Sources: SPEC-009 §1–§3 and §7; Moonkale ADR-0029. code: `folder.rs`, `tests/folder.rs`.

**What a file contributes** (`FileIngest`): its path and text (`None` = deleted), the normaliser, the records (ids from Moonkale's WP-66), a byte span per record, names of definitions, and methods of generic functions (`FileGeneric`), plus facts about its records.

**What is stored per file:** a **file manifest** record (`lang = sophia-ingest`, kind `file-ingest`, normaliser `sophia-file-manifest@0`) with path, normaliser, source id, names, generic memberships and spans, and the source text as a source record (the same as `sophia-ingest`'s). Meta rows:
- `file/<path>` → the file's current manifest;
- `owner/<name>` → the file that owns a definition's name;
- `member/<generic>/<member>` → how many files contribute that method;
- `defined-in/<record>/<path>` → the files whose manifest places a record (for `text_source`);
- `folder/normaliser`; `store/mode = derived` (`set_derived`).

**`replace_files(&[FileIngest])`**, one write transaction:
1. validates names, indices and spans; refuses a name claimed by two files of the batch;
2. refuses, with `StoreError::NameClaimed { name, owner }`, a name owned by a file outside the batch (`owner` empty when no file owns it, such as a generic function other files still contribute to), so Moonkale can retry with that file added; nothing is written;
3. refuses a different normaliser than the folder's (`NormaliserChanged`: rebuild);
4. stores records and sources; removes names whose file no longer has them, moves names whose target changed (logged as `MoveName`, lineage extended, as an edit), adds new ones;
5. counts generic-function memberships per contributing file: a method stays while some file has it; a generic function's name is created with its first method and removed with its last;
6. replaces the file manifests, `defined-in` rows and facts.

An unchanged batch logs nothing. A definition moves between files only when both files are in the call.

**Spans come from the manifest, not from facts:** facts are immutable, so a span fact about an unchanged record would keep pointing at an old version of a moved file. `text_source(id, name)` reads the current manifests; with one id in several files, the span whose text contains `name` is chosen.

**Tests** (`tests/folder.rs`, all four backends): two files with a shared generic function; an edit (one name removed, one moved, one added, a method replaced); a move between files; a claim of another file's name (refused, rolled back); two files of one batch claiming one name; deletions; one method in two files; a normaliser change.

**Not yet:** Rust and Python names in Moonkale's crate-and-module form (the producer computes names, so this is Moonkale's side).

**Facts follow their file (2026-10-24, found by Moonkale's Codex before WP-21).** At `cafe940e`, `replace_files` stored a file's facts but never retracted them: a replacement with fewer facts, or deleting the file, left them in `facts_of`. Now:
- the manifest (`sophia-file-manifest@1`; `@0` manifests still decode, with no facts) lists the file's fact ids;
- the meta row `fact/<id>` counts the files contributing a fact; a fact is stored with its first contributor and removed (`WriteTx::remove_fact`, all four backends; logged as a `fact` entry with only an old id) with its last;
- a fact already stored before any file contributed it is marked `fact-external/<id>` and never removed by the folder writer;
- all of it happens in the batch's transaction, so a failed batch leaves facts and manifests as they were.
Tested on all four backends (`facts_follow_their_files`: replacement, a resolution change with an unchanged body, two contributors, deletion while the record lives on in another file, an external fact, rollback) and with Codex's reproducer (`next17-sophia-probe`, 2 of 2, failing on `cafe940e`).

**Migration:** folder stores written before this have no fact provenance; their facts stay until the store is rebuilt. No folder store is in use yet.

**Memberships under history replay (2026-10-24, Moonkale's P-231).** Moonkale replayed its history (467 commits) and Lenticulum.jl's (130) into a folder store and compared it every 50 commits against a store built fresh from the tree. Manifests, records, text sources and facts matched; generic names and memberships did not (10 and 35 differing keys). There were two causes at `v0.1.0`:
- **Repeated members were subtracted more than once.** A refresh added a file's new methods once per distinct id but subtracted its old ones once per definition, so two methods with one normalised id lost a live member. Now both sides count each member once per file and generic (`FileManifest::contributions`, `FileIngest::contributions`).
- **Names changing kind kept or lost memberships.** Methods are not counted while a generic function's name is a definition's. When the name became a definition's, the old generic function's relations and `member/` counts stayed behind as orphans. Now they are closed and removed in the same batch. When the definition went, the methods files still contribute did not come back, from files in the batch (their old contributions were subtracted from counts that were zero) or from files outside it (not looked at). Now a freed name's memberships are rebuilt from every file's manifest.

Tests: `repeated_methods_survive_a_refresh` and `names_changing_kind_keep_memberships_right` on all four backends. Each step is compared with a store built fresh from the files so far. Both tests fail at `v0.1.0`. With the freed-name rebuild turned off, the second still fails. Folder stores written by `v0.1.0` may hold wrong counts; rebuild them.

**Collection follows the source of truth (2026-10-24, SR-146).** The store's mode is the setting:
- **Derived (`store/mode = derived`): the files are the truth.** Git keeps the history, so `replace_files` ends by collecting what the batch's old manifests reached and nothing reaches any more: the old manifest, its source text, and old versions of definitions. These go, and what only they referenced goes after them. A record is kept while any of these hold:
  - it is a current file manifest;
  - a name targets it;
  - it is a current generic member;
  - a stored record outside its component references it (`dependants`).

  A component goes as a whole. Each removal is logged as `remove-definition`, with the id as `old`. `WriteTx::remove_record` deletes the record and its index rows on all four backends. Lineage entries stay, as ids without bodies. `Store::collect_folder()` sweeps a derived store once, for stores written before this.
- **Not derived: the store is the truth.** Nothing is removed, and every version stays in the graph as its history. `collect_folder` refuses with `StoreError::NotDerived`.

Tests, all four backends:
- `files_truth_collects_what_files_no_longer_reach`: a six-commit history with edits, a move, a replaced method, a two-member component and deletions. After every commit, the derived store's record set equals a fresh build's.
- `store_truth_keeps_history_until_swept`: every version stays; the sweep is refused, then after `set_derived` leaves a fresh build's records.

**A known gap (2026-10-25):** a record in `FileIngest::records` with no name, span or generic membership, and referenced by nothing, is stored but not referenced by the manifest. When its file drops it, it is never a seed and stays, so the store differs from a fresh build (found while writing [[RQ-002 When Incremental Collection Equals a Fresh Build]], by a scratch test). Moonkale's index is not known to send such records. **Fixed (2026-10-26, Daniel: reject):** `replace_files` refuses a file with a record that no name, span or generic membership reaches, directly or through the file's own references, with `StoreError::UnreachedRecord`, and writes nothing (`unreached_records_are_refused`, all four backends). The alternative, a manifest that references every record, needs a new manifest version and is kept for later if a producer needs unreached records.

On copies of Moonkale's replayed stores, `folder_garbage STORE --collect` removed 5 601 records in 1.81 s (Moonkale, 471 commits) and 1 005 in 0.39 s (Lenticulum). In both, the record set afterwards equals the fresh build's, checked in SQL. After `VACUUM` the files are 39.2 and 10.3 MB, against 36.2 and 9.8 MB fresh; the rest is the log ([[Folder Store Garbage]]). SQLite reuses the freed pages; the file shrinks only on `VACUUM`.

**Codex's review of the changes since `v0.1.0` (2026-10-24).** Daniel let Sophia's agent give tasks to a Codex agent. Its read-only review reported three findings, traced in the code but not executed; each was checked here:
1. **"Collection ignores facts' content inputs"**: a record a fact uses could be collected. Checked: true. The answer, though, is the opposite fix: facts no longer keep records at all. In a derived store the files are the truth, and a fresh build of them stores neither a fact's input nor its subject when no file provides them. Protecting them would make the store differ from a fresh build. A fact whose record has gone dangles, as it does in a fresh build; facts still follow their files. Moonkale's facts (`uses_generic`) each have their own subject as their only input (checked in SQL on both replays), so they never meet this case.
2. **"Removing a fact leaves protected garbage"**: true, and caused by the subject protection that (1) removes.
3. **"Retirement subtracts duplicate contributions across entries"**: true. The check before a generic function's name becomes a definition's counted per manifest entry, so a file listing one generic twice outvoted another file's method and retired it. It now uses `contributions()` like the counting.

Tests, all four backends: `facts_do_not_keep_records` (a fact's input and a fact's subject, compared with a fresh build after every commit) and `a_file_listing_a_generic_twice_is_one_contributor`. Both fail before the fix.

Codex found nothing in `remove_record`, the SQLite prefix reads or `Interface::all`.

**Store-truth folders (2026-10-28, SPEC-010, Moonkale ADR-0030).** A folder whose truth is the store runs in normal mode: nothing is collected, and every version stays.
- **`file_token(path)`**: a hash over the file's current manifest id, and over each name and generic function the manifest lists: its `seq`, its target, and a generic function's version. It is 0 for a file the store does not have, so a new file is written with token 0. A file's token changes when the file is replaced, when one of its names moves, or when a method is added to one of its generic functions.
- **`replace_files_checked(&[(FileIngest, token)])`**: `replace_files` with a token per file, checked inside the write transaction. One stale token writes nothing, and `StoreError::FileConflict` lists every stale file. A derived store refuses the call (`Derived`).
- **`laid_out(name)`**: the file a name sits in (its `owner/` row) and that file's token. A generic function's name has no owner row: it is laid out by the first file, by path, that contributes a method to it. The interface refuses edits of such names (`LaidOut`).
- **`propose_files`**: says what an accept would do, per file: names added, removed and retargeted, memberships, moved spans, the text changing, conflicts, and the report or the error. It runs `replace_in` in a write transaction that is always rolled back, so it is exactly what `replace_files_checked` would do at that moment. It writes nothing, which the test checks against the records, the state and the log position. It takes the write lock for that time, and SPEC-010 §5's "read transaction" is amended to say so.
- **`take_truth(files)`**: replaces the files, then clears `store/mode`, logs a `Setting` entry `store/truth=store` and sets `store/truth-since` to its sequence number, all in one transaction. A store that is not derived refuses (`NotDerived`).
- **`give_truth(exported)`**: every current file must be listed with the source id of the text Moonkale wrote, and must match its manifest. Otherwise `ExportMismatch` is raised and nothing changes. Every name must be in a file (`laid_out`), or `NotInFiles` is raised: a fresh build of the files would not have it. Then derived mode is set, a `store/truth=files` setting is logged, and everything no file reaches is collected. This discards the store's history.
- **Refactor:** `replace_files` is now `validate_batch` (before any transaction) plus `replace_in` (inside one), so that the checked, proposing and switching calls share it unchanged.

Tests, all four backends: `checked_replacement_refuses_stale_tokens`, `a_proposal_writes_nothing_and_accepting_it_equals_replacing`, and `the_truth_moves_into_the_store_and_back`. The last runs `history()`: derived for the first commit, the truth into the store at the second, two checked commits that keep every version, then the truth back. The result equals a fresh derived build.

**Codex's review of SPEC-010 (2026-10-28, `fb6f531..686f8e0`).** Read-only. It reported six findings, traced in the code but not executed. Each was checked here with a test that fails before the fix:
1. **Generic names escaped `LaidOut`**: true. `laid_out` read only `owner/` rows, which generic functions do not have, so the interface could rename or delete `jl.Pkg.g` while files still contributed methods to it. Now `laid_out` also finds a generic function's contributing file (`generic_names_are_laid_out`; in the interface, `generic_and_group_names_route_through_files`).
2. **An edit could change a group's other laid-out name**: true, see [[edit]].
3. **`give_truth` kept names no file has**: true. A name written through the interface while the store was the truth survived `give_truth` as a root, though a fresh build of the files would not have it. Now `give_truth` refuses with `NotInFiles` (`give_truth_refuses_names_no_file_has`).
4. **Export could pair a new token with old text**: true by reading the code (no test: it needs a write between two reads). See [[sophia_interface]].
5. **Export of a recursive member with two names**: true, see [[sophia_interface]].
6. **A proposal did not list generic functions created or removed**: true. These follow from all files' contributions, not one file's diff. `Proposal` now has `generics_added` and `generics_removed`, read before and after the rolled-back replacement (`proposals_name_generic_functions_created_and_removed`).

Codex found no backend-specific difference in the rolled-back proposal.

**Codex's review of the fixes (2026-10-29, `e84baa3`).** Read-only, medium effort. Five findings, traced but not executed; checked here:
1. **A rule could add a method to a laid-out generic function**: true, see [[edit]].
2. **Export's retry missed changes outside the exported names**: true, see [[sophia_interface]].
3. **A proposal could miss a generic function brought back through another file**: not reachable. The case needs `g` to be a constant of `a.jl` while `b.jl` has a method of it. One batch cannot hold both (`validate_batch`), and a later batch with only `a.jl` is claimed by `b.jl`'s method. The test checks both refusals.
4. **`give_truth` checked owner rows, not targets**: true. A file's name retargeted by a store call (`Store::update`) kept its owner row, so `give_truth` accepted it, and the store differed from a fresh build. `give_truth` now reads every manifest once and requires each content name to have its file's target (`ExportMismatch` for that file otherwise) and each generic function name to be contributed by some file (`NotInFiles` otherwise). The test fails before the fix.
5. **`give_truth` scanned every manifest once per generic name**: true; unmeasured. The one manifest pass of (4) replaces it. `laid_out` still scans the manifests for a generic function's name, once per interface edit of such a name.
