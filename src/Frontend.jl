"""
    Sophia.Frontend

The Julia frontend: Julia code -> Sophia Core terms.

Design notes: `Frontend.md`, `markdown/Design/Roadmap.md` (M3).
COMMENTS ONLY — nothing here is implemented.

This module can only be written in Julia. `Meta.lower`, `code_typed`,
`CodeInfo`, `Core.MethodInstance` and the method tables are not reachable from
outside a running Julia process, and any external "Julia parser" would be a
reimplementation that drifts. That constraint is the main reason this
repository has a Julia half at all.

# Which representation to ingest

Julia offers several, and the choice matters:

| stage      | access           | verdict |
|------------|------------------|---------|
| surface    | `Meta.parse`     | no — would mean reimplementing desugaring |
| macroexpanded | `macroexpand` | no — still sugar below it |
| **lowered**| `Meta.lower`, `@code_lowered` | **yes, primary** |
| typed      | `code_typed`     | yes, but SEPARATELY — see below |
| LLVM       | `@code_llvm`     | no — too late, semantics already gone |

Lowered IR is the last representation that is still generic (not specialised to
one argument-type tuple) and is produced by Julia itself rather than by a
reimplementation. See `markdown/Wiki/Julia/Julia Lowered IR.md`.

Typed IR is ingested too, but as a DIFFERENT kind of node: one per
specialisation, with its dispatch decisions recorded. It is not the definition;
it is a compiled view of the definition under a world age.

# Intended API

    elaborate(m::Method)                    -> (Term, Type, EffectRow)
    elaborate_specialization(mi::Core.MethodInstance)
                                            -> (Term, Vector{DispatchFact})
    ingest(mod::Module; subset=default_subset) -> Vector{Hash}

# What elaboration has to make explicit

Everything Julia leaves implicit, because the hash is taken on the result:

  * **Dispatch.** Which method a call site resolves to is a fact about the
    method table, not about the syntax. Record it as a `DispatchFact` node so
    invalidation can later be computed exactly
    (`markdown/Design/Content-Addressed Precompilation.md`).
  * **Promotion and conversion.** `1 + 1.0` inserts a `promote`; that becomes
    an explicit `Convert` node, never an implicit coercion.
  * **Overflow.** Julia's `+` on `Int64` WRAPS. Emit `Overflow.Wrap`. Getting
    this wrong is how a false Julia/C++ equivalence gets into the store.
  * **Bounds checks.** Present by default; `@inbounds` removes them. An
    `@inbounds` is an unattributable promise today; here it must become an
    explicit assumption node that someone signed for
    (`markdown/Design/Effects Memory and Resources.md`).
  * **Indexing and layout.** 1-based, column-major. Elaborate to explicit
    linear-index arithmetic, or no comparison with C code is meaningful.
  * **Effects.** Allocation, mutation, I/O, `ccall` (which is `Unsafe` and
    poisons its callers), possible non-termination.
  * **`@fastmath`, `@simd`, `@inbounds`** are all unchecked promises. Each
    becomes an explicit attribute or an assumption node.

# The subset problem

Julia will not be covered. Things that are genuinely hard or out of scope:

  * `eval` and generated functions — code that does not exist until it runs
  * `ccall` into arbitrary C — model as `Unsafe`, do not pretend
  * tasks, `@async`, `Threads` — Julia has no formal memory model
    (`markdown/State of the Art/State of the Art - Formal Semantics of Real Languages.md`)
  * `unsafe_wrap`, `pointer`, `reinterpret` — aliasing becomes unknowable
  * finalizers, `atexit`, global mutable state, world-age-crossing `invokelatest`

The subset boundary must be EXPLICIT and ingestion outside it must FAIL. A
guessed meaning is worse than no ingestion, because it will be hashed, stored,
trusted and compiled.

# Validation

There is no formal Julia semantics to check the elaboration against, so
differential testing is the only option: run the original function, evaluate
the extracted core term, compare on generated inputs. This suite IS the
specification of what the ingested subset means, and it should be built before
the frontend grows.
"""
module Frontend
end
