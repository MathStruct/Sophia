#implementation

> Printer: core definitions → text in a profile. Source: `print.rs`.
>
> Sources: [[SPEC-005 Rendering Profiles]] §3 to §7. code: `print.rs`

- **Binder names:** display names where they are usable. Otherwise `x`, and `_` when unused. Names are freshened against the scope, the items being printed and the built-ins.
- **References:** printed by name only after checking that the name resolves back. Otherwise as `#hex` or `##hex`.
- **Members of the printed recursive group** are printed by their names, and so are drafts after a parse.
- **Infix** for the built-in operators, with minimal parentheses.
- **`let` and `match`** print as indented blocks in the profile's block style (2026-10-21): `let x = v` … `end` with `blocks = end`; `let x = v {` … `}` and `match s {` … `}` with braces; `let x = v:` and `match s:` with indentation (display only). Before, they always ended in `end`, so a Rust-profile text mixed both styles (found by Moonkale's desktop check). Negative literals are parenthesised, so a block body never starts with `-`.
- **Profile settings honoured:** `kw.function`, `blocks` (`end`, braces, indent), `type.annotation`, `operators`.

**Not yet:** `type.show`, `implicit.show`, `generic.call`, `index.base` (no arrays yet), and comments inside bodies (SP-001).

**Any depth (2026-10-24, SP-015).** `term_level` and `occurs` go through `deep::grow`. Three costs that grew faster than depth, found at 4 000 levels and fixed:
- whether a binder is used was an `occurs` walk per binder (quadratic); now `binder_uses` computes it for a whole item in one pass;
- `fresh` scanned the scope for each candidate suffix (cubic for a chain of same-named binders); now `Scope` counts names and keeps, per base name, the smallest suffix that might be free (same names as before: 1 000 generated definitions print identically in both profiles);
- indentation grew 4 columns per nested block, so text was quadratic in depth; it now stops growing at 64 levels (`MAX_INDENT`), keeping the line breaks.
Measured (release, `tests/deep.rs` probe): at 16 000 levels a sum chain prints in 56 ms, a lambda chain in 29 ms, a let chain in 5.0 s for 8.5 MB. **Still superlinear:** each level copies its children's text into its own string, which dominates for nested `let`s; the fix is a printer that writes into one buffer or builds a document tree (SP-015 step 1b).

**An explicit-stack printer (2026-10-24, SP-015 step 1b).** Terms are no longer printed recursively. `Printer::doc` runs a machine of steps: `Eval` a child at a minimum level, `Push` / `Pop` / `Truncate` scope names, `Indent`, `Combine` the top children's text with literal pieces, and `Con` for constructors. Each node is planned when its `Eval` step runs, so scope and indentation are exactly what the recursive printer saw, and binder names are chosen the same way. Text is collected as a tree of pieces (`Doc`) and joined once (`flatten`), so no parent copies its children's text. `binder_uses` and `occurs` use explicit stacks too.
- **Same output:** the 1 000 generated definitions and the 18 sample definitions print byte-identically in all four profiles (compared with the previous build); 20 000 random round trips pass.
- **Linear:** at depth 100 000 (release) a sum chain prints in 0.31 s, a lambda chain in 0.06 s, a let chain in 0.20 s for 53 MB; before, a let chain took 5.0 s at 16 000 levels.
- **Any depth on wasm:** no recursion is left in printing (`tests/wasm_deep.rs`).
