#implementation

> The `sophia-mcp` command: the agent server on stdio ([[sophia_mcp]]). Source: `sophia-mcp.rs`.
>
> Sources: [[sophia_mcp]]. code: `sophia-mcp.rs`

```sh
sophia-mcp --store <path> [--backend sqlite|redb|lmdb|duckdb] [--worker <path>] [--max-memory <MiB>] [--reader <name>] [--allow-edits] [--allow-removals]
```

- `--store`: an existing store (a file; LMDB's is a directory); opened read-only unless a flag below allows writes.
- `--backend`: the store's backend, `sqlite` by default. redb and DuckDB are served read-only: the write flags are refused (exit 1), because the worker cannot open their file while the server holds it for writing (Daniel, 2026-11-18, SR-159). A backend the binary was built without (DuckDB by default; cargo feature `duckdb`) is an error at start.
- `--worker`: the query worker; by default `sophia-mcp-worker` beside this binary.
- `--max-memory`: the worker's memory bound in MiB (default 512).
- `--reader`: whose rendering profile `show` prints in (default `agent`).
- `--allow-edits`: serve `edit`, `add` and `save_query`.
- `--allow-removals`: serve `rename` and `delete`.

Without the two flags the server is read-only ([[sophia_mcp]], "Permission classes").

It exits 2 on bad arguments, 1 when the store cannot be opened or the session fails, 0 when the client closes stdin. Logs go to stderr; stdout carries only MCP messages.

To use it from Claude Code: `claude mcp add sophia -- <path>/sophia-mcp --store <file.sqlite>` (add `--backend …` for another backend).
