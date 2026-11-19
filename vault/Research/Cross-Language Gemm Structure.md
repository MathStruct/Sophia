#experiment #measurement #rust #julia

> **E-2 (B-023, 2026-11-19): do Rust `matrixmultiply` and Julia's matmul share structure a query can find without names?** No, beyond a trivial shape. **In the store**, the comparison cannot be made at all: none of the 18 `matrixmultiply` functions that contain a loop has an id, because the shared normaliser (WP-66 v8) refuses Rust `for`, `match` and closures, so its driver, packing, macro-kernel and micro-kernels are all missing (0 of 11 labelled definitions). **On the source**, every definition counted, the only name-free shape both share is "a loop nest of depth ≥ 2 over a callee that loops", which finds the blocking driver in both (Octavian 3 of 3, matrixmultiply 1 of 1) but also tests and splitters; packing and kernels fall into one class with every other loop nest, and matrixmultiply's micro-kernel loops are macro calls (`loop4!`) and its macro-kernel's loops iterator closures, which no syntax-level feature sees. Structural reuse across languages (SR-73) needs L4 (macros expanded, iterators lowered) and data-flow facts, as the experiment's failure clause foresaw. Julia's own generic matmul (LinearAlgebra 1.13) has no blocking or packing to compare: it is a plain loop nest; Octavian.jl is the BLIS-shaped counterpart. Found on the way: Sophia explained these Rust refusals only as `UnsupportedSyntax` (fixed); the refusals reported to Moonkale.
>
> Sources: Goto and van de Geijn, "Anatomy of High-Performance Matrix Multiplication" (2008), and Van Zee and van de Geijn, "BLIS" (2015), for the roles; [[Semantic Boundary Experiments]] §E-2; [[Rust Overview]]; [[Languages]]. code: `crates/sophia-ingest/examples/gemm_structure.rs` ([[gemm_structure]]), `examples/gemm_structure.tsv` (labels), `src/code.rs` (`REFUSED`), `tests/code.rs` (`refusals_name_the_refused_rust_construct`).

## Corpora

| corpus | version | files | definitions found per file | stored | refused | no id in module mode |
|---|---|---|---|---|---|---|
| matrixmultiply (Rust) | 0.3.10, crates.io | 23 | 516 | 419 | 97 | — |
| Octavian.jl | `83f70f1b` (2025-10-21), with its two `ext/` files it includes | 14 | 214 | 189 | 13 | 12 |
| LinearAlgebra `matmul.jl` | Julia 1.13.1 stdlib | 1 | 185 | 166 | 1 | 18 |

"Found per file" is `code::definitions` on each file; Julia module mode does not identify some of them (mostly assignments it does not count as definitions, and the module itself), and they are no candidates in the store.

Rust is ingested per file and Julia in module mode, as `sophia ingest-code` does. Octavian's 6 "syntax error" refusals are tree-sitter-julia failures in `complex_matmul.jl` (an `@eval` loop over methods using operators `+ᶻ`, `-ᶻ`); not followed further.

## Roles (the oracle)

Labelled by hand before any predicate ran (`gemm_structure.tsv`), after the BLIS decomposition:

| role | matrixmultiply | Octavian | LinearAlgebra |
|---|---|---|---|
| driver (nc, kc, mc blocking) | `gemm_loop` | `matmul_st_pack_A_and_B!`, `matmul_st_only_pack_A!`, `sync_mul!` | none |
| pack | `pack_impl`, `pack_complex` | `unsafe_copyto_turbo!`, `packamul!` (packs A while multiplying) | none |
| macro-kernel (nr, mr tiles) | `gemm_packed` | `loopmul!`, `ploopmul!`, `packamul!`, `packaloopmul!`, `inlineloopmul!` | none |
| micro-kernel | `kernel_fallback_impl`, `kernel_x86_avx`, `kernel_target_neon` (s and d), `kernel_fallback_impl_complex` | none in source: LoopVectorization generates it from `@turbo` | none |
| naive loop nest | none | none | `_generic_matmatmul_nonadjtrans!`, `_generic_matmatmul_generic!`, `_generic_matmatmul_adjtrans!` |

## Result 1: Rust loops get no id

Functions (Rust `fn` items, Julia `function … end` forms; Julia short forms `f(x) = …` are not counted, since their kind, `assignment`, also covers constants) by whether they contain a loop, and their status (`definitions.tsv`, run of 2026-11-19; the reason is the first refused construct in source order):

| corpus | with a loop: stored / all | without: stored / all |
|---|---|---|
| matrixmultiply | **0 / 18** (first refused construct: 13 `for_expression`, 3 a `use` inside the body, 1 `closure_expression`, 1 `match_expression`) | 210 / 220 |
| Octavian | 31 / 38 (6 syntax errors, 1 refused) | 58 / 60 |
| LinearAlgebra matmul | 9 / 9 | 61 / 61 |

Every labelled Rust definition is among the refused: `gemm_loop` (`for`), `gemm_packed` (closure), `pack_impl` and `pack_complex` (`for`), the micro-kernels (an inner `macro_rules!` or `use`, or a macro definition themselves). So the in-store predicates match nothing in matrixmultiply, and the cross-language comparison cannot be run on the store. The normaliser's list is in Moonkale's `normalize.rs` (v8, `06fd112c`): Rust `for_expression`, `match_expression`, `closure_expression`, `let`-`else` and `while let` are refused outright, while Julia loops are scopes since v4. **Reported to Moonkale** (handoff of 2026-11-19).

[[Languages]] claimed Rust at L2 "yes"; it is L2 only for loop-free, match-free, closure-free definitions, and the matrix now says so.

## Result 2: the shared shape is trivial

The two predicates, on the source (all definitions, calls by name) and in the store (facts on stored definitions, calls as `refs` or `uses`+`member`; query time on the 774-definition store: 7.2 ms and 2.1 ms):

| predicate | corpus | source: matched / labelled among them / recall | store: matched / labelled / recall |
|---|---|---|---|
| nest-over-nest (driver) | Octavian | 4 / 3 / 3 of 3 | 4 / 3 / 3 of 3 |
| | matrixmultiply | 3 / 1 / 1 of 1 | 0 / 0 / 0 of 1 |
| | LinearAlgebra | 0 | 0 |
| leaf-nest (pack, kernels, naive) | Octavian | 25 / 5 / 5 of 6 | 19 / 5 / 5 of 6 |
| | matrixmultiply | 6 / 2 / 2 of 10 | 0 / 0 / 0 of 10 |
| | LinearAlgebra | 6 / 2 / 2 of 3 | 6 / 2 / 2 of 3 |

Where the store holds a definition, the store and the source agree exactly (Octavian's nest-over-nest; LinearAlgebra); the 6 fewer leaf nests in Octavian's store are its 6 syntax errors. So the query language and the facts express these predicates faithfully; what fails is what they can say.

**What the predicates find.**
- **The driver shape is found in both languages**, with no names: Octavian's three drivers plus `matmulsplitn!` (it splits N across threads and calls the kernels: a driver in all but name); matrixmultiply's `gemm_loop` plus two tests (`test_a_kernel`, `test_complex_packed_kernel`, loop nests that call kernels). LinearAlgebra has none, correctly: its matmul does not block.
- **Leaf nests do not separate roles.** Octavian's 20 unlabelled leaf nests are the complex and dual-number `_matmul!`/`_matmul_serial!` methods (`complex_matmul.jl`, `ext/`): genuine `@tturbo` multiply loops, kernels in all but label. Packing (`unsafe_copyto_turbo!`) and multiplying (`loopmul!`) look the same; LinearAlgebra's leaf nests include `generic_syrk!`, `copytri!` and the matrix-vector kernels. Telling a copy loop from a multiply-accumulate loop needs data flow, which no stored fact has.
- **The shapes do not line up in depth.** Octavian's driver nests 2 to 4 deep and its kernels 3 (`@turbo for n, m` + `for k`); matrixmultiply's driver is 2 deep (`l5`, `l4`; `l3` is a `range_chunk(…).for_each(closure)`), its macro-kernel 1 (`l1`; `l2` is again a closure), its micro-kernel 0, because its three loops are `unroll_by!`, `loop8!` and `loop4!`, whose expansion is a `for` loop or an unrolled `let` sequence depending on `cfg`. The 5-loop BLIS nest exists in both, but in Rust only after macro expansion and iterator inlining.

## What it teaches (SR-73)

Cross-language structural reuse is **not reachable at L3** on these two implementations:
1. at L2 the Rust half is missing (normaliser coverage, Moonkale's to fix);
2. with every definition in hand, syntax-level shape (loop depth, call reachability) recovers only "a nest over a nest", and recovers it with tests and splitters;
3. the corresponding parts are written with different means (Julia `@turbo` loops; Rust macros, closures, iterator adapters), so a matching shape exists only in a lowered form: macros expanded (`cargo expand`, Julia's `macroexpand`), closures and iterator adapters inlined, plus data-flow facts (reads, writes, multiply-accumulate) to tell packing from kernels.

This is L4 ([[Languages]]): a lowered IR per language with common loop and memory-access facts. The next step, if pursued, is a small one: take the lowered loop nests from each language's own tool (rustc MIR or LLVM IR for Rust, Julia's `code_typed`/LLVM for Julia) for the labelled definitions and compare their loop trees and access patterns, before any ingest work (SR-126). A proposal, not done: backlog B-086 (*proposed*, waiting on Daniel).

## Not checked

- Rust name resolution is by spelling; calls to `kernel` reach every `kernel` impl. With rustc's resolution the driver predicate could only lose matches, not gain them (the over-approximation adds edges).
- The labels are one person's reading (Claude's) of the two code bases; `packaloopmul!` is labelled a macro-kernel although it only combines `packamul!` and `loopmul!` (depth 0), which costs Octavian's leaf-nest recall one.
- Octavian's 6 syntax errors were not reported to tree-sitter-julia.
- Two ways the store and the source could differ that these corpora do not trigger (Mistral's review, F4 and F5): a call between members of one recursive component is in no base relation (`refs` holds only edges out of the component), and a Julia file included into two modules would be read twice with one id; the store and the source agree on every stored definition here, and no Octavian file is included twice.
- Reviewed by Mistral: [[Review 2026-11-19 Mistral B-023 (external)]].
