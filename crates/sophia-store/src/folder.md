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

**Not yet:** garbage collection of records no manifest references; Rust and Python names in Moonkale's crate-and-module form (the producer computes names, so this is Moonkale's side).

**Facts follow their file (2026-10-24, found by Moonkale's Codex before WP-21).** At `cafe940e`, `replace_files` stored a file's facts but never retracted them: a replacement with fewer facts, or deleting the file, left them in `facts_of`. Now:
- the manifest (`sophia-file-manifest@1`; `@0` manifests still decode, with no facts) lists the file's fact ids;
- the meta row `fact/<id>` counts the files contributing a fact; a fact is stored with its first contributor and removed (`WriteTx::remove_fact`, all four backends; logged as a `fact` entry with only an old id) with its last;
- a fact already stored before any file contributed it is marked `fact-external/<id>` and never removed by the folder writer;
- all of it happens in the batch's transaction, so a failed batch leaves facts and manifests as they were.
Tested on all four backends (`facts_follow_their_files`: replacement, a resolution change with an unchanged body, two contributors, deletion while the record lives on in another file, an external fact, rollback) and with Codex's reproducer (`next17-sophia-probe`, 2 of 2, failing on `cafe940e`).

**Migration:** folder stores written before this have no fact provenance; their facts stay until the store is rebuilt. No folder store is in use yet.
