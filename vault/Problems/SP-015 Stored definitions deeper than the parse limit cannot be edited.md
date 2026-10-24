#open-problem

> **Status: resolved for reading; open only for writing on wasm (2026-10-24).** Step 1a: every pass depth-safe on native targets, the parse depth limit gone. Step 1b: the read path uses explicit stacks on every target, and printing is linear. Revised by Daniel on 2026-10-24 (~10:15, relayed by the Moonkale agent): no limit on expressiveness, so no store-time depth limit that refuses records. This replaces that morning's approval of options 2 and 3 below.** Found 2026-10-21. Found while checking the Rust-profile fix against `sophia gen` stores. Related: [[SPEC-005 Rendering Profiles]] §5 (rule 4: text in an editable profile parses back), [[lexer]] (`Limits`), SR-50.

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

## Done (2026-10-24): step 1a

- `sophia_hash::deep::grow` ([[deep]]): `stacker::maybe_grow` on native targets; a marked placeholder on wasm32, to be replaced.
- Through it, at every level: encode, decode, `for_each_name`, `Term::walk` / `map` / `Clone` / `PartialEq` / `Hash` / `Debug`, the parser (`expr`, `atom`), the elaborator, the printer (`term_level`, `occurs`), `SExpr`'s `Clone` / `PartialEq` / `Debug` / `Drop`. `Term`'s `Drop` is iterative.
- The parser's depth limit (128) is replaced by a node budget (`Limits::max_nodes`, 2²⁰); `parse_text` no longer starts a 64 MiB thread.
- **Tests** (`crates/sophialang-syntax/tests/deep.rs`): sum, lambda and let chains 100 000 deep, on a 256 KiB thread: encode, decode, clone, eq, hash, Debug, walk, map; and print and parse back to the same id (sum in the julia profile, lambda in rust). 2.0 s release, 5.3 s debug. The old round-trip test that expected 300 nested parentheses to be refused now expects them to parse.
- **The measurement this problem started from:** B-15's `gen1000`, first 300 definitions printed and saved unchanged: 300 of 300 keep their id (22 failed before). All 1 000 print exactly as before in both profiles.
- **Moonkale's side:** `moonkale-content-id` at `09ce6d9c` has no depth limit either (heap continuations, budgets on size and work), reported by the Moonkale agent.

**Still open:** printing nested `let`s is superlinear (5.0 s for 16 000 levels; each level copies its children's text), step 1b; the wasm placeholder; option 3 (narrowed) for display-only profiles.

## Done (2026-10-24): step 1b and the wasm read path

Daniel asked for both open parts to be fixed before the `v0.1.0` tag.
- **Printing is linear:** an explicit-stack printer that assembles a tree of pieces ([[print]]). Let chain at 100 000 levels: 0.20 s (53 MB of text); before, 5.0 s at 16 000. Output byte-identical to before (1 000 generated and 18 sample definitions, four profiles).
- **The read path needs no stack growth:** encode, decode, display names, `walk`, `map`, `Clone`, `PartialEq`, `Hash`, printing, `binder_uses`, `occurs` and `Drop` use explicit stacks. In wasm, `tests/wasm_deep.rs` runs them on sum, lambda and let chains 100 000 deep: 3 of 3 under Node and in headless Firefox. Before, 10 000 levels overflowed Node's stack. No id changed (identity tests, golden ids, 300 generated definitions saved back unchanged).
- **Still recursive, through `deep::grow`:** the parser, the elaborator, `SExpr`'s traits and `Term`'s `Debug`, that is the write path and debugging. On native targets they work at any depth (`stacker`); on wasm the placeholder leaves them bounded by the engine's stack. Writes on wasm need a clock too and are not supported yet; making the parser and elaborator iterative is what remains if they are to be.
