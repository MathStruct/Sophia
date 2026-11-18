#implementation

> The store `sophia-mcp` opens, on any of the four backends (B-064): one enum, `AnyBackend`, implements `Backend` by dispatch, so the server holds one `Interface<AnyBackend>` whatever the backend; `Kind` says how each backend is opened and whether it can serve writes. Source: `backends.rs`.
>
> Sources: [[bounded]] ("Backends"); [[Database Comparison]]; SR-159. code: `backends.rs`, `tests/backends.rs`

## Why an enum

`Backend` has generic methods (`read<R>`, `write<R>`), so it is not object-safe and there is no `dyn Backend`. Making the server generic over `B` would put a type parameter on every `rmcp` handler. An enum with one arm per built backend costs one `match` per transaction.

## Which backend serves what

| backend | store | read-only server | writing server |
|---|---|---|---|
| SQLite | a file | `Sqlite::open_read_only` | `Sqlite::open`; the worker reads beside it (WAL) |
| LMDB | a directory (`data.mdb`, `lock.mdb`) | `Lmdb::open_read_only` | `Lmdb::open`; the worker reads beside it (LMDB is multi-process) |
| redb | a file | `Redb::open_read_only` (a shared lock) | **refused** |
| DuckDB | a file | `DuckDb::open_read_only` (one thread) | **refused** |

- **redb and DuckDB are read-only here** (Daniel, 2026-11-18, SR-159). Both lock their file against every other process while it is open for writing, so the worker could not open it. Closing the store around each query, or giving the worker a copy after each write, were the alternatives; neither was chosen. Their read-only opens share the file, so a read-only server and its worker both have it open at once (tested).
- **Every store is first opened read-only and read** (records, names, log, meta), whatever the flags; a failure is "not a Sophia … store". Only then is a writing server's store opened read-write, so an empty file or another database is never initialised into a store (Codex's review of B-064).
- `Kind::exists` checks the store's shape before anything is opened: a file, or for LMDB a directory holding `data.mdb`. `Lmdb::open` would otherwise create the directory; the server never creates a store.
- A backend the binary was built without (cargo features `redb`, `lmdb` on by default, `duckdb` off) is an error at start. SQLite is always built.
- A read-only LMDB session still writes its reader slot in `lock.mdb`, as every LMDB reader does; `data.mdb` is unchanged (tested).
