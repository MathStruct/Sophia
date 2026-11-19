#measurement #julia

> **Which facts about Julia code Sophia knows, which it approximates, and which it leaves open, measured against Julia itself.** The conformance suite the external review asked for (SR-129), started 2026-10-20 with its first two parts: generic-function ownership on a fixture and on LinearAlgebra. Part 3 (dispatch, 2026-11-18) found two kinds of wrong reference in the shared normaliser, reported to Moonkale. Part 4 (world change, 2026-11-18): a generic use survives a method added anywhere; a content reference to a function is stale when another package adds a method, and moves ids when the package does. Part 5 (macros, 2026-11-18): a macro around a definition is not part of the definition's id, so `@inline f`, `@generated f` and plain `f` share one (202 of 202 such definitions in LinearAlgebra); `@eval` loops are templates (347 definitions for 1 155 methods); what macros make by themselves is left open (97 methods). Part 6 (closures, 2026-11-18): Sophia's encoding binds every name in a closure as Julia does (captured, the closure's own, global: 273 of 276 names in LinearAlgebra, 3 left open, 0 wrong); found on the way: a call's keyword argument name is in no id (`f(; atol = x)` and `f(; rtol = x)` share one; 127 calls in LinearAlgebra). Part 7 (mutable globals, 2026-11-19): an untyped mutable global is no definition, so a use keeps only its spelling (state, right), and a constant is a `const` definition reached by content (right); wrong: a typed global `x::Int = 1` is taken for a function definition (an id, a content reference to a value that changes, a generic function in the store), and the argument of a callable object's method `(I::UniformScaling)(n)` is read as the constant `I` (1 in LinearAlgebra, which has no mutable global read in a method body). Part 8 (parser agreement, 2026-11-19): JuliaSyntax against the pinned tree-sitter parser per definition: LinearAlgebra 3 001 of 3 983 rows agree, 376 left open, 550 wrong, all of two kinds already reported: 394 typed assignments `x::T = v` (mostly typed default parameters) identified as functions that do not exist, 156 definitions whose call keywords are lost.
>
> Sources: `crates/sophia-ingest/tests/generics.rs` and `tests/julia-semantics/generics/` (the fixture); `tests/julia-semantics/methods.jl` (Julia's ground truth, [[methods]]); `examples/julia_conformance.rs` ([[julia_conformance]]); `tests/dispatch.rs`, `tests/julia-semantics/calls.jl` ([[calls]]), `src/conformance.rs` ([[conformance]]), `examples/julia_dispatch.rs` ([[julia_dispatch]]); `tests/world.rs` and `tests/julia-semantics/world/` (part 4); `tests/macros.rs`, `tests/julia-semantics/macros.jl` ([[macros]]) and `tests/julia-semantics/macros/`, `examples/julia_macros.rs` ([[julia_macros]]) (part 5); `tests/closures.rs`, `tests/julia-semantics/closures.jl` ([[closures]]) and `tests/julia-semantics/closures/`, `examples/julia_closures.rs` ([[julia_closures]]) (part 6); `tests/globals.rs`, `tests/julia-semantics/globals.jl` ([[globals]]) and `tests/julia-semantics/globals/`, `examples/julia_globals.rs` ([[julia_globals]]) (part 7); [[Julia Name Resolution]]; [[Julia Macros and Generated Functions]]. `tests/parse.rs`, `tests/julia-semantics/parse.jl` ([[parse]]), `tests/julia-semantics/parse/`, `examples/julia_parse.rs` ([[julia_parse]]) (part 8).

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

## Part 3: dispatch, which function and which method a use reaches (2026-11-18)

Sophia does not compute dispatch ([[Julia Multiple Dispatch and World Age]]). What it does claim, through the shared normaliser, is what a module-level name in a definition's body refers to:
- a **content reference** (in the id) when the name has exactly one definition in the user's module and is not imported there. For a function name this says more than "calls g": it says the use reaches **this one definition**;
- a **generic use** (`nominal_refs`, not in the id) when several methods bind the name in the module: the function (module, name), the method left open.

**Ground truth:** `tests/julia-semantics/calls.jl` ([[calls]]) prints, per method of the package, the global bindings its **lowered** body uses, each with Julia's function, its method count and which methods are in the package. The comparison is `sophia_ingest::conformance` ([[conformance]]), run by `examples/julia_dispatch.rs` ([[julia_dispatch]]) and on the fixture by `tests/dispatch.rs` (in CI, against the committed `dispatch/truth.tsv`).

**Fixture** `tests/julia-semantics/dispatch` (package `Disp`, 30 comparisons):

| Case | Julia | Sophia | |
|---|---|---|---|
| `call_solo(x) = solo(x)`, `solo` has one method; also in a multi-line `function` | `Disp.solo`, 1 method | content reference to it | knows |
| `pass_solo(xs) = map(solo, xs)` (a value, not a call) | the same | the same | knows |
| `defarg(x, y = 1)`; `call_defarg(x) = defarg(x)` | 2 methods, both from that definition | content reference to it | knows |
| `call_duo(x) = duo(x)`, two methods; also in a closure and a generator | `Disp.duo` | generic use of `Disp.duo` | knows the function; the method is open |
| `make(x) = Point(x)` | the type `Disp.Point` | content reference to the struct | knows |
| `call_length(v) = length(v)` after `import Base: length` | `Base.length` | nothing (imported names are never resolved) | leaves open |
| `call_qualified(x) = Inner.h(x)` | `Disp.Inner.h` | nothing for `h` | leaves open |
| `call_sum(v) = sum(v)` | `Base.sum` | nothing | leaves open |
| `both(x) = h(x) + Inner.h(x)`, with `h(x) = 5` in `Disp` | `Disp.h` and `Disp.Inner.h` | content reference to `Disp.h`; nothing for `Inner.h` | knows; leaves open (matched by binding, not by name) |
| `same_a(x) = 1; same_b(x) = solo(x)` on one line | one line for both methods | — | not compared: Julia's line cannot tell them apart |
| `shadow(duo) = duo(1)` (a parameter) | no global | no claim | knows (nothing to claim) |
| `Inner.call_h(x) = h(x)`; `Inner.h(x::Int) = 2` added from the parent module | `Disp.Inner.h`, **2 methods**; `call_h(1)` returns 2 | content reference to `h(x) = 1` | **wrong** |
| `call_lone(x) = lone(x)`; `lone(x::String)` added in `Ext` after `import ..Disp: lone` (another file) | `Disp.lone`, **2 methods** | content reference to `lone(x) = 0` | **wrong** |
| `for (T, duo) in ((:Int, :(:U)),) @eval function tagged(x::$T) … $duo … end end` | `$duo` is the symbol `:U` | generic use of `Disp.duo` | **wrong** |

**LinearAlgebra** (Julia 1.13.1, normaliser at the pin `06fd112c`; 24 995 comparisons):

| Outcome | Count | What |
|---|---|---|
| knows: the only definition | 846 | content reference; every method of Julia's function comes from that definition |
| knows: the function | 1 758 | generic use of the function Julia uses; the method is open |
| knows: the binding | 249 | content reference to a type, constant or module |
| wrong: one method of several | **0** | the fixture's case does not occur in LinearAlgebra |
| wrong: another function | **0** | |
| wrong: a spliced loop variable | **5** | `$uplo`, `$gemm`, `$gemmt` in `@eval` templates read as uses of `LinearAlgebra.uplo`, `BLAS.gemm`, `BLAS.gemmt` (`bidiag.jl:1489`, `special.jl:96`, `special.jl:104`, `blas.jl:1557`, `blas.jl:1635`) |
| approximates: not in the body | 913 | Sophia's reference is to a type named in the signature (`BlasFloat` 101, `UniformScaling` 95, `AbstractQ` 88, …) or `::typeof(f)`; the lowered body does not mention it, so this oracle cannot check it |
| leaves open: a function or type of this package | 10 370 | mostly Base's functions LinearAlgebra extends (`==`, `size`, `getindex`), which are imported and never resolved; also qualified names |
| leaves open: from outside | 10 854 | Base and Core (`+`, `iterate`, `getproperty`) |

Not compared: 180 lines of Julia methods that lie inside no Sophia definition (macro output, refused definitions), 3 lines holding several Sophia definitions, and 300 Sophia definitions in which Julia lists no use (a body without globals, code under `@static if`, or one of those shared lines). Reviewed by Mistral ([[Review 2026-11-18 Mistral B-071 (external)]]): two comparison bugs found and fixed before these counts.

**The two wrongs, reported to Moonkale** (the shared normaliser decides both; Sophia works around neither):
1. **A content reference to one method of an open function.** The crate resolves a name to a single definition by looking only at the user's module. A method added from elsewhere, by a qualified head (`Inner.h(x::Int) = …`) or by `import ..M: f` in another module, makes the function generic, and the id still names one method. Besides being false, this puts "calls method m" into the caller's id, which [[SP-006 Open method tables versus content addressing]] rules out. A fix is a migration of ids (SPEC-006 §6): a name that any definition in the package extends (qualified head or import) would become a generic use.
2. **`$name` inside a quoted template** is read as the module's binding. In a `for … @eval` loop it is the loop's variable. A fix: a spliced name is never a use (or is resolved against the loop's bindings).

Classification counts are reproduced by the commands under "How to run"; the fixture's are asserted by `tests/dispatch.rs`.

## Part 4: world change, a method or an import added after loading (2026-11-18)

A **change** is a Julia file evaluated after the package is loaded. Under the package's `src/` it goes into the package's module, and Sophia ingests it as if it were included at the end of the root file; elsewhere it goes into `Main`, standing for another package or an `eval` at run time, which Sophia does not ingest. Julia prints part 3's table (`calls.jl`, [[calls]]) before and after; Sophia runs part 3's comparison on both sides and classifies each (definition, name) by what the change did to it (`world_ledger`, [[conformance]]): still true, followed on re-ingest, **made stale** (wrong after, Sophia still states it), its id moved though its claim did not change, still open. The facts compared are what Sophia stores (content references, generic uses, ids); which method a call reaches (`which`) and Julia's invalidated compiled code are E-3's.

**Fixture** `tests/julia-semantics/world` (package `World`), one change per file, in CI (`tests/world.rs`, against the committed `world/truth/*.tsv`). Reviewed by Codex ([[Review 2026-11-18 Codex B-073 (external)]]): three comparison bugs and one oracle bug found and fixed before these results.

| Change | Julia after | Sophia after | |
|---|---|---|---|
| `duo(x::Float64) = 3` in the package (`changes/duo_more.jl`), or from another package (`outside/duo_elsewhere.jl`); `duo` had 2 methods | `World.duo`, 3 methods | `call_duo` still a generic use of `World.duo`; no id moves | knows: nothing to invalidate (SP-006's answer) |
| `solo(x::String)` in the package (`changes/solo_more.jl`); `solo` had 1 method | `World.solo`, 2 methods | `call_solo`'s content reference becomes a generic use (followed); its id moves, and so does `twice`'s, which refers to `call_solo` by content and claims nothing new | knows, but **ids move**: 2 (approximates: invalidated more than needed) |
| `World.solo(x::String)` from another package (`outside/solo_elsewhere.jl`) | `World.solo`, 2 methods | `call_solo` still refers to `solo(x)` by content | **wrong** (made stale): Sophia cannot see the method |
| `Inner.h(x::Int) = 2` from the parent module (`changes/inner_h.jl`) | `World.Inner.h`, 2 methods | `call_h` still refers to `h(x) = 1` by content | **wrong** (made stale): part 3's first wrong, on re-ingest |
| `sum(v) = 0` in the package (`changes/shadow_sum.jl`), where `call_sum` used `Base.sum` | Julia ≥ 1.12: from the next world `call_sum` calls `World.sum` | content reference to `World.sum`; `call_sum`'s id moves | knows (followed) |
| `include("more/solo_str.jl")`, which adds `solo(x::String)` (`changes/via_include.jl`) | as `solo_more.jl` | as `solo_more.jl` | knows, **ids move**: 2 |
| `World.call_solo(x) = 0` from another package, replacing its method (`outside/redefine_call_solo.jl`) | no method of the package at `call_solo`'s line | the same content reference to `solo(x)` | not compared (`Unchecked`): Julia lists no use there |
| `import Base: maximum`; `maximum(p::Point) = p.x` (`changes/import_max.jl`) | `Base.maximum` gains a method here; `call_max` uses it before and after | nothing stated before or after | leaves open |

Checked by hand in Julia 1.13.1 on the way: a local definition after an implicit use shadows Base's binding from the next world on, whether or not the use already ran; inside one top-level `try` block (one world) the old binding is still seen.

**What the fixture shows.** A generic use survives every method addition unchanged. A **content reference to a function** does not: added inside the package it is followed but moves the caller's id and every id that contains it; added from outside, or by a qualified head, it is stale. The fix proposed to Moonkale after part 3 (a name extended anywhere *in the package* becomes a generic use) would cure `inner_h` but not `solo_elsewhere`: any package may add a method to any function. Only "a function name is never a content reference" makes Sophia's ids independent of the method table, as [[SP-006 Open method tables versus content addressing]] requires; that is a proposal, sent to Moonkale.

**LinearAlgebra** (Julia 1.13.1, normaliser pin `06fd112c`; part 3's table, `Ledger::dependants`): the 846 single-definition content references point at **215 functions**, referred to by 847 definitions directly. Each of these claims is one that another package could make stale with a single method, and each of these 215 is a function whose second method, added in LinearAlgebra, would move ids: per function a median of **3** ids, at most **76** (`LAPACK.chklapackerror`, 75 direct referrers; then `_sym_uplo` 47, `_rmul_or_fill!` 43, `eigtype` 40, `_modify!` 34). These are counts from Sophia's reference graph, not a change Julia was run with.

## Part 5: macros, what a macro call at module level makes (2026-11-18)

A **form** is a module-level expression of the package that is a macro call (a docstring is one, `Core.@doc`) or contains one outside a definition (`for … @eval … end`, `@static if`). Julia's side is `macros.jl` ([[macros]]): it loads the package from source form by form and attributes each method to the form during which it entered the method table (world age), so a method counts for the macro call that made it wherever Julia files it; it also lists the names each form bound, and whether a macro wrapping a definition changes the definition's lowered code (`Meta.lower` with and without the macro). Sophia's side is the definitions the shared normaliser finds in the form. The comparison is `macro_ledger` ([[conformance]]); outcomes per definition (knows, template, not made), per thing Julia made that Sophia does not cover (open), and, for a definition a macro wraps, whether Sophia's id changes when the macro is removed.

**Fixture** `tests/julia-semantics/macros` (package `Mac`, 25 forms, 23 of them compared, making 34 methods), in CI (`tests/macros.rs`, against the committed `macros/truth.tsv`). Reviewed by Mistral ([[Review 2026-11-18 Mistral B-074 (external)]]): six comparison and oracle defects, each reproduced by a case at the end of `Mac.jl` (two forms on one line, a non-ASCII macro argument, a constructor of a type parameter, a module docstring, `baremodule`, `@eval` with `$` outside a loop), fixed before these counts:

| Form | Julia | Sophia | |
|---|---|---|---|
| `@inline inl(x) = x + 1`; also `@noinline`, `Base.@propagate_inbounds`, `@inline` with a default argument | one method (two for the default), its lowered code changed by the macro (an `inline` flag, bounds-check propagation) | the definition `inl`; **its id is the id of `inl(x) = x + 1`** | knows the definition; **wrong**: the id ignores the macro |
| `@generated function gen(x) :(x) end` | a generated function: `gen(1)` returns `1` | the same id as the plain `function gen(x) :(x) end`, which returns the expression `:x` (`generated_and_plain_share_an_id`) | **wrong**: two programs, one id |
| `Base.@kwdef struct K a::Int = 1 end` | the type `K`, its default constructors and a keyword constructor `K(; a = 1)` | the struct; the same id without `@kwdef` | knows the type; constructors left open; **wrong**: the id ignores the macro |
| `@same kept(x) = x + 1` (a macro that returns its argument); `@eval evald(x) = 3` | the same method as without the macro | the definition; the same id | knows |
| `@twice tw` (a user macro whose `quote` defines `tw` and `tw_neg`) | two methods, filed by Julia at the macro's own lines | nothing | leaves open |
| `@deprecate old(x) solo(x)`; `@enum Color red green` | a method of `old`; the type `Color`, constants `red`, `green`, methods of `Base.typemin`, `Base.instances`, … | nothing | leaves open |
| `for (n, v) in … @eval $n() = $v end`; `for T in … @eval tmpl(x::$T) = 1 end` | `one`, `two`; two methods of `tmpl` | one definition each, with `$` in it (the first without a name) | approximates: a template, not a definition |
| `@static if Sys.isapple() plat() = 1 else plat() = 2 end` | one method, from the `else` branch (on Linux) | both definitions | knows the one; approximates the other (not made here) |
| a docstring before `documented(x) = x` | the method and its documentation | the definition | knows (the id is not compared: documentation is not the method) |
| `@inline` inside a submodule with a docstring, in a `baremodule`, on a constructor `(::Type{T})(c::Char) where {T<:Box}` | as above | as above | knows the definition; **wrong**: the id ignores the macro (11 such rows in the fixture) |
| `@inline same_a(x) = 1; @noinline same_b(x) = 2` | two forms on one line | two definitions | not compared: Julia's line cannot tell them apart |

**LinearAlgebra** (Julia 1.13.1, normaliser pin `06fd112c`; `macros.jl` loads it from source): **834 forms** (442 docstrings, 139 `for` loops with `@eval`, 108 `@inline`, 51 `@propagate_inbounds`, 38 `@constprop`, 32 `@deprecate`, 11 `@noinline`, 10 `@commutative`, one each of `@enum`, `@views`, `@static`); all found in Sophia's parse. They make **1 753 methods**:

| Outcome | Rows | Methods | What |
|---|---|---|---|
| knows the definition | 525 | 501 | the definition and Julia's methods at its lines (including `function f end` declarations, and `Base.cbrt` for Julia's `Base.Math.cbrt`, SP-013) |
| approximates: template | 347 | 1 155 | `@eval` templates: 187 make 2 methods, 124 make 4, at most 31; 66 have no name (`$f(…)`, `Base.$f(…)`) |
| approximates: not made | 3 | — | the `@static if Sys.isapple()` branch (`_sysctl_str`, `_sysctl_int32`, `cpus_to_use`) |
| leaves open | 59 | 52 | `@deprecate` (32 methods), `@commutative`'s swapped method (10, which Julia files at no line), `@enum` (6), constructors of `@eval`-made types (4) |
| leaves open: constructors | 37 | 45 | default constructors of documented structs |
| **wrong: the id ignores the macro** | **202** | | every definition a macro wraps whose lowered code the macro changes, and which has an id: `@inline` 106, `@propagate_inbounds` 45, `@constprop` 35, `@noinline` 8, `@commutative` 7, `@views` 1 (the other 15 of the 217 changed forms are refused definitions, with no id) |

**What the part shows.** (1) **The wrong is one rule of the shared normaliser:** a definition's unit is the node *inside* the macro call, so the macro name and its other arguments (`:aggressive` for `@constprop`) are in no definition's encoding, and removing or adding them leaves the id alone. For most macros the difference is how Julia compiles the method (inlining, bounds checks, constant propagation); for `@generated` it is what the function returns, and for `@kwdef` which constructors exist. Reported to Moonkale (`HANDOFF-from-sophia.md`; follow-up B-075). A fix is a migration of ids (SPEC-006 §6): the unit would be the macro call with its definition, or the encoding would carry the macro's name and arguments. (2) **`@eval` loops are the bulk of what Sophia cannot see as methods** (1 155 of 1 753), and Sophia already holds the template; expanding it needs the loop's values, which Julia knows (E-4's option 2). (3) What macros make on their own is small in LinearAlgebra: 97 methods, a third of them `@deprecate`. Whether part 2's 1 059 Julia methods "at lines where Sophia stores none" are mostly these templates is not checked here (part 2 counts methods of named generic functions; a template's methods may or may not have been among them).

## Part 6: closures and captured variables (2026-11-18)

Inside a definition, the shared normaliser decides which names are local (encoded by position, so renaming them leaves the id alone) and which are module names (a content reference, a generic use, or the spelling). In a closure that decision has three answers in Julia: a variable the closure **captures** from the enclosing method, one that is the closure's **own** (an argument or a local), or a **global**. Julia's side is `closures.jl` ([[closures]]): for each closure method, the closure type's fields (`fieldnames(typeof(closure))`; `Core.Box` marks a variable assigned after capture), static parameters it uses, its slot names and the globals of its lowered body. Sophia's side is the definition's encoding (`closure_ledger`, [[conformance]]): a name the normaliser reports as non-local is a module name; a local one is the closure's own if renaming it inside the closure leaves the encoding unchanged, captured if that changes it and renaming it in the whole definition does not.

**Fixture** `tests/julia-semantics/closures` (package `Clo`, 41 closure methods, 58 names compared), in CI (`tests/closures.rs`, against the committed `closures/truth.tsv`; a second test feeds falsified truth lines and gets each of the four kinds of wrong back, so the comparison can see them). Reviewed by Mistral ([[Review 2026-11-19 Mistral B-076 (external)]]): two matching defects (a `do` block whose body line starts another closure; two definitions with closures on one line) found and fixed, each with a case at the end of `Clo.jl`, before these counts:

| Case | Julia | Sophia | |
|---|---|---|---|
| `adder(n) = x -> x + n`; also a `do` block, a generator, a comprehension, nested `b -> c -> a + b + c`, a loop variable, a `let` variable | `n` a field of the closure | bound outside the closure | knows |
| `shadows(k) = x -> x + k` with a module constant `k` | `k` captured | captured, not the constant | knows |
| `counter() = (c = 0; () -> (c += 1))`; `nested_assign`, the same from a closure inside a closure | `c::Core.Box` | captured (the closure's assignment updates the outer `c`) | knows |
| `late() = (f = () -> z; z = 3; f())` with a module global `z`; `late_solo`, the same with a module function `solo` | `z` boxed: the local scope is the whole function, so the closure reads the later local | captured, no content reference to `solo` | knows |
| `branch(b)`: `w` assigned only in an `if` after the closure | `w` boxed | captured | knows |
| `sp(x::T) where {T} = () -> T` | `T` a type parameter of the closure type | captured | knows |
| `fresh() = () -> (y = 1; y)`; `() -> (total = 2; total)` with a module global `total`; `local x` in the closure; a parameter `x -> x + 1` shadowing an outer `x` | the closure's own | own | knows |
| `scaled(v) = map(x -> x * scale, v)`; `() -> solo(a)`; `q` in `h` when only another closure `g` assigns `q` | global | a module name | knows |
| `(x; d = a) -> x + d` | `d` the closure's own (a keyword argument) | a keyword parameter's spelling is in the encoding: renaming cannot decide | leaves open |
| `kwname(atol) = () -> isapprox(1.0, 1.0; atol = atol)` | `atol` captured | undecided: the keyword name `atol` is read as a variable (below) | leaves open |
| `() -> (global total += 1)`; an inner function, in short form or `function … end` | | the definition is refused (`UnsupportedSyntax`) | leaves open (4 closures) |
| `two_h(a) = (x -> x + a); two_k(b) = x -> x + b` | two closures at one line | | not compared: Julia's line cannot tell them apart |

**LinearAlgebra** (Julia 1.13.1, normaliser pin `06fd112c`): 126 closure methods; 108 matched to a closure of Sophia's parse, 18 in 8 definitions Sophia refuses (inner functions such as `print_var`, `_repr`, `abs1`; `_cholpivoted!`, `generic_bunchkaufman!`, three in `tridiag.jl`), none unmatched or ambiguous. **276 names compared:** 58 captured (7 of them static parameters), 116 own, 99 global, all as Julia has them; **0 wrong**; 3 undecided (`atol`, `rtol`, `nans` in `isapprox`'s closure, `generic.jl:2005`, passed on as keyword arguments of the same name). 24 names in matched closures are not in Julia's lists (field and keyword names, names lowering folds away) and are not compared.

**Found on the way (wrong, reported to Moonkale): a call's keyword argument name is in no id.** Tree-sitter's Julia grammar parses `f(x; atol = v)` with `atol = v` as an `assignment` inside the argument list; the normaliser keeps the spelling of a `keyword_argument` or `named_argument` (node kinds of other grammars), not of this, and reads it as an assignment to a local. So `kw_a(atol) = isapprox(1.0, 2.0; atol = atol)` and `kw_r(rtol) = isapprox(1.0, 2.0; rtol = rtol)` share one id, and so do `isapprox(1.0, 2.0; atol = 0.5)` and `isapprox(1.0, 2.0; rtol = 0.5)`, while Julia returns `false` and `true` for each pair (`keyword_names_are_in_no_id` in `tests/closures.rs`; Julia 1.13.1: `julia -e 'include("crates/sophia-ingest/tests/julia-semantics/closures/src/Clo.jl"); println((Clo.kw_a(0.5), Clo.kw_r(0.5), Clo.kw_x(), Clo.kw_y()))'` prints `(false, true, false, true)`). The other way round, `kw_t(tol) = isapprox(1.0, 2.0; atol = tol)` is `kw_a` renamed and has another id (approximates). In LinearAlgebra, **127 keyword arguments in 60 definitions** lose their name (`check` 22, `nA` 10, `nB` 10, `by`, `init` and `shift` 9 each). A fix is a migration of ids (SPEC-006 §6): an `assignment` directly in a call's argument list keeps its left side's spelling. Follow-up B-077.

**What the part shows.** Sophia's v4 scoping agrees with Julia's closure lowering on every name compared, including the cases where Julia's rule is not lexical order (a local assigned after the closure, in a branch, a static parameter); the boxing of captured variables is not represented in the encoding and need not be (it follows from the code). Closures are left open where the whole definition is refused, which an inner named function causes. The wrong is outside closures: keyword names.

## Part 7: mutable globals, content or state (2026-11-19)

A module-level name in a definition's body is, in Sophia's id, a content reference (to the one definition of it), a generic use, or only its spelling ([[Julia Name Resolution]], part 3). For a binding that holds data the right answer depends on Julia's binding partition (Julia ≥ 1.12): a **constant** is content (a use reads the value it was given; a redefinition is a new world, like a new method), a **mutable global** is state (a use reads whatever the binding holds when the code runs, and any code may assign it). Julia's side is `globals.jl` ([[globals]]): each binding the package owns that holds data, with its kind from `Base.binding_kind` and, for a constant, whether its value is a mutable object; per method, the bindings its lowered code reads (`GlobalRef`), writes (`Core.setglobal!`, `setproperty!` on a module) or has as a slot (an argument or local spelled like one). Sophia's side is `global_ledger` ([[conformance]]).

**Fixture** `tests/julia-semantics/globals` (package `Glob`, 20 bindings, 30 uses compared, 3 at lines several definitions share), in CI (`tests/globals.rs`, against the committed `globals/truth.tsv`; one test feeds falsified truth and gets each wrong, the open case and `NoJuliaUse` back, another malformed truth; a third shows the store side). Reviewed by Codex ([[Review 2026-11-19 Codex B-078 (external)]]): six defects, five in the comparison and one in the oracle, fixed with fixture cases at the end of `Glob.jl` before these counts:

| Case | Julia | Sophia | |
|---|---|---|---|
| `const LIMIT = 10`; `use_limit(v) = v + LIMIT`; also `const DOUBLE = 2 * LIMIT` | a constant | a `const` definition; a content reference | knows |
| `const CACHE = Dict{Int,Int}()`, `const TICKS = Ref(0)`, read in methods | a constant whose object is mutable | content reference | approximates: the binding is content, what the object holds is state |
| `scale = 2` read in a body and in a closure; `counter = 0` that a function assigns; `mode` assigned twice; `global later`; `fresh` that only a function assigns; `Inner.depth` | a mutable global | no definition; a use keeps the spelling | knows |
| `shadow_limit(LIMIT) = LIMIT + 1`, `local_limit() = (LIMIT = 3; LIMIT)`, the same with `scale` | an argument or local | a local | knows |
| **`level::Int = 1`; `use_level() = level + 1`; also read in a closure, while `set_level` assigns it** | a mutable global of type `Int` | a definition with an id (the normaliser takes `x::T = v` for the short form `f(x)::T = …`); a content reference to it; in the store a generic function `jl.Glob.level` with one method | **wrong** |
| **`mode2::Symbol = :a; mode2::Symbol = :b`; `use_mode2() = mode2`** | a mutable global | two "methods" of `jl.Glob.mode2`; a generic use | **wrong** |
| **`(UNIT::Scale)(x) = UNIT.k * x` with `const UNIT = Scale(1)`** | `UNIT` is the method's argument | a content reference to `const UNIT`: the method's id depends on a definition it does not use | **wrong** |
| `const STEP = 1; const STEP = 2` (allowed since 1.12); `Inner.WIDTH` by a qualified name; `@enum Colour RED GREEN` | a constant | no content reference | leaves open |
| `set_counter(n) = (global counter = n)`, `function reset_counter() global counter; … end`, `make_later`, `make_fresh` | writes a global | the definition is refused (`global` in a body) | leaves open |
| `set_depth(d) = setproperty!(Inner, :depth, d)` | writes `Inner.depth` | an id; nothing records the write | leaves open |

**LinearAlgebra** (Julia 1.13.1, normaliser pin `06fd112c`): 45 bindings hold data: 43 constants (9 of them mutable objects, the BLAS forwarding maps `LBT_*_MAP`; `ismutable` would also count the strings `libblas` and `liblapack`, which `globals.jl` treats as immutable), 37 with a `const` definition, 6 made by `@enum` (`BlasFlag`) and left open; 2 mutable globals, `ClosedReal = T where T <: …` and `ClosedScalar` in `bunchkaufman.jl`, no definition in Sophia (knows), used only in method signatures, which Julia evaluates once when the method is defined (not compared). **114 uses:** 73 constants by content (knows; 18 of them in constant initializers such as `LBT_INV_INTERFACE_MAP`), 14 uses of constants holding mutable objects (approximates; 4 of them in initializers), 11 locals named `I` (knows), 9 `@enum` values (open), 6 in refused definitions (open), **1 wrong**: `(I::UniformScaling)(n::Integer) = Diagonal(fill(I.λ, n))` (`uniformscaling.jl:84`) refers by content to `const I = UniformScaling(true)`. No method body reads or writes a mutable global of the package, and no typed global exists, so the typed-global wrong does not occur here.

**What the part shows.** The shared normaliser's rule (an untyped `x = v` is no definition; a `const` is one) already makes the distinction Julia makes: content for constants, spelling for mutable globals. The `const` keyword is in the encoding (`const L::Int = 1` and `L::Int = 1` encode differently). Two wrongs, both in how a definition's head is read, are sent to Moonkale (follow-up B-079): `x::T = v` at module level is a typed global, not a function (a fix: an `assignment` whose left side is a `typed_expression` around a bare identifier is no definition, or a definition of a binding that a use never reaches by content); and the arguments of a callable object's method (`(I::T)(…) = …`, a head with no name) are locals. Both change ids (SPEC-006 §6). Left open on purpose: writes (`global x = v` in a body is refused; Sophia has no effect facts), and what a constant's mutable object holds.

## Part 8: parser agreement (2026-11-19)

**Julia's side** is `parse.jl` ([[parse]]): Julia 1.13.1's `Base.JuliaSyntax.parseall(SyntaxNode, text; filename, ignore_errors=true)`, without loading or evaluating the package. Its diagnostics are printed separately. Definitions are source forms, including macro/doc wrappers, conditional definitions and nested named methods; quotes and macro expansion are outside this measurement. Local typed assignments and keyword defaults are not module-level typed globals.

**Sophia's side** is `code::file("julia", text)` per file reached by `module::discover`: identities and explicit refusals, plus the pinned tree-sitter tree's positional parameter count, keyword parameter names and body call keyword count. The join is `(file, name_byte)`, not a source line (Unicode offsets are bytes). The fixture checks `Def.start`, its exclusive end and its name offset against JuliaSyntax's zero-based inclusive spans.

The checked fields are form, name, positional parameter count, keyword parameter names and body call keyword count. `Knows` means these fields agree, not that the whole trees or semantics agree. `Wrong` names differing fields, a missed Julia definition, an extra identified Sophia definition, or an ERROR/MISSING node in an identified definition without Julia's corresponding error. A refused match `LeavesOpen`, with its reason; Julia errors are reported apart. `struct_definition` covers both `struct` and `mutable_struct`, so agreement there `Approximates` (for either form). The full kind mapping is in [[conformance]]. No normaliser, parser or id was changed to make this comparison agree.

**Fixture.** The following table and tally come from:

```sh
julia crates/sophia-ingest/tests/julia-semantics/parse.jl crates/sophia-ingest/tests/julia-semantics/parse/src | sort > crates/sophia-ingest/tests/julia-semantics/parse/truth.tsv
cargo run -p sophia-ingest --example julia_parse -- crates/sophia-ingest/tests/julia-semantics/parse crates/sophia-ingest/tests/julia-semantics/parse/truth.tsv --rows
cargo test -p sophia-ingest --test parse
```

47 comparison rows: **28 knows, 3 approximates, 6 leaves open, 10 wrong**; no malformed truth, Julia errors or undiscovered files. The tests assert every row, falsify every checked field, add and remove definitions, and check malformed input, duplicate keys, recovery refusals and undiscovered files. No Julia runs in CI.

| case | name | outcome | detail |
|---|---|---|---|
| `More.jl:1` | `outer` | LeavesOpen | refused: UnsupportedSyntax |
| `More.jl:2` | `inner` | Knows | checked fields agree |
| `More.jl:5` | `documented_macro` | Knows | checked fields agree |
| `More.jl:6` | `typed_const` | Knows | checked fields agree |
| `More.jl:6` | `typed_const` | Wrong | form: Julia=const Sophia=short; npos: Julia=None Sophia=Some(0) |
| `More.jl:8` | `conditional` | Knows | checked fields agree |
| `More.jl:10` | `check` | Wrong | extra: identified definition absent from JuliaSyntax |
| `More.jl:11` | `==` | Knows | checked fields agree |
| `More.jl:12` | `positional_default` | Knows | checked fields agree |
| `More.jl:12` | `k` | Wrong | extra: identified definition absent from JuliaSyntax |
| `Pars.jl:1` | `Pars` | LeavesOpen | refused: function_definition |
| `Pars.jl:3` | `long_where` | Knows | checked fields agree |
| `Pars.jl:6` | `short_where` | Knows | checked fields agree |
| `Pars.jl:7` | `kw_semicolon` | Wrong | callkw: Julia=1 Sophia=0 |
| `Pars.jl:8` | `kw_comma` | Wrong | callkw: Julia=1 Sophia=0 |
| `Pars.jl:9` | `kw_shorthand` | Wrong | callkw: Julia=1 Sophia=0 |
| `Pars.jl:10` | `x` | Wrong | form: Julia=typed_global Sophia=short; npos: Julia=None Sophia=Some(0) |
| `Pars.jl:11` | `a` | LeavesOpen | unnamed: Moonkale's naming gives this head no name |
| `Pars.jl:11` | `b` | Wrong | missed: JuliaSyntax definition absent from Sophia |
| `Pars.jl:12` | `+` | Knows | checked fields agree |
| `Pars.jl:13` | `⊕` | Wrong | missed: JuliaSyntax definition absent from Sophia |
| `Pars.jl:14` | `+` | Knows | checked fields agree |
| `Pars.jl:15` | `Callable` | Approximates | struct_definition covers struct and mutable_struct |
| `Pars.jl:16` | `-` | Knows | checked fields agree |
| `Pars.jl:17` | `-` | Knows | checked fields agree |
| `Pars.jl:20` | `juxtapose` | Knows | checked fields agree |
| `Pars.jl:21` | `adjoint_case` | Knows | checked fields agree |
| `Pars.jl:22` | `end_index` | Knows | checked fields agree |
| `Pars.jl:23` | `dotted` | Knows | checked fields agree |
| `Pars.jl:26` | `var"odd name"` | LeavesOpen | unnamed: Moonkale's naming gives this head no name |
| `Pars.jl:27` | `interpolated` | Knows | checked fields agree |
| `Pars.jl:29` | `strings` | Knows | checked fields agree |
| `Pars.jl:30` | `do_case` | Knows | checked fields agree |
| `Pars.jl:33` | `lambda_case` | Knows | checked fields agree |
| `Pars.jl:34` | `Options` | Approximates | struct_definition covers struct and mutable_struct |
| `Pars.jl:35` | `n` | Wrong | extra: identified definition absent from JuliaSyntax |
| `Pars.jl:37` | `Mutable` | Approximates | struct_definition covers struct and mutable_struct |
| `Pars.jl:40` | `Bits` | Knows | checked fields agree |
| `Pars.jl:41` | `Abstract` | Knows | checked fields agree |
| `Pars.jl:42` | `Bare` | Knows | checked fields agree |
| `Pars.jl:43` | `bare` | Knows | checked fields agree |
| `Pars.jl:45` | `Inner` | Knows | checked fields agree |
| `Pars.jl:46` | `nested` | Knows | checked fields agree |
| `Pars.jl:48` | `inlined` | Knows | checked fields agree |
| `Pars.jl:52` | `documented` | Knows | checked fields agree |
| `Pars.jl:55` | `m` | LeavesOpen | refused: UnsupportedSyntax |
| `Pars.jl:58` | `refused` | LeavesOpen | refused: global_statement |

**LinearAlgebra.** Reproduced with Julia 1.13.1, normaliser pin `06fd112c`:

```sh
LA=$(julia -e 'print(dirname(dirname(Base.find_package("LinearAlgebra"))))')
julia crates/sophia-ingest/tests/julia-semantics/parse.jl "$LA/src" > parse-la.tsv
cargo run -p sophia-ingest --example julia_parse -- "$LA" parse-la.tsv --rows
```

3 983 comparison rows: **3 001 knows, 56 approximates (structs), 376 leaves open, 550 wrong**; no malformed truth, Julia errors or undiscovered files. Left open: 340 heads Moonkale's naming gives no name (`(\)(F, B) = …`, `Tridiagonal{T}(A) where T`, `CartesianIndex{2}(b)`, `$func` in `@eval` loops; the other fields agree), 36 refused definitions. **Wrong, two kinds and nothing else:**
- **394 phantom definitions:** every `name::T = v` that is not a definition in Julia is identified as a short-form function with an id: typed default parameters (`tril!(M, k::Integer=0)` makes a "function" `k`, `tridiag.jl:856`; 68 named `k`, 44 `check`, 41 `sortby`, 19 each `atol` and `rtol`), typed keyword defaults, struct field initializers under `@kwdef` and typed globals. Classified from the source after the run: all 394 have a `::` right after the name. This is part 7's typed-global wrong (B-079), far more common than part 7 could see: in the store each is a generic function of the package with one method.
- **156 definitions whose call keyword arguments the tree has no keyword node for** (`lbt_openblas_onload_callback`, `LinearAlgebra.jl:890`, 2 keywords): part 6's lost keyword names (B-077), counted here per definition and for every form (`f(a; k = v)`, `f(a, k = v)`, `f(; k)`); part 6 counted 127 keywords after `;` in 60 definitions.

**Not a parser disagreement, fixed in the comparison before these counts** (Claude's reading of Codex's first run, which counted 1 457 wrong): Moonkale's name span covers a qualified name (`Base.to_indices`, `Base.:(==)`) while JuliaSyntax's name is the last component, so the join split each such method into a missed and an extra row; and a head Sophia gives no name was counted wrong instead of open. The fixture has both (`Pars.jl:14`, `More.jl:11`).

**What the part shows.** On every checked field (form, name, positional arity, keyword parameter names, call keywords) tree-sitter-julia and JuliaSyntax agree for 3 001 of LinearAlgebra's 3 983 definition rows, and every disagreement found is one of the two wrongs parts 6 and 7 had already reported, now measured in full: the typed-assignment rule makes 394 definitions that do not exist, the keyword rule loses keywords in 156 definitions. No syntax error of either parser occurs in LinearAlgebra. The fixture shows two more wrongs that LinearAlgebra does not contain: an infix definition `a ⊕ b = …` is not seen as a definition (`Pars.jl:13`), and `const a, b = 1, 2` gives one unnamed definition, so `b` is missed (`Pars.jl:11`). Callable heads, juxtaposition, adjoints, `end` indexing, dotted assignment, strings and string macros, `do` blocks, lambdas and the documentation and `@inline` wrappers agree. Both wrongs are Moonkale's (the shared extractor and normaliser) and change ids when fixed (SPEC-006 §6).

**Limits.** This compares a source projection, not all tree structure, lowering, dispatch, macro output or execution. Quoted definitions are not oracle definitions; a Sophia id for quoted syntax is an extra source claim in this ledger. Definitions in files outside literal include discovery are listed separately. These findings belong to Moonkale's shared extractor/normaliser; no Sophia-side workaround or content-id change is made here. Work was restricted to this worktree, so this session did not write a Moonkale handoff.

## Part 9: planned

| Part | Construct | Julia's ground truth | Sophia's claim |
|---|---|---|---|
| 9 | **include order** | `pathof`, file/line of each definition | include graph (`module::discover`) |

## How to run

```sh
cargo test -p sophia-ingest --test generics
julia crates/sophia-ingest/tests/julia-semantics/methods.jl LinearAlgebra > truth.tsv
cargo run --release -p sophia-ingest --example julia_conformance -- <stdlib>/LinearAlgebra truth.tsv
# part 3
cargo test -p sophia-ingest --test dispatch
julia crates/sophia-ingest/tests/julia-semantics/calls.jl LinearAlgebra > calls.tsv
cargo run -p sophia-ingest --example julia_dispatch -- <stdlib>/LinearAlgebra calls.tsv   # also prints part 4's LinearAlgebra counts
# part 4 (the truth tables: see [[calls]])
cargo test -p sophia-ingest --test world
# part 5 (the truth table: see [[macros]])
cargo test -p sophia-ingest --test macros
julia crates/sophia-ingest/tests/julia-semantics/macros.jl LinearAlgebra <stdlib>/LinearAlgebra/src > macros.tsv
cargo run --release -p sophia-ingest --example julia_macros -- <stdlib>/LinearAlgebra macros.tsv
# part 6 (the truth table: see [[closures]])
cargo test -p sophia-ingest --test closures
julia crates/sophia-ingest/tests/julia-semantics/closures.jl LinearAlgebra <stdlib>/LinearAlgebra/src > closures.tsv
cargo run --release -p sophia-ingest --example julia_closures -- <stdlib>/LinearAlgebra closures.tsv
# part 7 (the truth table: see [[globals]])
cargo test -p sophia-ingest --test globals
julia crates/sophia-ingest/tests/julia-semantics/globals.jl LinearAlgebra <stdlib>/LinearAlgebra/src > globals.tsv
cargo run --release -p sophia-ingest --example julia_globals -- <stdlib>/LinearAlgebra globals.tsv
# part 8 (source truth: see [[parse]])
cargo test -p sophia-ingest --test parse
julia crates/sophia-ingest/tests/julia-semantics/parse.jl <stdlib>/LinearAlgebra/src > parse-la.tsv
cargo run -p sophia-ingest --example julia_parse -- <stdlib>/LinearAlgebra parse-la.tsv
```
