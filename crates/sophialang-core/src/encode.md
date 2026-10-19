#implementation

> The canonical encoding of sophialang definitions, its decoder, and the display data. Source: `encode.rs`.
>
> Sources: [[SPEC-003 Sophialang Core v0]] §3, §5; [[canonical]]. code: `encode.rs`

- **Encoding.** Every term constructor has a short tag (`v`, `r`, `m`, `n`, `c`, `l`, `a`, `p`, `e`, `x`, …). Binders encode only usage and implicitness. Holes are numbered in first-visit order. Drafts are written as `m` (same component) or `r` (an earlier component), as the normaliser decides through a callback.
- **Decoding** inverts the encoding exactly, except for names, which come back empty. It never panics on malformed input (a property test feeds it random bytes).
- **Display data** is the list of binder and constructor names in stream order. `apply_display` puts them back.

**Known wart** ([[SP-003 Display names of alpha-equivalent definitions]]): α-equivalent definitions share content, so the display data stored first wins.
