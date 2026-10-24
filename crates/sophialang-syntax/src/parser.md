#implementation

> Recursive-descent parser: tokens → surface items. Source: `parser.rs`.
>
> Sources: [[SPEC-005 Rendering Profiles]] §6. code: `parser.rs`

The parser is **profile-agnostic**. It accepts:
- `function`, `fn` or `def`;
- `end` blocks or braces;
- `::` or `:` annotations;
- `)::R` or `) -> R` return types;
- `→` or `-->`, and `≤` or `<=`.

So any editable profile's output parses with one parser.

Newlines matter only between the parts of `let`, `match` and blocks, and are ignored inside brackets.

Precedence, loosest first: arrows (lambda `->`, Π `-->`, both right-associative), comparisons (non-associative), `+` and `-`, `*` and `/`, application.

A binder before `->` or `-->` is recovered from the parsed left side (`to_binder`). For `-->`, any other expression is an anonymous domain.

**Not parsed:** the `python` profile's indentation blocks. That profile is display-only until it has its own parser.

**`let` with braces** (2026-10-21): `let x = v {` body `}` is read when the brace is on the `let`'s line (no newline token between the value and the brace), as `match s {` … `}` already was; otherwise `let x = v [in]` body `end`. The random round trip (`tests/round_trip.rs`, 20 000 cases in both profiles) also checks that Rust text contains no `end` line.

**Diagnostics name tokens as written** (2026-10-21): ``expected `end` (found `)`)``, not the token's Debug form (`Sym(")")`). `Tok`'s `Display` is in [[lexer]].

**Any depth (2026-10-24, SP-015).** `expr` and `atom` (every recursive cycle passes through one of them) call `deep::grow`; the limit is a node budget (`Limits::max_nodes`, counted in `atom`), not nesting. `tests/deep.rs` parses 100 000-deep texts on a 256 KiB stack.
