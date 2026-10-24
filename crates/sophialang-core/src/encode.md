#implementation

> The canonical encoding of sophialang definitions, its decoder, and the display data. Source: `encode.rs`.
>
> Sources: [[SPEC-003 Sophialang Core v0]] §3, §5; [[canonical]]. code: `encode.rs`

- **Encoding.** Every term constructor has a short tag (`v`, `r`, `m`, `n`, `c`, `l`, `a`, `p`, `e`, `x`, …). Binders encode only usage and implicitness. Holes are numbered in first-visit order. Drafts are written as `m` (same component) or `r` (an earlier component), as the normaliser decides through a callback.
- **Decoding** inverts the encoding exactly, except for names, which come back empty. It never panics on malformed input (a property test feeds it random bytes).
- **Display data** is the list of binder and constructor names in stream order. `apply_display` puts them back.

**Known wart** ([[SP-003 Display names of alpha-equivalent definitions]]): α-equivalent definitions share content, so the display data stored first wins.

**`called_refs(bytes)`** (2026-10-23): the definitions called directly (the `Ref` heads of applications), read from the canonical bytes with an explicit stack, without building terms. The decoder, `Term::walk` and dropping a `Term` each recurse once per nesting level; on generated definitions nested 700 levels deep that overflowed a 2 MiB thread stack in a debug build (Moonkale P-229, reproduced here), while `called_refs` runs in constant call depth. Tested to agree with the term walk on the sample, on all 99 254 definitions of B-15's 100 000-name store (240 011 calls, no disagreement), and on a 20 000-deep definition scanned on a 64 KiB stack.

**Encoding and decoding at any depth (2026-10-24).** `Encoder::term`, the decoder's `term` and `for_each_name` go through `deep::grow` at each level. (Superseded the same day: every pass now uses explicit stacks and `deep::grow` is gone; see below.)

**Explicit stacks (2026-10-24, SP-015).** The encoder (`Encoder::term`, with work items for wrappers, cases and closing containers), the decoder (`term`: one `Frame` per composite term being read, with its sub-reader and the subterms so far) and `for_each_name` use explicit stacks instead of recursion, so they work at any depth on every target, wasm included. The encoder writes fields in the same pre-order as before: the identity tests, the golden ids and 300 generated definitions saved back unchanged confirm that no id changed.
