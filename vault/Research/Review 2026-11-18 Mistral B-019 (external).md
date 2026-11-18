#reference

> A defect-only review of the B-019 design, [[SPEC-007 Sophia Query Language]] §13 (query functions), by Mistral Large 4 through the `vibe` CLI (`ask` agent, read-only, 2026-11-18). It read the design, SPEC-006 and `crates/sophia-query`, and reported 8 findings. All 8 are right at least in part; each is fixed in the design text. Nothing is built, so no finding was reproduced by a test; the tests that would show them are named in the build rows B-067 to B-070.
>
> Sources: the review's answer, kept outside the repository at `~/.cache/sophia-corpora/vibe/review-b019.md`. code: `crates/sophia-query/src/plan.rs`, `canon.rs`, `sophia_query.rs`, `sql.rs`, `syntax.rs`.

## How it was run

`vibe -p --agent ask --max-price 1 --max-turns 40` on the main checkout with a brief giving SR-111, the diff and the request for defects only, with counter-examples. Free model; the agent could not run commands (headless `ask`), so its code claims are from reading. Claude checked each against the code it names.

## Findings

| # | finding | right? | done |
|---|---|---|---|
| 1 | §13.1's `Safe` ("result drawn from a finite set fixed by its arguments") holds for every function, so it cannot separate `+` from `parent` | **Right** | §13.4 now defines `Safe` by a closure `C` to finite sets, closed under itself, with `f(x…) ∈ ⋃ C(xᵢ)`; §13.8 states `C` per function; a property test checks it (B-069) |
| 2 | "output reaches the head" is undefined; `z == x + 1 && y == z` escapes a direct check | **Right** | §13.4: a flow graph over the rule's variables (call inputs → output, `==` both ways); refuse if an `Inventive` output reaches a head argument in a recursive rule |
| 3 | A negated partial test translated as `NOT tpl` is `NULL` where undefined, under SQLite's three-valued logic | **Right**, and §13.3 was also unclear on what `!f` means where `f` is undefined | §13.3: a partial test holds only where defined and true, its negation only where defined and false; §13.6 gives the three translations with `IS NOT NULL` |
| 4 | The migration contradicts SPEC-006 §6 (`migrated_to`, not lineage); `load`'s canonicity check refuses anything but `query-v0` | **Right** (checked: `canon.rs` refuses other headers) | §13.2 follows SPEC-006 §6; `load` made version-aware; both in B-067 |
| 5 | `Def(id)` would not become a `ref`: `Prepared::record` takes refs from `canon::constants`, which sees only terms | **Right** about today's code; the design described the goal | §13.2 and B-070 say `canon::constants` must include `FnRef::Def` |
| 6 | A `Def` call pins the function's code but not the interpreter's semantics, so a result id could survive a change of answer | **Right** | §13.7: the evaluator's semantic version goes into the query header with any `Def` call; in B-070 |
| 7 | "ranks like `!=`" (score 100) contradicts "Heavy after every narrow lookup" (60+); `name_prefix` and the parser keyword are keyed on the old literal | **Right** (checked: `plan.rs` scores, `name_prefix`, the `startswith` keyword) | §13.5 gives scores per cost class and re-keys both; B-067's acceptance keeps the prefix-range test |
| 8 | §13.3's list of what binds an argument omits equations, which `plan::bind` counts | **Right** (wording) | §13.3 now uses `plan::bind`'s sense |

It also confirmed, from the code: the `startswith` SQL template is exact for valid UTF-8; SQLite's `length` counts characters and integer overflow turns into floating point; `t_name.path` uses `BINARY` collation; negated calls need no stratification.
