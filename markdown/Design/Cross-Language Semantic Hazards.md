# Cross-Language Semantic Hazards

A concrete catalogue of the ways "this Julia code does the same thing as this C++ code" turns out to be false. Every row here is a place where a naive [[Equivalence and Witnesses|equivalence]] claim is wrong, and therefore a place where [[Core Calculus|SC]] must be explicit rather than generic.

This list is the practical justification for why SC carries machine-level attributes instead of an abstract `Int`.

## Integers

| Hazard | Julia | C++ | Rust |
| --- | --- | --- | --- |
| Signed overflow | wraps (two's complement) | **undefined behaviour** | panics in debug, wraps in release |
| Unsigned overflow | wraps | wraps | panics in debug, wraps in release |
| Default integer width | `Int` = 64-bit on 64-bit platforms | `int` = 32-bit on almost everything | explicit, `i32` default for literals |
| Shift ≥ bit width | defined (yields 0) | **UB** | panics/masks depending on operator |
| Signed division by `typemin / -1` | throws `DivideError` | **UB** | panics |
| `%` sign | follows dividend (`rem`); `mod` follows divisor | follows dividend | follows dividend |
| Integer→smaller conversion | `Int8(300)` throws `InexactError` | silent truncation | `as` truncates, `try_into` errors |

The C++ UB column is the important one: LLVM exploits signed-overflow UB to optimise, so a "faithful" translation of Julia's wrapping `+` into C++'s `+` is *actively unsound*. The Julia operation corresponds to `add.wrap`; the C++ operation corresponds to `add.nsw`. **These are different primitives in SC and must never be given the same hash.**

## Floating point

- IEEE-754 basic operations are deterministic, so `f64` addition genuinely is the same everywhere — but almost nothing in practice sticks to basic operations.
- **Contraction**: `a*b + c` may fuse into an FMA. C and C++ allow this by default (`FP_CONTRACT` on); Julia does not fuse unless asked (`muladd`, `@fastmath`). Different results.
- **Reassociation**: `@fastmath` / `-ffast-math` / `-Ofast` license reassociation, which is not a refinement — it changes results.
- **x87 excess precision**: on 32-bit x86, intermediate results may be computed at 80-bit. Still a live hazard in cross-platform claims.
- **Transcendentals**: `sin`, `exp`, `pow` are *not* correctly rounded and differ between libm implementations, between Julia's openlibm and glibc, and across versions. Two implementations of the same formula are almost never bit-equal.
- **NaN payloads and signalling**, signed zero, and rounding-mode state are all observable.
- **Reduction order** under `@simd`/vectorisation changes results for non-associative operations.

Consequence: nearly every floating-point equivalence claim needs `modulo ⊇ {fp_assoc, fp_contract}` at minimum, or must be restricted to bit-exact basic operations.

## Arrays and indexing

| | Julia | C++ | Fortran |
| --- | --- | --- | --- |
| Base index | 1 | 0 | 1 |
| Multidim layout | column-major | row-major (`std::mdspan` configurable) | column-major |
| Bounds checking | on by default, `@inbounds` disables | none | optional |
| Slicing | copies by default, `view` for a slice | spans/iterators, never copies | sections copy |
| Arbitrary index offsets | `OffsetArrays` legal | no | declarable |

Index-base and layout differences make the *element access function* differ, so a naive structural equivalence of two loops is false even when the mathematical result agrees. Both must elaborate to explicit linear-index arithmetic in SC before any comparison is meaningful.

## Strings and text

- Julia `String`: immutable, UTF-8, indexed by *byte* with non-contiguous valid indices; iterating yields `Char` (a UTF-32 code point).
- C++ `std::string`: mutable byte sequence, no encoding guarantee, `char` may be signed.
- Rust `String`: immutable-by-borrow UTF-8, guaranteed valid, byte-indexed with panics on non-boundaries.
- Lengths mean different things (`length` vs `sizeof` vs `.len()`), and normalisation (NFC/NFD) is nobody's default.

## Control flow and errors

- **Exceptions**: Julia throws freely and unwinding is cheap-ish; C++ may be compiled `-fno-exceptions`; Rust distinguishes `panic` (unwinding or abort) from `Result`. Whether a function *can* throw is part of its type in C++ (`noexcept`) and in Sophia's effect row ([[Effects Memory and Resources]]), and invisible in Julia.
- **Evaluation order**: C++17 fixed some order guarantees; before that, argument evaluation order was unspecified. Julia is left-to-right. Programs with effectful arguments are genuinely different.
- **Short-circuiting** is consistent across these three, but operator overloading can change it in C++.
- **Divergence** is an observation. A tail-recursive Julia function that stack-overflows and a C++ loop that runs forever are not equivalent.

## Dispatch and generics

- Julia: [[Multiple Dispatch|multiple dispatch]] on runtime types, open world, method tables mutable at runtime ([[World Age]]). Which method runs is not determined statically in general.
- C++: templates are compile-time, monomorphised, with overload resolution + ADL + SFINAE; the specialisation set is closed at link time.
- Rust: traits, monomorphised or via vtables; coherence rules keep it closed.
- Lean: type classes, elaboration-time.

The open/closed-world distinction is a deep mismatch. A Julia definition's meaning can change when a package is loaded later; a C++ definition's cannot. Any cross-language claim about a Julia generic function is implicitly conditioned on a **world age**, which must therefore be part of the claim.

## Concurrency and memory models

- C++11 has a formal memory model (sequential consistency for data-race-free programs, plus explicit orderings); data races are UB.
- Rust has a memory model borrowed from C++11 plus the ownership rules that prevent most races statically.
- Julia has tasks, `@atomic` fields since 1.7, and no complete formal memory model.
- Any equivalence claim involving shared mutable state under concurrency is currently out of reach for all four languages and should be refused rather than approximated.

## ABI and layout

Struct layout, padding, bitfield ordering, calling conventions, name mangling, and vtable layout all differ and all matter the moment any code is actually shared at the machine level. The project's premise is to avoid FFI, but the *runtime* still has to pick one layout per type, and that choice must be recorded on the `Target` node ([[Graph Schema]]).

## Undefined behaviour as a category

The deepest structural mismatch is that C and C++ semantics contain **undefined behaviour**, which is not a value but a licence for the compiler to assume the case never happens. Julia, Rust-safe and Lean have no such construct. Mapping UB into SC has exactly three honest options:

1. `poison`/`undef` values, as LLVM does — accurate, complicates every proof.
2. A `⊥`/`unsafe` effect meaning "no guarantees past this point" — coarse, sound, loses optimisation power.
3. Refuse to ingest UB-capable operations without an accompanying assumption witness ("this index is in bounds"), which is then an obligation the graph records.

Option 3 is the one that fits this project's philosophy: turn every implicit UB assumption into an explicit, attributable node. It is also the most work, and it is why C++ ingestion is the hardest frontend rather than the most obvious one.

## Practical rule

Every `EQUIV` edge should be *rejected by default* and only admitted with an explicit `modulo` set covering the hazards above that are relevant to the terms involved. A checker that enumerates which hazards apply to a given pair of core terms — and demands the corresponding `modulo` tags or a witness discharging them — is one of the more valuable early deliverables of this project.

## Related

- [[Equivalence and Witnesses]]
- [[Effects Memory and Resources]]
- [[Core Calculus]]
- [[World Age]]
- [[Multiple Dispatch]]
- [[State of the Art - Formal Semantics of Real Languages]]
- [[Trusted Computing Base]]
