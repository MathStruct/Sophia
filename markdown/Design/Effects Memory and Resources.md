# Effects Memory and Resources

[[Start Here]] mentions annotations that prove "correctness or solve memory management". This note is about the part of the semantics that is hardest to unify across languages and easiest to forget: what a program *does* as opposed to what it *computes*.

`Project.toml` already lists `BorrowChecker` as a dependency, which suggests this is live territory rather than speculation.

## Effects as a row on the judgement

[[Core Calculus|SC]]'s typing judgement is $Gamma ⊢ t : A ! epsilon$ where `ε` is an **effect row** — an unordered set of effect constructors, with row polymorphism so that generic code can be effect-generic:

$ epsilon ::= ∅ | l, epsilon | rho $

where `ρ` is a row variable. The effect labels that matter for this project:

| Label | Meaning |
| --- | --- |
| `read(r)` / `write(r)` | reads or writes region `r` |
| `alloc(r)` | allocates in region `r` (GC heap, arena, stack) |
| `free(r)` | deallocates — present in C++/Rust frontends, absent in Julia |
| `throw(E)` | may raise `E` |
| `div` | may not terminate |
| `io` | observable interaction with the world |
| `unsafe` | escapes the model entirely (inline asm, `unsafe` blocks, `ccall`) |
| `nondet` | result depends on scheduling, address layout, iteration order |

Effects are hashed into the identity of a `Decl`, because a pure function and an impure function with the same body-shape are not the same function.

## Why this is load-bearing for equivalence

Two functions cannot be equivalent if they differ observably, and effects *are* the observations. Practically:

- Purity is what licenses the most valuable rewrites (CSE, reordering, memoisation, parallelisation). An effect row is the certificate that licenses them.
- Cross-language claims almost always need an effect-modulo. Julia's allocator behaviour differs from C++'s; a claim of equivalence "modulo `alloc`" is honest, a claim of plain equality is false. See the `modulo` machinery in [[Equivalence and Witnesses]].
- `nondet` is the one that bites: hash iteration order, pointer-value-dependent behaviour, and floating-point reductions under parallel schedules are all sources of it, and none of them are visible in the syntax.

## Memory: three incompatible models under one roof

| Language | Model | What SC must record |
| --- | --- | --- |
| Julia | tracing GC, everything heap-ish, escape analysis may stack-allocate | region = GC heap; no `free`; finalisers are `io`-ish |
| C++ | manual, RAII, arbitrary aliasing, UB on misuse | explicit regions, explicit `free`, no aliasing guarantees unless `restrict` |
| Rust | affine ownership, borrows with lifetimes, `&mut` implies noalias | regions + borrow forms + lifetime constraints |
| Lean | GC with reference counting and functional-update-in-place | region = RC heap; uniqueness matters for perf, not semantics |

The unifying device is **regions plus a borrow discipline**: `ptr(r)`, `ref(r, A)`, `mutref(r, A)` in [[Core Calculus|SC]], with region variables quantified at function boundaries. This subsumes all four:

- Rust's lifetimes *are* region variables; the mapping is nearly mechanical.
- C++ maps in with a single ambient unsafe region unless the frontend can prove better — so C++ code starts life with an `unsafe` effect and gets refined upward by analysis or annotation.
- Julia maps in with one GC region and a `noalias` obligation that is *false in general* (two `Array`s can alias via `view`/`reshape`/`unsafe_wrap`), so the frontend must be conservative.
- Lean maps in with an RC region where uniqueness is an optimisation hint.

The important consequence: **going from Julia to Rust is refinement, not equivalence.** Rust's model forbids programs Julia allows. A `REFINES` edge is the right relation, not `EQUIV`. Conflating them is how a system like this would produce a miscompiled aliasing bug.

## Aliasing is the hardest part

Nearly every cross-language memory bug reduces to aliasing assumptions:

- LLVM's `noalias` on a parameter is a *promise*; violating it is UB, and the miscompilation appears far from the cause.
- Rust encodes `noalias` for `&mut` — and the exact rules (Stacked Borrows / Tree Borrows) are still research, which is a sobering data point for anyone hoping to formalise Julia's.
- Julia has no aliasing model at all beyond "arrays may alias"; `@inbounds`, `@simd` and `@fastmath` are unchecked promises with no formal statement.

So: a Sophia frontend for Julia can only emit aliasing facts it can prove or that the user has asserted, and every such assertion must be a first-class, attributable [[Equivalence and Witnesses|witness]] node rather than a flag on a line of code. That is arguably an improvement over the status quo, where `@inbounds` is an unattributable promise buried in a source file.

## Effects, algebraically

[[Unison Abilities]] and the algebraic-effects literature (Koka, Eff, OCaml 5) give the right shape: an effect is an *interface* of operations, and a handler is an interpretation. This matters here because it turns "does this Julia mutation mean the same thing as this C++ pointer write" into a question about handlers rather than about syntax, and because handlers compose. See [[Effect System]].

Categorically, effects are the Kleisli category of a monad (or, more precisely for multiple interacting effects, of a graded/parameterised monad); pure functions are the base category. That is why the [[Functor|functor]] story in [[Multi-AST Layering]] must be stated for the *effectful* category, not the pure one.

## Open questions

- Is a full region-and-borrow system in the core calculus worth its complexity, or should effects be a coarse annotation layer with the fine-grained story deferred? (Leaning: start coarse, `pure`/`alloc`/`mut`/`io`/`unsafe`, and refine.)
- Can Julia's escape analysis be reused to infer regions, or must they be inferred from scratch?
- How are effect rows for `ccall`/`unsafe` handled — a universal bottom effect that poisons everything, or an explicit trusted annotation?

## Related

- [[Effect System]]
- [[Linear and Affine Types]]
- [[Cross-Language Semantic Hazards]]
- [[Equivalence and Witnesses]]
- [[Unison Abilities]]
- [[State of the Art - Formal Semantics of Real Languages]]
- [[effects]] — the Rust module that would implement this
