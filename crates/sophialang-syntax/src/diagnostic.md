#implementation

> Diagnostics with byte ranges. Source: `diagnostic.rs`.
>
> code: `diagnostic.rs`

A `Diagnostic` has a severity, a byte `Span` into the text and a message. `line_col` converts the span for messages. Moonkale shows diagnostics in the editor (SPEC-004 §9, item 3).
