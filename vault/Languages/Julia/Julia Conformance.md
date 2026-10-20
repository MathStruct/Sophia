#measurement #julia

> **Which facts about Julia code Sophia knows, which it approximates, and which it leaves open, measured against Julia itself.** The conformance suite the external review asked for (SR-129), started 2026-10-20 with its first two parts: generic-function ownership on a fixture and on LinearAlgebra.
>
> Sources: `crates/sophia-ingest/tests/generics.rs` and `tests/julia-semantics/generics/` (the fixture); `tests/julia-semantics/methods.jl` (Julia's ground truth, [[methods]]); `examples/julia_conformance.rs` ([[julia_conformance]]); [[Julia Name Resolution]].

## Principle

For each Julia construct, compare **what Julia actually does** (by running Julia and asking it) with **what Sophia records**. Each comparison yields one of:
- **knows**: Sophia's fact equals Julia's;
- **approximates**: Sophia's fact differs in a known, bounded way, and the page says how;
- **leaves open**: Sophia records "unknown" (a refusal, a dynamic include, an unplaced method);
- **wrong**: Sophia states something false. Every "wrong" is a bug to fix or report.

The suite is not a Julia implementation. It is a ledger of which claims Sophia may make.

## Part 1: generic ownership on a fixture (in CI)

`tests/generics.rs` on the package `Gen`, `src/Gen.jl` and `src/sub.jl`:

| Case | Julia's meaning | Sophia | |
|---|---|---|---|
| `f` in `module A` and in `module B` | two functions, `Gen.A.f`, `Gen.B.f` | `jl.Gen.A.f` (4 methods), `jl.Gen.B.f` (1) | knows |
| `f` in `Gen` itself | a third function, `Gen.f` | `jl.Gen.f` (2) | knows |
| `A.f(p::Point) = …` in `Gen` | a method of `Gen.A.f` | member of `jl.Gen.A.f` | knows |
| `import ..A: f` then `f(s::String) = …` in `Gen.C` | a method of `Gen.A.f` | member of `jl.Gen.A.f` | knows |
| `import Base: show`; `show(io, p) = …` | a method of `Base.show` | member of `jl.Base.show`; no `jl.Gen.show` | knows |
| `import Base.length`; `length(p) = …` | a method of `Base.length` | member of `jl.Base.length` | knows |
| `Base.:+(p, q) = …` | a method of `Base.:+` | member of ``jl.Base.`+` `` | knows |
| `Point() = …`, `Point(a, b) = …` | constructors of `Gen.Point` | `jl.Gen.Point.new` (2) | knows |
| `use(p) = f(p) + f(p, p)` | calls `Gen.f` | `nominal_refs = [jl.Gen.f]` | knows |

## Part 2: generic ownership on LinearAlgebra (Julia 1.13.1)

Julia lists 4 933 methods from LinearAlgebra's source files. Sophia stores 2 920 methods (normaliser v7; on v6 it was 2 939, see the `check` row).

| Outcome | Methods | What |
|---|---|---|
| **same owner** | **2 777 (95.1 %)** | knows |
| different owner: re-exported binding | 86 | approximates: `Base.Matrix` is `Core.Array`, `Base.cbrt` is `Base.Math.cbrt`, `Base.uppercase` is `Base.Unicode.uppercase`, `Base.CartesianIndex` is `Base.IteratorsMD.CartesianIndex` ([[SP-013 Julia bindings of modules Sophia has not ingested]]) |
| different owner: spurious unit in a macro call | 0 (11 on v6) | was **wrong**: `@deprecate f(…; check::Bool = true) …` gave a "method" named `check`. Reported to Moonkale, fixed in v7 the same day (nothing inside a call is a module-level unit) |
| no Julia method at that line | 57 | approximates: `@static if Sys.isapple()` code (not compiled on Linux), `function f end` (no method), multi-line signatures |
| Julia methods at lines where Sophia stores none | 1 059 | leaves open: `@eval` loops, macros, default constructors, refused definitions |

**Before the (module, name) fix** (`--by-name` reproduces the old grouping): **1 578 of 2 939 (53.7 %)** on v6, 1 578 of 2 920 (54.0 %) on v7. The 1 296 disagreements (v6) were 898 methods of Base functions filed as LinearAlgebra's (662 by `import`, 236 by qualified heads such as `Base.to_indices`, stored as a function named `` `Base.to_indices` ``), 234 methods of BLAS and LAPACK functions filed under LinearAlgebra, 38 of `Base.Broadcast`, and the alias cases that remain.

## Part 3 onwards: planned

Each part is one fixture package (or a real one) plus a Julia script that prints the ground truth.

| Part | Construct | Julia's ground truth | Sophia's claim |
|---|---|---|---|
| 3 | **dispatch**: which method a call reaches | `which(f, Tuple{…})` for chosen signatures | none yet; record Julia's answer as a dispatch fact ([[Julia Multiple Dispatch and World Age]]) |
| 4 | **world change**: adding a method or an import | `methods(f)` before and after; `Base.get_world_counter()` | which facts become stale (SR-85) |
| 5 | **macros** | `macroexpand`, `Meta.lower` | expansions as transformation records ([[Julia Macros and Generated Functions]]) |
| 6 | **closures and captured variables** | `fieldnames(typeof(closure))` | references inside a unit (v4 scoping) |
| 7 | **mutable globals** (`const` vs non-`const`, `global x`) | binding partition / world semantics in Julia ≥ 1.12 | whether a global reference is content or state |
| 8 | **parser agreement** | JuliaSyntax.jl's tree vs tree-sitter's | refusals and silent disagreements |
| 9 | **include order** | `pathof`, file/line of each definition | include graph (`module::discover`) |

## How to run

```sh
cargo test -p sophia-ingest --test generics
julia crates/sophia-ingest/tests/julia-semantics/methods.jl LinearAlgebra > truth.tsv
cargo run --release -p sophia-ingest --example julia_conformance -- <stdlib>/LinearAlgebra truth.tsv
```
