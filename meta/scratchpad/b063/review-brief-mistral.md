# Review: the permission classes of `sophia-mcp` (Sophia, B-063)

You are reviewing a change in the Rust workspace at your working directory (a git worktree on
branch `mistral/b063-review`). Find **defects only**: wrong results, a way around a check, panics,
missing checks, races, missing tests. Do not suggest style changes or new features.

## What the code must do (the requirement)

`crates/sophia-mcp` is a Model Context Protocol server on stdio for AI agents, built on the `rmcp`
crate. It serves a Sophia store (a SQLite file). Its owner asked for this:

- Every tool has a permission class. Reading is always allowed. Each riskier class is served only
  when the server is started with that class's own flag.
- Classes: **Read** (`query`, `run`, `show`, `list`, `context`) always; **Write** (`edit`, `add`,
  `save_query`) only with `--allow-edits`; **Remove** (`rename`, `delete`) only with
  `--allow-removals`. The two flags are independent.
- A tool whose class is not allowed must be neither listed (`tools/list`) nor callable
  (`tools/call`).
- Without either flag, the server must not change the store file at all.
- No tool deletes content (definitions); `delete` removes a name only.
- Every query an agent sends (`query`, `run`) runs in a separate worker process under a memory
  bound; agents never reach SQL.

## Where to look

- `crates/sophia-mcp/src/sophia_mcp.rs`: `Class`, `TOOLS`, `Config::allows`, `SophiaServer::open`
  (opening the store, removing routes), `permit`, every tool method, and the
  `#[tool_handler(router = self.tool_router)]` impl.
- `crates/sophia-mcp/src/bin/sophia-mcp.rs`: argument parsing.
- `crates/sophia-mcp/tests/stdio.rs`: the existing tests (they start the real binary and speak raw
  JSON-RPC lines). Do not repeat what they already check.
- The `rmcp` 3.11 source is in `~/.cargo/registry/src/index.crates.io-*/rmcp-3.11.0/` and the
  macros in `rmcp-macros-3.11.0/`, if you need to know how routing, listing and calling work.
- The interface it calls: `crates/sophia-interface/src/edit.rs` (`write_text`, `create_text`,
  `rename`, `delete`) and `crates/sophia-query/src/bounded.rs`.

Questions worth checking: can any path call a Write or Remove operation without its flag (another
tool, a parameter, a stored query, an MCP method other than `tools/call`)? Does opening the store
without flags really write nothing (WAL files, profiles, schema)? Can `run` or `save_query` reach
something other than a bounded read? Can argument parsing turn a flag on by accident?

## How to work

You cannot run `cargo`. To show a finding, write a Rust test that would fail because of it, in
`crates/sophia-mcp/tests/review_mistral.rs` (copy the `Client` helper from `tests/stdio.rs`). The
maintainer will run it. Comment each test with the finding it shows and what it expects.

**Write your answer to `scratch/b063/ANSWER.md` as you go:** after each probe, append one entry
(what it checked, the test's name, and either "no defect, because …" with the code that shows it,
or the finding with the lines involved) before starting the next. A run that stops at its turn
limit must leave a usable report.

**By turn 70, stop probing and finish the answer file,** with the findings first, each marked
*verified by reading* (you traced the code) or *suspected* (you could not confirm it).

Do not commit, push, check out, or delete anything. Change no file outside
`crates/sophia-mcp/tests/review_mistral.rs` and `scratch/b063/`.
