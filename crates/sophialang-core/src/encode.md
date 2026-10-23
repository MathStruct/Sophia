#implementation

> The canonical encoding of sophialang definitions, its decoder, and the display data. Source: `encode.rs`.
>
> Sources: [[SPEC-003 Sophialang Core v0]] §3, §5; [[canonical]]. code: `encode.rs`

- **Encoding.** Every term constructor has a short tag (`v`, `r`, `m`, `n`, `c`, `l`, `a`, `p`, `e`, `x`, …). Binders encode only usage and implicitness. Holes are numbered in first-visit order. Drafts are written as `m` (same component) or `r` (an earlier component), as the normaliser decides through a callback.
- **Decoding** inverts the encoding exactly, except for names, which come back empty. It never panics on malformed input (a property test feeds it random bytes).
- **Display data** is the list of binder and constructor names in stream order. `apply_display` puts them back.

**Known wart** ([[SP-003 Display names of alpha-equivalent definitions]]): α-equivalent definitions share content, so the display data stored first wins.

**`called_refs(bytes)`** (2026-10-23): the definitions called directly (the `Ref` heads of applications), read from the canonical bytes with an explicit stack, without building terms. The decoder, `Term::walk` and dropping a `Term` each recurse once per nesting level; on generated definitions nested 700 levels deep that overflowed a 2 MiB thread stack in a debug build (Moonkale P-229, reproduced here), while `called_refs` runs in constant call depth. Tested to agree with the term walk on the sample, on all 99 254 definitions of B-15's 100 000-name store (240 011 calls, no disagreement), and on a 20 000-deep definition scanned on a 64 KiB stack.
