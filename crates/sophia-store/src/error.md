#implementation

> Store errors. Source: `error.rs`.
>
> code: `error.rs`

- `Conflict` carries the expected and found versions, for Moonkale's optimistic edits.
- `InvalidPath` names the offending segment (SPEC-004 §3).
- `QueryRefused` covers everything the read-only guard rejects.

**`NameClaimed { name, owner }`** and **`NormaliserChanged { stored, new }`** (2026-10-24) come from `replace_files` ([[folder]]): a name owned by a file outside the batch (its path in `owner`, for a retry with that file added), and a folder built with another normaliser. **`NotDerived`** (SR-146): `collect_folder` on a store that is the source of truth, where history is kept.
