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
