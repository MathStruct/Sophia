#reference

> Three correctness reviews of the agent server `sophia-mcp` on 2026-11-18: Codex (GPT) on B-062 (the read tools) and on B-063 (every tool, in permission classes behind flags), and Mistral Large 4 on B-063's permission check, the second reviewer for an authorising check. Four defects, all reproduced by a failing test and fixed; one observation accepted as documented behaviour.
>
> Sources: the briefs and answers in `meta/scratchpad/b063/`. code: `crates/sophia-mcp/src/sophia_mcp.rs`, `tests/stdio.rs`, `tests/review_mistral.rs`.

## How they were run

```sh
codex exec review --uncommitted -o REVIEW-from-codex.md        # B-062, then again for B-063
vibe -p --prompt-file review-brief-mistral.md --agent review --trust \
     --workdir ../Sophia-mistral --max-price 1 --max-turns 100   # B-063, branch mistral/b063-review
```

Mistral got the requirement (SR-157, SR-158), not the implementation's reasoning. Its review agent cannot run `cargo`, so it wrote its probes as `tests/review_mistral.rs`, and Claude ran them. Cost: $0 (Mistral Large 4 on the education plan).

## Findings

| # | review | finding | verdict | what was done |
|---|---|---|---|---|
| 1 | Codex, B-062 | `context` with `direction: "in"` starts at the name node, whose only incoming edge is `Contains` from its namespace, so it never reaches the users | **right**: `context_in_finds_the_users` failed (the answer held the name and `sample`) | walk from the definition the name points to; the name stays in the answer |
| 2 | Codex, B-063 | `run` refuses a name path to a nominal entity as an `Id` parameter, though its hex id works | **right**: `review_findings_b063` failed with "names a nominal entity" | `Target::Nominal` becomes `Val::Id` like `Target::Content` |
| 3 | Codex, B-063 | `add` returns version 0 for code (`create_text` has no edited name), so `edit` with it conflicts | **right**: the test failed, 0 against the real token | `add` answers with the created name's `text_token` |
| 4 | Codex, B-063 | `add` reports `namespace.name` even when the text defines another name | **right**: with the old path logic the test failed ("no name sample.foo") | `add` answers with the key `create_text` returns |
| 5 | Mistral, B-063 | (its test `read_only_session_creates_no_sidecar_files`, expected to pass) a read-only session leaves the directory as it was | **failed, but not a defect**: the session leaves `s.sqlite-shm` and a 0-byte `s.sqlite-wal`, which every SQLite reader of a WAL-mode file creates; the main file's bytes and mtime are unchanged. `immutable=1` would avoid them but is unsafe while another process writes | documented in [[sophia_mcp]]; the test, renamed `read_only_session_writes_nothing_to_the_store`, allows exactly those two files and requires the log to be empty |
| 6 | Mistral, B-063 | no way to reach a write or remove operation without its flag (another tool, a parameter, a stored query, another MCP method); `delete` cannot reach a namespace; parsing cannot turn a flag on; two edits with one version: one wins | **no defect**: its six other tests pass | kept as regression tests |

Mistral also noted, as not defects: query text is parsed in the server before the bounded worker runs it (the message is already in memory then), and with a write flag `Interface::open` writes the built-in rendering profiles if missing (only when writes are allowed). It could not read the `rmcp` source (outside its readable paths) and relied on the tests for the routing.

Found by Claude's own tests before review, for completeness: `#[tool_handler]` without `router = self.tool_router` serves a fresh router with every tool, so a read-only server listed all ten; the second check (`permit`) still refused the calls.
