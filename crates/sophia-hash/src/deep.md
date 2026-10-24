#implementation

> Recursion at any depth: one function, `deep::grow`, that every recursive pass over terms, syntax trees and encodings calls at each step. Source: `deep.rs`.
>
> Sources: [[SP-015 Stored definitions deeper than the parse limit cannot be edited]]; SR-145 in [[Requirements]]. code: `deep.rs`; the `stacker` crate (rust-lang/stacker), as used by rustc's `ensure_sufficient_stack`.

- **Native targets:** `stacker::maybe_grow(RED_ZONE, SEGMENT, f)`. With less than 256 KiB of stack left, `f` runs on a new 4 MiB segment allocated on the heap; depth costs memory, never an overflow. The test recurses 1 000 000 levels on a 256 KiB thread.
- **wasm32: a placeholder** that runs `f` unchanged (Daniel asked for the wasm case to be replaceable, 2026-10-24). `stacker` can at most move wasm's shadow stack, not the engine's call stack, so it would not help; psm's README lists wasm as unsupported. Until a wasm solution is chosen, deep recursion there is bounded by the engine's stack. Passes that the read-only web client needs can also be rewritten with explicit stacks, as `encode::called_refs` is.
- **Who still uses it (2026-10-24):** only the write path and debugging: the parser (`expr`, `atom`), the elaborator, `SExpr`'s traits and `Term`'s `Debug`. Everything a reader needs (encode, decode, display names, walk, map, clone, eq, hash, print, drop) uses explicit stacks and works at any depth on wasm too. On wasm the write path is bounded by the engine's stack; writes there need a clock as well, and are not supported yet.
- **Why one function and not `stacker` at each site:** the wasm replacement, and any tuning of the sizes, happen here only.
