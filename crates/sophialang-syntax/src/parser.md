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

**Any depth (2026-10-24, SP-015).** `expr` and `atom` (every recursive cycle passes through one of them) call `deep::grow`; the limit is a node budget (`Limits::max_nodes`, counted in `atom`), not nesting. `tests/deep.rs` parses 100 000-deep texts on a 256 KiB stack. (Superseded the same day: every pass now uses explicit stacks and `deep::grow` is gone; see below.)

**An explicit-stack parser (2026-10-24, later).** Expressions are parsed by a machine: `run` keeps a stack of `Frame`s, one for each place where a grammar rule waits for the value of a rule it called (`Arrow`, `Cmp`, `AddLoop`, `PostfixLoop`, `Args`, `Paren`, `LetValue`, `MatchCase`, …). `start` begins a rule, `resume` continues one with a value. The grammar, the spans and the errors are those of the recursive parser; `to_binder` is a loop. Any depth on every target: `tests/deep.rs` (native, 256 KiB stack) and `sophia-interface/tests/wasm_deep.rs` (Node and Firefox) parse 100 000-deep texts. At that depth (release): a 400 KB sum chain parses in 74 ms, a 1.1 MB lambda chain in 75 ms, a 54 MB let chain in 177 ms.
