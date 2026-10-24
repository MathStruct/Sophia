#implementation

> The core terms of SPEC-003 §2 as a Rust enum. Source: `term.rs`.
>
> Sources: [[SPEC-003 Sophialang Core v0]] §2; Atkey 2018 (usages). code: `term.rs`

- Variables are de Bruijn indices.
- References are `Ref(digest)` outside the recursive group, `Member(k)` inside it, and `Draft(i)` before hashing (an index into the batch being normalised).
- `Nominal(id)` names generic functions and nominal types.
- `App` is n-ary, so that dispatch sees all arguments.
- Binders carry a display name (not hashed), a usage (0, 1 or ω; hashed) and implicitness.
- Floats are stored as bits, so equality and hashing are exact.

`walk` visits subterms in the canonical stream order. `map` rewrites bottom-up (it is used to turn drafts into references).

**Not in v0:** effects in types, arrays, mutation, foreign calls, macros (SPEC-003 §2).

**Any depth (2026-10-24, SP-015).** `Term` implements `Clone`, `PartialEq`, `Eq`, `Hash` and `Debug` by hand (generated uniformly from the variants, the same results as the derived impls), each through `deep::grow`; `walk` and `map` too. `Drop` is iterative: children are moved onto a heap stack (`take_children`), because dropping nested `Box`es recursed once per level. Because `Term` has a `Drop` impl, code cannot move fields out of a term by pattern; `map` therefore rewrites children in place with `mem::take` (`Term::default()` is `Prop`). `tests/deep.rs` (in sophialang-syntax) runs encode, decode, clone, eq, hash, Debug, walk and map at depth 100 000 on a 256 KiB stack. (Superseded the same day: every pass now uses explicit stacks and `deep::grow` is gone; see below.)

**Explicit stacks everywhere on the read path (2026-10-24).** `walk`, `map`, `Clone`, `PartialEq` and `Hash` now use explicit stacks (`children` lists a node's subterms in canonical order; `shallow` copies a node without them; `slots` gives mutable access to them), so they need no stack growth and work at depth 100 000 in a wasm engine (`tests/wasm_deep.rs`, Node and Firefox). `Debug` is iterative too (below).

**`Debug` without recursion (2026-10-24).** `Term`'s `Debug` describes one level at a time to [[debug]]'s renderer, which prints derive's layout for `{:?}` and `{:#?}`. A test compares both modes with a mirror enum that derives `Debug`, on 2 000 random terms (`tests/round_trip.rs`, `debug_like_derive`); the `core` profile, which prints `{:#?}`, is byte-identical to before on the sample and 200 generated definitions. Nothing in `term.rs` recurses on the call stack any more.
