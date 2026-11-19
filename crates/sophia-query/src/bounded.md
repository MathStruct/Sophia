#implementation

> A hard memory bound for untrusted queries (SR-108, B-017): the query runs in a worker process whose data segment the kernel limits, so going over the bound ends the worker, never the caller. Source: `bounded.rs`.
>
> Sources: [[SPEC-007 Sophia Query Language]] §6.0; [[Review 2026-11-10 Fable (external)]] S-3; Linux `setrlimit(2)` (`RLIMIT_DATA` covers private writable mappings since Linux 4.7). code: `bounded.rs`, `bin/sophia-query-worker.rs`, `tests/bounded.rs`

## Why a process

The executor's byte budget (`Limits::max_bytes`) is an estimate: it counts what the executor keeps, not temporaries, allocator overhead or the backend's C allocations. Daniel chose a subprocess over a counting allocator (2026-11-18), because nothing inside one process gives a hard bound:
- SQLite's `sqlite3_hard_heap_limit64` is process-wide, and covers SQLite only;
- on stable Rust a failed allocation aborts the process (`handle_alloc_error`), so a counting allocator can only refuse by killing the caller, or detect and overshoot by up to one allocation.

## How it works

- `run_bounded(bound, store, prepared, params, limits, strategy)` starts `bound.worker` (the `sophia-query-worker` binary) with the bound in bytes as its argument. It sends one request on stdin: the store kind and path, the query's canonical bytes and names, its id, the parameters, the limits and the strategy.
- The worker first sets `RLIMIT_DATA` on itself, then reads the request, prepares the query again (and refuses one whose id differs), opens the store **read-only** (`open_read_only` on each backend; nothing is created or written), runs `Prepared::run` in one read transaction, and writes the rows or the error on stdout.
- **Over the bound**, one of three things happens, and each becomes `Error::MemoryBound { limit }`:
  - a Rust allocation fails: the worker prints Rust's "memory allocation of … failed" and aborts (SIGABRT);
  - the backend reports that it is out of memory (SQLite's `SQLITE_NOMEM`, DuckDB's "Out of Memory Error"): the worker answers with the `memory` error kind;
  - the kernel's out-of-memory killer ends the worker (SIGKILL).
- **Time:** the executor stops itself at `limits.deadline`; the caller kills a worker still running `bound.grace` (5 s) later and returns `Error::Limit`.
- **Waiting (B-066):** the caller sleeps until the worker's stdout and stderr both reach end of file (each reader thread signals a channel; `recv_timeout` up to the deadline), then reaps the worker, polling `try_wait` with a pause growing from 20 µs to 2 ms; a worker that closed its pipes and still runs is killed at the same deadline. Before B-066 it polled `try_wait` every 2 ms, so every answer came up to 2 ms late (measured: worker times stepped by about 2.08 ms, [[Benchmark Results]]).
- **Other errors** come back as their own variant with their message. A store error keeps only its message (`StoreError::Backend`). A worker that cannot start or ends without an answer is `Error::Worker`.
- `Rows::stats` is zero: the worker does not send its statistics.

## What it covers and what it does not

- **The bound covers the whole worker:** the executor, the backend's allocations, thread stacks and a few MiB of fixed overhead. SQLite's page cache is heap and counts. File mappings do not: LMDB's 64 GiB map is shared and file-backed.
- **Backends.** SQLite (WAL) and LMDB can be read by the worker while another process writes. redb and DuckDB lock their file: the worker can open them only while no process holds the store for writing, so `sophia-mcp` serves them read-only (SR-159, [[backends]]): its own read-only open shares the file with the worker. DuckDB's read-only open uses one thread, since each thread's stack counts against the bound.
- **Linux only.** macOS does not enforce `RLIMIT_DATA` on `mmap`, which large allocations use, so `run_bounded` returns `Unsupported` off Linux. The module is not built for wasm (no processes).
- **Smallest bound:** below about 1 MiB the worker cannot start, and every query fails with `MemoryBound` (Codex's sweep, [[Review 2026-11-18 Codex B-017 (external)]]).
- **Cost** (measured 2026-11-19, B-065/B-066, [[Benchmark Results]]): the worker process alone 1.3 ms; an empty query through it 1.5 ms (LMDB, redb) to 1.9 ms (SQLite); whole-relation queries 10–40 % slower than in-process (a cold process).
- **Not covered:** the raw `sql` dialect (user SQL on SQLite and DuckDB). It has its own guard (`SQLITE_LIMIT_LENGTH`, a result byte budget, DuckDB's `memory_limit`) and agents do not reach it (SR-100).

## Tests (`tests/bounded.rs`)

Found by review and fixed (each in `review_findings`): an oversized request, a non-UTF-8 store path, a deadline too far to add ([[Review 2026-11-18 Codex B-017 (external)]]).

`hanging_worker_is_killed_at_the_deadline` (B-066): two stand-in workers (shell scripts) that hang, one with its pipes open and one after closing them, are each killed after `deadline + grace` and give `Limit`. `a_grandchild_holding_the_pipes_does_not_hold_the_caller` (Mistral's review, [[Review 2026-11-19 Mistral B-021 B-066 (external)]]): a worker that forks a child holding its pipes and hangs still gives `Limit` at the deadline; after a kill the caller does not join its reader threads, and it never joins the writer.

On all four backends, with every logical limit and the byte estimate switched off: a query deriving 700² pairs fails with `MemoryBound` under 64 MiB, and a transitive closure gives the same rows as an in-process run under the same bound. On SQLite with the store held open by the caller: the same closure runs, a logical limit comes back as `Limit`, a missing store as `Store`, a missing worker binary as `Worker`. Measured once (2026-11-18): the pair query with N = 1 500 (2.25 M pairs; the executor's estimate: 936 MB held) answers under 6 GiB (13 s, debug build) and fails under 128 MiB (1.4 s).
