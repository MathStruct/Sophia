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
