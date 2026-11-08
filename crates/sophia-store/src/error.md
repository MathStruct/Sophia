#implementation

> Store errors. Source: `error.rs`.
>
> code: `error.rs`

- `Conflict` carries the expected and found versions, for Moonkale's optimistic edits.
- `InvalidPath` names the offending segment (SPEC-004 §3).
- `QueryRefused` covers everything the read-only guard rejects.

**`NameClaimed { name, owner }`** and **`NormaliserChanged { stored, new }`** (2026-10-24) come from `replace_files` ([[folder]]): a name owned by a file outside the batch (its path in `owner`, for a retry with that file added), and a folder built with another normaliser. **`NotDerived`** (SR-146): `collect_folder` on a store that is the source of truth, where history is kept. **`Derived`, `FileConflict { conflicts }`, `LaidOut { path, token }`, `ExportMismatch { path }`** (2026-10-28, SPEC-010): a store-truth call on a derived store; files whose tokens changed, as (path, expected, found); an interface edit of a name a file lays out, with that file and its token for the retry; and `give_truth` given an export that is not the stored text. **`NotInFiles { name }`** (2026-10-28, Codex review): `give_truth` while a name is in no file. **`UnreachedRecord { path, id }`** (2026-10-26): `replace_files` given a record that no name, span or generic membership of its file reaches; collection could never remove it ([[RQ-002 When Incremental Collection Equals a Fresh Build]]).

**`IncompleteComponent { component, size, stored }`** (Gemini's review, 2026-11-07 ([[Review 2026-11-07 Gemini (external)]]); fixed 2026-11-08): a write would leave a recursive component with other than `size` distinct members. It is a frontend bug (Moonkale's normaliser before `06fd112c`); see [[logic]].
