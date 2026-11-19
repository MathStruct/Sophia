#implementation #julia

> The comparisons behind parts 3 to 6 of [[Julia Conformance]]: what Sophia states about each module-level name a Julia definition uses, against what Julia's lowered code uses (part 3, dispatch); what a change to the world, a method or an import added after loading, does to those statements (part 4); what a module-level form that calls a macro made in Julia against the definitions Sophia records in it (part 5); how Sophia's encoding binds each name inside a closure, against what Julia's closure captures (part 6); and whether Sophia treats each global binding that holds data as content or as state, against Julia's binding partition (part 7). Source: `conformance.rs`.
>
> Sources: [[Julia Multiple Dispatch and World Age]]; [[Julia Name Resolution]]; [[Julia Macros and Generated Functions]]. code: `conformance.rs`, `module.rs` (`ingest`), `tests/julia-semantics/calls.jl`, `tests/julia-semantics/macros.jl`, `tests/julia-semantics/closures.jl`, `tests/julia-semantics/globals.jl`, `tests/dispatch.rs`, `tests/world.rs`, `tests/macros.rs`, `tests/closures.rs`, `tests/globals.rs`, `examples/julia_dispatch.rs`, `examples/julia_macros.rs`, `examples/julia_closures.rs`, `examples/julia_globals.rs`

**Sophia's side** is the module-mode ingest (`module::ingest`), the facts `store_module` stores:
- a **content reference** from a definition to another (the crate's `references`): the name has exactly one definition in the user's module and is not imported there. For a function name that is a claim about dispatch: the id says the call reaches *this* method;
- a **generic use** (`calls_generic`, stored as `nominal_refs`): the name has several methods in the user's module; the use names the function (module, name), not a method.

**Julia's side** is `calls.jl`: per method, the bindings its lowered body uses, with each function's owner and its methods. A Julia method is attributed to the innermost identified Sophia definition whose lines contain its line, so a closure's method counts for the definition it is written in.

**Outcomes** (`Outcome`), per (definition, name):

| outcome | when | class |
|---|---|---|
| `KnowsOnlyDefinition` | content reference to a function's definition; every method of Julia's function comes from it (one, or several by default arguments; matched by the definition's lines) | knows |
| `KnowsFunction` | generic use of the function Julia uses | knows the function; the method is left open |
| `KnowsBinding` | content reference to a type, module or constant Julia uses | knows |
| `WrongOneOfMany` | content reference to one method of a function that has several in Julia | **wrong** |
| `WrongTarget` | Sophia's target is not Julia's function or binding | **wrong** |
| `NoJuliaUse` | Sophia states a reference the lowered code does not have: mostly a type named in the signature, which the lowered body does not mention | approximates: not checked by this oracle |
| `WrongInterpolated` | as `NoJuliaUse`, and every use is `$name` in a quoted template (`for (T, uplo) in … @eval … $uplo`): the loop's variable, not the module's binding | **wrong** |
| `OpenHere`, `OpenOutside` | Julia uses a function or type (of this package, or outside it) about which Sophia states nothing | leaves open |

A claim is matched to Julia's use of the same binding (`Module.path.name`), else to a use of the same name, so `h(x) + Inner.h(x)` keeps the two apart. A line holding several Sophia definitions (`f(x) = 1; g(x) = 2`) is not attributed: Julia gives a method's line only. Compiler-made names (`#…`, closures and keyword sorters) are skipped. Sophia's definitions in which Julia lists no use (a body with no global, code under `@static if`, a refused or never-compiled definition) and Julia methods inside no Sophia definition (macro output, default constructors) are counted, not compared.

It does **not** compute dispatch for given argument types; Julia's `which` would. A function-level answer (`KnowsFunction`) is the most Sophia claims by design ([[Julia Multiple Dispatch and World Age]]).

## Part 4: a change to the world (`world_ledger`)

A **change** is a Julia file evaluated after the package is loaded (`calls.jl PKG SRCDIR CHANGE`). Under the package's `src/` it is evaluated into the package's top module, and Sophia ingests it, with the files it includes, as a file included at the end of the root file; anywhere else it is evaluated into `Main`, standing for another package or an `eval` at run time, and only Julia sees it. The package's files are never edited.

`world_ledger` runs the part 3 comparison twice, on the package with `calls.jl`'s output before the change and on the package (plus the change, if ingested) with the output after it, and pairs the rows by (file, line, caller, name). The definitions of the change and of the files it includes are skipped: they are new code, part 3's subject. Paths are canonicalised first, so file names relative to `src/` match Julia's however the arguments are written. Reviewed: [[Review 2026-11-18 Codex B-073 (external)]]. Each pair gets a `WorldOutcome`:

| outcome | when | class |
|---|---|---|
| `StillTrue` | the same true claim (outcome and Sophia's target) before and after, the caller's id unmoved | knows: nothing to invalidate |
| `Followed` | the claim changed on re-ingest and is not wrong after (a generic use where there was a content reference, a content reference where Base was used, a claim withdrawn) | knows |
| `MadeStale` | wrong after, and not the same wrong before | **wrong**: a fact the change made false |
| `StillWrong` | the same wrong before and after | part 3's wrong |
| `IdMoved` | the same claim, but the caller's id moved (it refers by content to a definition whose id moved) | approximates: invalidated more than needed |
| `StillOpen` | Sophia states nothing (or nothing this oracle checks) before and after, id unmoved | leaves open |
| `Unchecked` | Julia lists no use on one side (another package replaced the method, or a use appeared), and the pair is not wrong after | not compared |

`ids_moved` lists every package definition outside the change whose id differs after it, whether or not Julia lists a use in it.

It does **not** compare which method a call reaches before and after (`which`, E-3), nor Julia's invalidated compiled code (backedges); it compares the facts Sophia stores.

## Part 5: macros (`macro_ledger`)

**Julia's side** is `macros.jl` ([[macros]]): for each module-level form that calls a macro, the methods the form put in Julia's method table (attributed by world age, so wherever Julia files them), the names it bound, and for a macro wrapping a definition whether the macro changes the definition's lowered code.

**Sophia's side** is the module-mode ingest: the definitions the shared normaliser finds inside the form's span (refused ones too), each with its kind and, for a method, the generic function it belongs to (its own module, or the one it extends: `julia_extends`). The form is found in Sophia's parse by its first line; a string before a form is its docstring.

**Outcomes** (`MacroOutcome`):

| outcome | when | class |
|---|---|---|
| `KnowsDefinition` | a Sophia definition, and Julia's form made methods of that function at the definition's lines (any number: default arguments make several), or bound the type, constant or macro it names; `function f end` (a declaration, no method) counts | knows |
| `Template` | as `KnowsDefinition`, but the definition holds a `$` splice outside a quote of its own (an `@eval` template); for a type or constant, the form bound one or more of that kind | approximates: one definition stands for a method (or type) per loop value |
| `NotMade` | a Sophia definition of which Julia's form made nothing: a `@static` branch not taken on this platform | approximates |
| `Open` | methods (one row per generic function) or a binding Julia's form made that no Sophia definition covers: a macro's own output (`@deprecate`, `@enum`, a user macro), methods filed elsewhere | leaves open |
| `OpenConstructors` | methods of a struct the form defines, all at the struct's lines: Julia's default constructors and `@kwdef`'s keyword one | leaves open |
| `IdSeesMacro` | a macro call wraps a definition (`@m args… def`), the macro changes its lowered code, and Sophia's id changes without the macro | knows |
| `IdIgnoresMacro` | the macro changes the lowered code, and Sophia's id is the same without it: two definitions, one id | **wrong** |
| `IdNoChange` | the macro changes nothing (`@eval` without `$`, a macro returning its argument), and neither does the id | knows |
| `IdSplits` | the macro changes nothing, and the id changes | approximates |

The id comparison ingests the package a second time with every macro call that wraps a definition blanked out (spaces, up to the definition, so every span stays where it was) and compares each wrapped definition's id. Matching a Julia method to a Sophia definition takes the function and the lines: the method's generic function must be the definition's, or have its name (an alias such as `Base.cbrt` for `Base.Math.cbrt`, SP-013), and the method's line must lie inside the definition. A docstring's definition is compared for what it makes but not for its id (documentation is not the method).

Not compared: a line with several forms (`MacroLedger::ambiguous`: Julia's line cannot tell them apart), and the id of a definition in a parenthesised macro call (`@m(def)`, which is not blanked). Known limits: methods made by a literal `include` *inside* a form are counted for that form, and since the included file's definitions are not in the form's span they show as open; a definition without a name (`$n() = …`, `(::Type{T})(…)`) takes any method of its form at its lines. Reviewed: [[Review 2026-11-18 Mistral B-074 (external)]] (six defects fixed, each with a fixture case).

It does **not** expand macros for Sophia (that is E-4's option 2, a transformation record); it measures what is lost without doing so.

## Part 6: closures (`closure_ledger`)

**Julia's side** is `closures.jl` ([[closures]]): per closure method, its arguments, its captured variables (the closure type's fields, boxed or not, and static parameters), its other locals and the globals its lowered body uses.

**Matching.** A Julia closure is looked for in the outermost identified Sophia definitions whose lines contain its line, among the closures of Sophia's parse: anonymous functions, `do` blocks, inner named functions (short form or `function … end`), generators and comprehensions. A closure with a line is matched to those of Sophia's at that line: a closure's first line, for a `do` block the first line of its body, which is the line Julia gives it (else the innermost closure around the line). When several definitions share the line and more than one has a closure there, Julia's line cannot tell them apart: not compared (`ambiguous`). a generator to the generators of the definition. Of those, the one whose parameters are Julia's arguments (a destructured argument's names are Julia's locals; `_` is unnamed), else the only one whose parameters include them. Several closures of Sophia's and as many of Julia's with the same arguments at one place are paired in source order, which is the order of the numbers lowering gives them (`#f##0…` before `#f##2…`; an outer closure before the ones inside it). Several Julia lines that match one closure of Sophia's (a keyword body, default arguments) are merged.

**Sophia's side** is the encoding of the definition, by the shared normaliser, with no references (they do not decide what is local):
- a name is a **module name** when `definition_and_uses` reports its occurrence as non-local;
- otherwise it is a local, and the comparison renames it: its occurrences **inside the closure** to a fresh name, then **in the whole definition**. If the first leaves the encoding unchanged, the closure binds the name itself (alpha-renaming): **own**. If the first changes it and the second does not, the name is bound outside the closure: **captured**. If both change it, the spelling is part of the encoding (a keyword parameter's name) and the question stays open; so it does when the definition's text does not encode on its own.

Field names (`p.x`) and the keyword names of a call (`f(x; atol = …)`) are not names of variables and are not compared; nor are names inside a quote, a string or a macro name. A name Julia does not list in the closure (a field or keyword name, a name lowering folds away) is counted (`unlisted`), not compared.

**Outcomes** (`ClosureOutcome`), per (closure, name):

| outcome | Julia | Sophia | class |
|---|---|---|---|
| `KnowsCaptured` | captured (`captured`, `captured:box`, `captured:sparam`) | captured | knows |
| `KnowsOwn` | an argument or local of the closure | own | knows |
| `KnowsGlobal` | a global binding | a module name (which one is part 3's question) | knows |
| `WrongGlobal` | a variable | a module name | **wrong** |
| `WrongLocal` | a global binding | a local | **wrong** |
| `WrongCapturedAsOwn` | captured | own | **wrong** |
| `WrongOwnAsCaptured` | own | captured | **wrong** |
| `Mixed` | | some occurrences a module name, others local | approximates |
| `Undecided` | | the spelling is in the encoding | leaves open |

Also counted: Julia closures in a definition Sophia refuses (`refused`, left open), in no definition, matching none or several of Sophia's closures, and truth lines that are not closure lines (`malformed`). Reviewed: [[Review 2026-11-19 Mistral B-076 (external)]] (two matching defects fixed, each with a fixture case). Known limits: an anonymous function inside a quote counts as a closure (an `@eval` template's closure is one); a Julia closure with no exact match takes the only Sophia closure whose parameters include its arguments.

**Keyword names** (`keywords_unspelled`), in every identified definition: a call's keyword argument whose name the encoding does not keep. The shared normaliser reads `atol = x` inside a call's argument list as an assignment to a local, so the keyword's spelling is in no id: `isapprox(a, b; atol = x)` and `isapprox(a, b; rtol = x)` share one (**wrong**; reported to Moonkale). Detected by probing the keyword's identifier: a name the encoding keeps is reported as non-local.

`tests/closures.rs` also feeds the comparison falsified truth lines, so that each of the four wrongs is shown to be reported.

It does **not** check what a closure captures *by value*: Julia's boxes (`captured:box`) are a property of the variable, and Sophia's encoding has no notion of them; a captured variable is compared as captured either way.

## Part 7: mutable globals (`global_ledger`)

**Julia's side** is `globals.jl` ([[globals]]): every binding the package's modules own that holds data (a constant whose value is not a function, type or module, or any mutable global), with its kind (`const`, `global`, `declared`) and, for a constant, whether its value is a mutable object; per method, each such binding it reads or writes, and each of its slots spelled like one (`local`).

**Sophia's side.** A binding's definitions are the identified module-level definitions with that name in that module (a refused one has no id and claims nothing): a `const` statement, or an `assignment` (`x::T = v`, which the normaliser takes for the short form of a function). An untyped `x = v` is no definition. A use is attributed to the innermost definition (identified or not) whose lines contain the method's line: of the definitions containing that line, those that contain no other one (by bytes). Several such (two definitions on one line, or one ending on the line where the next starts) are not compared (`ambiguous`), and their content references to that binding are not reported as `NoJuliaUse`; none counts as `outside`. Sophia's claim about binding `b` in that definition is a content reference to a definition of `b`, a generic use of `b`'s name inside the definition's bytes (`calls_generic`), or nothing (the spelling).

**Truth checked.** A line that is not a `binding` line with a known kind and object, or a `use` line with a numeric line, a known access and the kind of its binding's `binding` line, is counted as `malformed` and not compared; so is a binding listed with two kinds (and every use of it).

**Outcomes** (`GlobalOutcome`). Per binding: `KnowsConstant` (a constant with only `const` definitions), `KnowsState` (a mutable global with none), `OpenConstant` (a constant with none: `@enum`, `const a, b = …`), `WrongContentOfMutable` (a mutable global with a definition, so with an id). Per use:

| outcome | Julia | Sophia | |
|---|---|---|---|
| `KnowsConstant` | reads a constant | content reference | knows |
| `ApproxMutableObject` | reads a constant whose value is mutable (`Dict`, `Ref`) | content reference | approximates: the binding is content, what the object holds is state |
| `KnowsState` | reads a mutable global | nothing | knows: the id holds the spelling, the value is read when the code runs |
| `KnowsLocal` | a slot spelled like the binding | nothing | knows |
| `OpenConstant` | reads a constant | nothing (a qualified name, a constant defined twice, macro output) | leaves open |
| `OpenRefused` | reads, writes or binds it locally | the definition has no id (`global x = v` in a body) | leaves open |
| `OpenWrite` | writes it | the definition has an id; nothing says it writes | leaves open |
| `WrongContentOfMutable` | reads a mutable global | content reference | wrong |
| `WrongGenericOfMutable` | reads a mutable global | generic use | wrong |
| `WrongLocalAsGlobal` | a slot spelled like the binding | content reference or generic use | wrong |
| `NoJuliaUse` | does not read it in that method | content reference | |

A local that Julia also reads as the global in the same method is not compared. A content reference from a definition that is itself a data binding (`const B = 2 * A`) has no Julia method behind it (Julia evaluated it at load): it is classified by the target's kind alone.

`tests/globals.rs` also feeds falsified truth (a constant made mutable, a mutable global made constant, a use removed, a truncated line, an unknown access, a non-numeric line, a contradicting kind, a binding with two kinds), so the wrongs, the open case, `NoJuliaUse` and `malformed` are shown to be reported. Reviewed: [[Review 2026-11-19 Codex B-078 (external)]] (five defects in the comparison and one in the oracle, fixed with fixture cases).

It does **not** check what a write does at run time (world age of a binding's value, `Core.setglobal!` against a typed global's conversion), nor globals of modules outside the package.

## Part 8: source parser agreement (`parse_ledger`)

`parse.jl` supplies JuliaSyntax definitions and error diagnostics. `module::discover` selects the package files, and `code::file("julia", text)` supplies identities and explicit refusals. The comparison joins on relative file and the byte offset of the name's last component: JuliaSyntax names `Base.show` by `show`, while Moonkale's name span covers `Base.show` (also `Base.:+` → `+`, `Base.:(==)` → `==`), and names are compared by that last component; tree-sitter supplies positional/keyword parameters and counts only its actual `named_argument` nodes in calls. Interpreting `assignment` nodes as keywords here would hide the known silent disagreement.

`ParseOutcome` is `Knows` (all checked fields agree), `Approximates` (a shared kind covers more than one Julia form), `LeavesOpen` (the matched definition is refused, with reason), or `Wrong` (field differences, missed or extra identified definition, or tree-sitter ERROR/MISSING in an identified definition without a corresponding JuliaSyntax diagnostic). `ParseRow.detail` names differing fields. Refusals take precedence over field comparison: they claim no identity. Julia errors, malformed truth and oracle files outside the include graph are reported apart.

Julia's Moonkale definition kinds map as follows: `function_definition` → function; `assignment` → short; `macro_definition` → macro; `struct_definition` → struct or mutable_struct (both approximated); `abstract_definition` → abstract; `primitive_definition` → primitive; `module_definition` → module; `const_statement` → const. The typed-global bug is deliberately exposed by mapping `assignment` to short, not repairing it from the source. Long versus short methods are distinct kinds; baremodule versus module share a kind and the oracle's `module` form. Definitions without a name span use their structural head's start as the join key (including callable heads); an incorrect named callable head is compared at its stated name span. A head Moonkale's naming leaves unnamed (`(\)(F, B)`, `Tridiagonal{T}(A)`, `var"…"`) while JuliaSyntax names it is `leaves open` when every other field agrees: Sophia states no name, so it states nothing wrong.

The checked projection is form/name/positional arity/keyword parameter names/body call keyword count. Equal projections do not establish equal trees, equal lowering or semantic correctness; string interpolation, quotes and macros are not evaluated. See [[parse]] for oracle traversal boundaries and offset conventions, and [[Julia Conformance]] for measured results.
