#implementation

> Store errors. Source: `error.rs`.
>
> code: `error.rs`

- `Conflict` carries the expected and found versions, for Moonkale's optimistic edits.
- `InvalidPath` names the offending segment (SPEC-004 §3).
- `QueryRefused` covers everything the read-only guard rejects.
