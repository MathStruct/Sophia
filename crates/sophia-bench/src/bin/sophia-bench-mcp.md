#implementation #benchmark

> B-11 through the agent server (B-065): how long a query takes when an agent asks for it through `sophia-mcp`, per backend and per form (`query` with text, `run` with a stored query), split into the layers it passes through: the executor in-process, the bounded worker, the server's own work, and the MCP round trip on stdio. Prints the Markdown that goes into [[Benchmark Results]]. Source: `sophia-bench-mcp.rs`.
>
> Sources: [[Benchmark Plan]] B-11; [[bounded]]; [[sophia_mcp]]. code: `sophia-bench-mcp.rs`

```sh
cargo build --release -p sophia-mcp --all-features      # the server and the worker, beside this binary
cargo run --release -p sophia-bench --bin sophia-bench-mcp -- --size 10000 --runs 5
cargo run --release -p sophia-bench --bin sophia-bench-mcp -- --size 100000 --runs 5 --backends sqlite,redb,lmdb
```

- **Store:** `sophia_bench::generate(42, N)` ingested through `Interface::open` (so the built-in rendering profiles are there), plus the six B-17 workloads stored as queries `bench.<slug>`. Built in a temporary folder under `--dir` (default `$SOPHIA_CORPORA`, never `/tmp`) and removed after.
- **Limits:** the server's (`Config::new`: 1 000 rows, 10 s, 512 MiB) in every layer, so every layer does the same work; a result over 1 000 rows is cut there in all of them. Each layer's row count is checked against the in-process one.
- **Fixed costs** per backend: a read-only open; `prepare_text`; an empty query (one empty range read) in-process, through the worker and through MCP `query`; the worker process alone (started with an empty request, waited for by its exit); MCP `list` (the protocol and the server without the worker); a server start with the `initialize` handshake.
- **Workloads:** the three per-probe ones (20 random definitions, `--runs` rounds) through `run` only, since `query` refuses parameters; the three whole-relation ones through both `query` and `run`. Columns: native in-process, SQL in-process (SQLite only; agents do not get SQL, SR-100), `run_bounded` from the bench, `SophiaServer::run_stored` in the bench process (adds `load`, parameter resolution and the `ids` description), MCP `query` and `run` on stdio.
- Median and 90th percentile of the calls; the in-process layers run in a first pass and the server's in a second, because LMDB lets a process open an environment only once.

## Results

2026-11-19 (B-065, and again after B-066): [[Benchmark Results]]. The run of 2026-11-18 was spoiled by load from other sessions and discarded.
