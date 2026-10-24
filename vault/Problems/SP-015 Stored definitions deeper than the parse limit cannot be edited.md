#open-problem

> **Status: open. Revised by Daniel on 2026-10-24 (~10:15, relayed by the Moonkale agent): no limit on expressiveness, so no store-time depth limit that refuses records. This replaces that morning's approval of options 2 and 3 below.** Found 2026-10-21. Found while checking the Rust-profile fix against `sophia gen` stores. Related: [[SPEC-005 Rendering Profiles]] §5 (rule 4: text in an editable profile parses back), [[lexer]] (`Limits`), SR-50.

**Problem.** The parser refuses text nested deeper than `Limits::max_depth` (128, Moonkale WP-66's limit, SR-50), counted as nested `expr` calls: every `let` value and body, lambda body, argument and parenthesis adds one. Nothing bounds the depth of a term that enters the store by another path (`normalise_batch`, `sophia gen`, a future ingest). Such a definition prints, but its text does not parse back, so Moonkale can show it and not save it, even unchanged.

**Measured** (2026-10-21, release build): print each definition in the `julia` profile and save the text unchanged with `sophia edit`.
- `sophia gen 50` (seed 42): 7 of 50 refused with `nesting deeper than 128`, the same 7 in the `rust` profile; the build before 2026-10-21 refuses them too (checked on `gen.m0000036`).
- B-15's `gen1000` store, its first 300 definitions: 22 refused.
- B-15 edited `m0000001`, which parses, so its numbers are unaffected.

**Also a stack problem** (2026-10-23): the same definitions nest up to 700 levels in their encoding, and every recursive pass over a term (decode, walk, print, drop) needs stack per level. Moonkale's `all` overflowed a 2 MiB worker in a debug build (P-229); browsing no longer decodes terms ([[sophia_interface]]), but printing still does. Bounding depth on entry (option 2) would bound every recursive pass at once.

**Options considered on 2026-10-21** (superseded, kept for the record): 1. raise the parse limit; 2. bound depth on entry (refuse or mark); 3. a realistic generator. The proposal was 2 and 3.

## Revision (2026-10-24)

Daniel: *"My goal was to under no circumstances limit its expressiveness, especially when doing the actual compiler. I tend towards solution 4."* Deep terms are legitimate: long operator chains (large polynomials, symbolic Jacobians from ModelingToolkit), generated MLIR or LLVM, e-graph extraction, macro expansions, Metamath and Lean proof terms, deep data literals.

The options put to him by the Moonkale agent:
1. **Depth costs memory, never stack:** decoding, printing, parsing, elaborating, walking and dropping terms must not overflow at any depth. The parser's depth limit (128) becomes a resource budget on total size (bytes, nodes), high and documented.
2. **Associative operators stored n-ary** (`+(a, b, c, …)` as one node). Changes the term encoding and therefore ids: a migration (SPEC-006 §6).
3. **Mark, do not refuse:** a definition whose text does not parse back in a profile is stored and flagged "not editable in this profile". A realistic generator stays useful for benchmark data only, not as the fix.
4. **Depth is the UI's problem:** Moonkale folds deep regions ("… 650 levels", expand on demand). Nothing in the store has to be shallow for Moonkale's sake.

Daniel leans to 4, which holds only if 1 holds on Sophia's side.

## Plan (Claude, 2026-10-24; not started)

1. **First: option 1.** The recursive passes are many (decode, encode, `Term::walk` and `map`, `occurs`, the printer, the parser, the elaborator, resolution, dropping `Term` and the syntax tree). Proposed in two steps:
   - **a. Depth-safe at once, little code:** an explicit-stack `Drop` for `Term` and the syntax tree (dropping a deep `Box` chain recurses), and on-demand stack growth at each recursive step of the other passes (the `stacker` crate's `maybe_grow`: when the remaining stack is short, it continues on a new heap-allocated segment). Then the parser's depth limit is replaced by a node budget. Tests at depth 100 000 for every pass, on a small thread stack.
   - **b. Later, where it pays:** rewrite hot passes with explicit stacks (as `encode::called_refs` already is), above all for wasm. There `stacker` gives no protection: psm's own README lists wasm as unsupported ("the callstack is not even in an address space and does not appear to be manipulatable"). stacker 0.1.25 does compile for `wasm32` and can at most move the shadow stack in linear memory, but deep recursion exhausts the engine's call stack, which a program can neither see nor move (in a browser: `RangeError: Maximum call stack size exceeded`). Not tested here.
   - **Replaceable for wasm** (Daniel, 2026-10-24): every recursive step calls one function of Sophia's own (`sophia_hash::deep::grow`), never `stacker` directly. On native targets it is `stacker::maybe_grow`; on `wasm32` it is a marked placeholder that runs the closure as is, to be replaced by whatever wasm solution is chosen, without touching the callers.
   Whether Moonkale's own WP-66 guard (128 in `moonkale-content-id`) can become a node budget too is for Moonkale; Sophia's parser no longer needs a depth guard after 1a.
2. **Then option 3, narrowed.** After 1, every editable profile's text parses back at any depth, so "not editable" remains only where a profile cannot express something (`python` is display-only, `core` is a debug view). Proposed as a fact computed on demand (print, parse, compare the id), not a check on every write.
3. **Option 2 separately, later.** It reads better and removes the commonest depth, but it changes ids, so it is a migration with its own proposal and Daniel's decision; it is not needed for safety after 1.
4. **Generator:** add a realistic-depth shape for benchmarks, keep the deep shape as a stress test.
