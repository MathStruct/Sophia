#reference

> A read-only correctness review of the `v0.1.9` code diff (`07ce70e..c19862a`, without the golden files and notes), by Mistral through the `vibe` CLI on 2026-11-08. Claude (Sophia) asked for it, after Daniel allowed handing tasks to Mistral/GLM. It found five things, all checked against the code and all real. Four are fixed with tests, and each test fails without its fix. One fix (the timer guard on an error path) has no test. It found no way past the DuckDB view check.
>
> Sources: the review's output, kept outside the repository at `~/.cache/sophia-corpora/vibe/review-v0.1.9.txt`. code: `duckdb_store.rs`, `logic.rs`, `folder.rs`, `sophia_interface.rs`, `tests/symmetric.rs`, `tests/folder.rs`, `tests/conformance.rs`.

## How it was run

```sh
vibe -p "<the prompt>" --trust --enabled-tools read_file --enabled-tools grep --auto-approve --max-turns 60 --max-price 3
```

The first run used `--agent plan` without pre-approved tools. It stopped after one sentence, probably waiting for a tool approval that cannot be given without the interface.

## Verdicts

| # | claim | verdict | done |
|---|---|---|---|
| 1 | DuckDB `query_readonly`: if `conn.transaction()` fails, or the code panics (`max_rows + 1` overflows for `usize::MAX` in a debug build), the timer thread outlives the call and interrupts a later query on the connection | **right** | The timer is a drop guard, joined on every path. The row cap is clamped to `i64::MAX`: DuckDB refuses a larger `LIMIT`, which the review missed. Test: `duckdb_queries_are_read_only_capped_and_timed` with `max_rows: usize::MAX`. Without the clamp, a debug build panics. The guard's error path has no test |
| 2 | `is_foreign_normaliser` accepts any `moonkale-` prefix, so a record named `moonkale-typo` with wrong bytes verifies as `Foreign` | **right** | Only WP-66 counts. In `v0.1.10` that was `moonkale-wp66@` alone, which made every definition in Moonkale's folder store `Invalid`, because it writes `moonkale-wp66-imports-v1@8` (Moonkale's report; Moonkale stayed on `v0.1.9`). In `v0.1.11`, `moonkale-wp66-` counts too. Test: `tests/symmetric.rs`, a forged `moonkale-typo` record is `Invalid` and an `-imports-v1` one is `Foreign` |
| 3 | the doc comment of `query_readonly` and the top of `duckdb_store.md` still say tables can be read | **right** | Both corrected |
| 4 | `text_source` splits the name at every dot, so a backquoted operator with a dot (`` jl.P.`a.b` ``) looks for `` b` `` | **right** | `folder::name_word`: the last segment, quotes removed, used by the store and the interface. Test: `text_source_follows_the_name`, where `` `x.b` `` comes first |
| 5 | `check_components` checks only the first size a batch claims for a component | **right** | Each claimed size is checked. Test: `tests/symmetric.rs`, a pair whose second record claims size 3 |

Areas it checked and found sound: the parse-tree walk in `only_views` (subqueries, lateral joins, set operations, CTE shadowing, table functions, non-SELECT statements). It notes that scalar functions are not checked by name, which is covered at run time by `enable_external_access = false`. It also checked annotation lookup by primary key, the order of checks in `Store::verify`, and that collection removes a component only as a whole.
