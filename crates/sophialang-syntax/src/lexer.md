#implementation

> Tokens for every profile, and the parse limits. Source: `lexer.rs`.
>
> code: `lexer.rs`

Tokens are:
- identifiers, and backquoted operator names;
- literals: `0x…` is an unsigned 64-bit integer; floats use Rust's shortest round-trip form; strings and characters take escapes;
- `#hex` (a content id) and `##hex` (a nominal id);
- `@markers`;
- symbols, including both `-->` and `→`;
- newlines, which only blocks treat as significant.

`#` followed by anything other than 64 hex digits starts a comment, as does `//`.

`Limits` default to 512 KiB, depth 128 and 4096 definitions per text (Moonkale WP-66's limits, SR-50).

`Tok` implements `Display` for diagnostics: a token as written, in backquotes (`` `)` ``, `` `@prim` ``, `` `##023be898216c…` `` shortened), a string or character literal quoted, and "end of line" or "end of text".
