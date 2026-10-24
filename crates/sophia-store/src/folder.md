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
  - a fact is about it;
  - a stored record outside its component references it (`dependants`).

  A component goes as a whole. Each removal is logged as `remove-definition`, with the id as `old`. `WriteTx::remove_record` deletes the record and its index rows on all four backends. Lineage entries stay, as ids without bodies. `Store::collect_folder()` sweeps a derived store once, for stores written before this.
- **Not derived: the store is the truth.** Nothing is removed, and every version stays in the graph as its history. `collect_folder` refuses with `StoreError::NotDerived`.

Tests, all four backends:
- `files_truth_collects_what_files_no_longer_reach`: a six-commit history with edits, a move, a replaced method, a two-member component and deletions. After every commit, the derived store's record set equals a fresh build's.
- `store_truth_keeps_history_until_swept`: every version stays; the sweep is refused, then after `set_derived` leaves a fresh build's records.

On copies of Moonkale's replayed stores, `folder_garbage STORE --collect` removed 5 601 records in 1.81 s (Moonkale, 471 commits) and 1 005 in 0.39 s (Lenticulum). In both, the record set afterwards equals the fresh build's, checked in SQL. After `VACUUM` the files are 39.2 and 10.3 MB, against 36.2 and 9.8 MB fresh; the rest is the log ([[Folder Store Garbage]]). SQLite reuses the freed pages; the file shrinks only on `VACUUM`.
