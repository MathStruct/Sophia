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
