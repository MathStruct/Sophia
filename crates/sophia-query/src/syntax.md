#implementation

> The query syntax: a lexer, a Julia-flavoured parser, and printers for the `julia` and `datalog` profiles. Source: `syntax.rs`.
>
> Sources: [[SPEC-007 Sophia Query Language]] §5; [[SPEC-005 Rendering Profiles]] (the round-trip guarantee). code: `syntax.rs`

- `parse` returns the IR in written order and `Names`. A file is `query NAME(p::Sort, …) … end`, with rules `head(args) = lit && lit` and one `select outs where body`. A bare `select … where …` is an anonymous query.
- Literals: `pred(args)`, `!pred(args)`, `a == b`, `a != b`, `startswith(a, b)`. Terms: variables, `_` (a fresh variable each time), parameters, `"text"`, `:Symbol` (text), integers, `0x` + 64 hex digits (an id).
- Bounds (SR-50): 64 KiB of text, 256 rules, 64 literals per body, 16 arguments.
- `print` in `Profile::Julia` parses back to the same IR (tested). `Profile::Datalog` is for display, including the magic-set program in `explain`.
