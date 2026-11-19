#implementation #julia

> JuliaSyntax's source tree is the independent oracle for part 8 of [[Julia Conformance]]. The script parses source without loading or evaluating the package. Source: `parse.jl`.
>
> Sources: Julia 1.13.1, `Base.JuliaSyntax.parseall`, `SyntaxNode`, `ParseStream.diagnostics`, `byte_range`, `kind`, `flags`, `is_infix_op_call`. code: `parse.jl`, `src/conformance.rs` (`parse_ledger`), `tests/parse.rs`, `examples/julia_parse.rs`

Run from the repository root:

```sh
julia crates/sophia-ingest/tests/julia-semantics/parse.jl crates/sophia-ingest/tests/julia-semantics/parse/src | sort > crates/sophia-ingest/tests/julia-semantics/parse/truth.tsv
```

The oracle visits `.jl` files recursively in sorted order, with paths relative to SRCDIR. Output is TSV:

- `error file first_byte last_byte`: each diagnostic of level `:error`. `parseall(SyntaxNode, text; filename, ignore_errors=true)` supplies the tree; a second `ParseStream` parse supplies diagnostics, including recovery errors. Offsets are zero based UTF-8 bytes, with inclusive last bytes (tree-sitter and Moonkale use exclusive ends).
- `def file name_byte first_byte last_byte form name npos kwnames callkw`: a source definition. `name_byte` is the name token's first byte, the join key against `Def.name_span.0`. A callable object's head has name `-` and its head's first byte. Qualified names take the last component, operators retain their spelling, and `var"odd name"` retains its source spelling.
- `form`: `function`, `short`, `macro`, `struct`, `mutable_struct`, `abstract`, `primitive`, `const`, `typed_global`, or `module`. JuliaSyntax represents short methods with kind `function` and its short-form flag. A module-level typed assignment is a `typed_global` only for a plain identifier. Destructuring `const` emits a row for each binding.
- `npos`: positional parameters (including optional and vararg parameters as one each), or `-` for a non-method. A generic declaration `function f end` has zero. `kwnames`: sorted comma-separated keyword parameter names after `;`, or `-`.
- `callkw`: keyword arguments in calls in the body's source, both comma assignments and semicolon parameters (including shorthand). Signatures are excluded; nested definitions are separate units, not counted in an enclosing definition's body. This is a syntax count, not a claim about execution or macro output.

Traversal follows module/baremodule, toplevel, block, doc and macrocall wrappers, conditional and loop containers, struct bodies and nested method bodies. Definitions under `@inline`, `@doc`, docstrings and `@kwdef` count. Macro calls are not expanded. Nested named methods count as definitions; their signatures are skipped and typed assignments inside local scopes or structs are not module-level globals. Constant declarations are emitted once, without revisiting their assignment as a typed global. Quotes and string contents are not traversed for definitions. Part 9 will measure include discovery separately: the oracle reads all source files, while the ledger reports oracle files outside `module::discover` as `undiscovered`.

The fixture intentionally need not execute: methods on Base, callable constructor syntax and unexpanded macros are compared as source.
