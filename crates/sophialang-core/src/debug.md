#implementation

> `Debug` output without recursion: a value describes one level of itself as a `DNode` (a leaf, a tuple, a struct, a list, or an unexpanded child), and `render` walks the description with an explicit stack, in the layout of `#[derive(Debug)]` for both `{:?}` and `{:#?}`. Used by `Term` and `SExpr` (SP-015). Source: `debug.rs`.
>
> Sources: [[SP-015 Stored definitions deeper than the parse limit cannot be edited]]. code: `debug.rs`.

- **Layout:** `Name(a, b)`, `Name { a: x }`, `[a, b]`, a unit variant as its name; in alternate mode one field per line, indented four spaces, each followed by a comma, as derive does. A multi-line leaf (a struct printed with `{:#?}`) has its continuation lines indented to its depth.
- **Checked** against derive with a mirror type (`sophialang-syntax/tests/round_trip.rs`, `debug_like_derive`, 2 000 random terms, both modes).
- `{:#?}` of a deeply nested value is quadratic in size by nature (every level indents further); `{:?}` is linear.
