#implementation

> Crate root of `sophialang-syntax`: rendering profiles, one printer per profile, and one parser for every editable profile. Source: `sophialang_syntax.rs`.
>
> Sources: [[SPEC-005 Rendering Profiles]]; [[S-ADR-0006 Edits through projections and the existing operations]]. code: `sophialang_syntax.rs`, [[lexer]], [[parser]], [[ast]], [[elab]], [[resolve]], [[print]], [[profile]], [[diagnostic]]
>
> Theory: [[Theory of the v0 Store]] §4 (names and printing as a lens).

`parse_text(env, ns, text, limits)` returns the items and any warnings, or the errors with byte ranges. It stores nothing. Since 2026-10-24 it runs on the caller's thread: parsing and elaborating use explicit stacks ([[parser]], [[elab]]), so any depth works without a big-stack thread (which wasm would not have). A panic still comes back as an error.

`print(env, ns, profile, items)` returns text.

## The round-trip guarantee and its tests (`tests/round_trip.rs`)

For every editable profile (`julia`, `rust`), printing, then parsing, then normalising gives the **same content id**, and printing again gives the **same text**. The tests check:
- every definition of the sample program, with recursive groups printed together;
- the exact Julia text of `add`, `double` and a weighted rule;
- relations;
- diagnostics and limits;
- a property test over random well-scoped definitions (400 cases by default; 20 000 passed with `PROPTEST_CASES=20000`).

## Difficulties met

- **Printing names that parse back.** The printer resolves every name it is about to print with the parser's own resolution ([[resolve]]), and falls back to `#<hex>` or `##<hex>` if the name would mean something else (a local shadows it, or a nearer namespace has the same name).
- **`Type {` versus `Type{1}`.** In the Rust profile, `data Nat: Type {` opens a block. A universe level is read only when the brace touches `Type`.
- **Implicit binders with usage ω** have no surface form: `{x}` means usage 0. Such binders do not occur in practice, and the random generator avoids them.
- **The stack.** A depth limit of 128 overflowed a debug test thread's 2 MiB stack (about 9 frames per level), hence, until 2026-10-24, a parser thread with 64 MiB. Replaced by explicit stacks and no depth limit (SP-015).
