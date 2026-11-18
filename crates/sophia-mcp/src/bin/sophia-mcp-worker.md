#implementation

> The query worker of [[sophia_mcp]]: [[bounded]]'s `worker_main`, built into this crate so that it sits beside `sophia-mcp`. Source: `sophia-mcp-worker.rs`.
>
> Sources: [[bounded]], [[sophia-query-worker]]. code: `sophia-mcp-worker.rs`

The same program as [[sophia-query-worker]], built with this crate's backends (SQLite). Cargo does not build a dependency's binaries, so the server's crate carries its own copy. The protocol is private to `bounded.rs`; server and worker must come from the same build.
