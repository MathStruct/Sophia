#design #definition

> **Draft, 2026-10-20 (Claude), from Daniel's message of 2026-10-20.** This page states what **sophialang**, the store's own language, must be. It is kept separate from **Sophia**, the store that every language lives in ([[SPEC-001 Sophia Store v0]]). It sets out the principles and the design consequences for types, dispatch, memory, concurrency, JIT compilation and tests, and what each one needs from the store. The first, small core calculus that realises a fragment of it is [[SPEC-003 Sophialang Core v0]].
>
> Sources: Julia's semantics (the default whenever in doubt, SR-29); Rust's ownership and `unsafe`; Bezanson et al., *Julia: Dynamism and Performance Reconciled by Design*, OOPSLA 2018; Jung et al., *RustBelt*, POPL 2018; Atkey, *Syntax and Semantics of Quantitative Type Theory*, LICS 2018; Brady, *Idris 2: Quantitative Type Theory in Practice*, ECOOP 2021; Leijen, *Koka: Programming with Row-Polymorphic Effect Types*, MSFP 2014. code: none yet.
>
> Theory (CT-ML wiki): [Curry-Howard-Lambek Correspondence](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence) · [Linear-Non-Linear Adjunction](https://mathstruct.org/CategoryTheory-ML-Wiki/Linear-Non-Linear-Adjunction) · [Algebraic Effects and Handlers](https://mathstruct.org/CategoryTheory-ML-Wiki/Algebraic-Effects-and-Handlers) · [Call-by-Push-Value](https://mathstruct.org/CategoryTheory-ML-Wiki/Call-by-Push-Value). This vault: [[Computational Trinitarianism]] · [[Type Taxonomy]].

## 0. The name and the split

- **Sophia** is the store and its machinery: identities, names, annotations, queries, and later the compiler driver. The name follows Julia's pattern (Julia++ would have been the obvious name; a name ending in *-ia* fits better).
- **sophialang** is one language in that store: the one the store is eventually written in (§9), and the one whose surface is Julia-flavoured by default ([[SPEC-005 Rendering Profiles]]).
- Rust, Julia, Python, C++ or Lean code parsed into the store is **not** sophialang. It keeps its own language tag and normaliser (SPEC-001 §3). Sophialang relates to those languages through proofs and translations, never by pretending to be them.

## 1. When in doubt, Julia (SR-29)

Whenever a question of behaviour is not settled by the principles below, sophialang does what Julia does. Examples:
- **Integers** are machine integers that wrap around on overflow (`Int64` by default on 64-bit machines). Checked arithmetic is a library choice.
- **Arrays** are column-major and 1-based *in their default display*. The stored form records the index *base* only where a base is semantically involved ([[SPEC-005 Rendering Profiles]] §4).
- **`missing` and `nothing`** have Julia's meanings: `Nothing` is the unit type, `Missing` the statistical missing value.
- **Floating point** is IEEE 754, with no fast-math by default.
- **Promotion and conversion** follow Julia's `promote_rule`/`convert` design, expressed as ordinary methods (§4).
- **Scoping** follows Julia's soft and hard scope rules in the surface. The core has only lexical binders.

The list grows as questions come up. Each answer is recorded here with its date.

## 2. The governing principle: autoresolve → unsafe → safe (SR-51)

Every controllable aspect of execution (memory, concurrency, compilation, dispatch, numerics) has three levels:

| Level | What the programmer writes | What happens | Status in the graph |
|---|---|---|---|
| **autoresolve** | nothing | the system chooses: GC, an automatic schedule, JIT on demand, dynamic dispatch | no annotation is needed. The choice made is recorded as a derived fact |
| **unsafe** (explicit) | an explicit control: manual allocation, explicit threads and locks, AOT compilation, a pinned method | exactly what was asked, including foot-guns | the section is marked `unsafe:<aspect>`, which is visible in every view |
| **safe** | an explicit control **plus a proof** in that aspect's DSL | as unsafe, but the proof is checked | an annotation `proves:<aspect>` with a checker and a status ([[SPEC-001 Sophia Store v0]] §8). The `unsafe` mark is discharged |

"Mostly safe" is a proof with explicit, named assumptions (a residual obligation). It is recorded as such, never rounded up to "safe".

Two consequences matter for the store:
- **The level is a property of a *section* of code**, meaning a term path or a whole definition, not of a whole program. A query can list every undischarged `unsafe` section in the transitive closure of `main`. That is the audit Rust's `unsafe` keyword makes possible, but here it extends to memory, concurrency and JIT.
- **Proofs are not part of the program's identity** (SR-53). Adding or removing a proof never changes a definition's content id. A proof **may** change the compiled code, for example by removing a check or a dynamic dispatch, but then the artifact records that it used the proof, and its identity includes it ([[SPEC-006 Worlds, Artifacts and Evidence]] §3). *Corrected 2026-10-20: the first version said proofs never change compiled code, which contradicted §5 to §7.*

## 3. Zero inherent overhead (SR-52, SR-53)

Sophialang may contain nothing that makes compiled code *inherently* slower than C++, Rust or Julia. Concretely:

1. **Types and proofs are erased.** Dependent types, refinement predicates, proofs and specifications have no runtime representation unless the program asks for one. This is the quantitative type theory discipline: every binder has a usage, and usage-0 binders are erased (Atkey 2018; Idris 2).
2. **The type system is exchangeable at compile time.** The compiler needs only the *representation types* ([[Type Taxonomy]] §1: sizes, alignments, calling conventions) and the *dispatch-relevant* types (§4). A dependent typing, a cubical typing or a refinement typing is an L3 annotation that a build may require ("refuse to build unless every definition checks under system X") or ignore. Switching systems never changes the generated code. It only changes what is accepted.
3. **No hidden boxing, no hidden indirection.** Concrete types are unboxed, as in Julia's `isbits` types and Rust's values. Abstractly typed values (`Any`) are boxed, as in Julia: that is a cost the programmer asked for by writing an abstract type, and it is visible as a derived fact ("this binding is boxed").
4. **No mandatory runtime.** The GC, the JIT and the scheduler are *libraries* that a program links only if it uses autoresolve for that aspect. A program that specifies everything explicitly compiles to a binary with no runtime beyond what C would need ([[Minimal Binaries]]).
5. **The store is not in the hot path.** The compiled program never queries the store. All identities are resolved at build time ([[Minimal Binaries]] §2).

## 3a. The mandatory contract (SR-78)

Optional checkers may prove more, but none may contradict this:
- **Representation** is fixed by the representation types, the erasure of usage-0 binders, and nominal layouts ([[Type Taxonomy]] §1). An optional type system cannot change a value's layout, its calling convention, or what is erased.
- **Dispatch** is decided by the classification lattice (Type Taxonomy §2–§3) in the cited world (SPEC-006 §2), never by an optional typing.
- **Execution** of unchecked code is defined (SPEC-003 §7). Stronger annotations without evidence are not exploited: a linear binder used twice runs as an unrestricted one, and an unproved "type-stable" method is dispatched dynamically.
- **Optimisations** that rely on properties beyond the mandatory well-formedness rules require evidence, and the artifact records it.

## 4. Types and polymorphism: multiple dispatch (SR-59, SR-53)

**Polymorphism is multiple dispatch,** as in Julia. A **generic function** is a set of **methods**. Each method has a signature, which is a tuple of types, and a call selects the most specific applicable method.

The interaction with content addressing is the hardest design question in this page. It is recorded as [[SP-006 Open method tables versus content addressing]], and the proposal is:

- **A method is a content-addressed definition** (its signature and body).
- **A generic function is a *nominal* entity**: a fresh identity created once, like a Julia `function f end` or a type in a nominal type system ([[Type Taxonomy]] §3). It is **not** the hash of its method set. Otherwise adding a method to `+` would change the identity of every function that calls `+`.
- **A call refers to the generic function's nominal identity,** not to a method. The *method table* (which methods belong to which generic function) is a mutable relation in the store, versioned like names.
- **Worlds:** results are computed in an immutable world snapshot, not in "the current state" (SPEC-006 §2).
- **Dispatch resolution is derived (L3):** "at method-table version *v*, this call site with these argument types resolves to method *m*". The compiled artefact records the method-table versions it relied on: Julia's backedges and world age, made persistent. Adding a method invalidates exactly the compiled artefacts and test results whose recorded resolutions it changes.
- **Static dispatch is the "safe" level of §2:** a proof that a call site's argument types are concrete enough to resolve at compile time. It removes the dynamic dispatch and its JIT trigger (§7).

Type parameters, abstract types, `Union`, `Any` and the dependent and other systems are systematised in [[Type Taxonomy]]. Their place in the core is in [[SPEC-003 Sophialang Core v0]].

## 5. Memory (SR-56)

| Level | Mechanism |
|---|---|
| autoresolve | **Garbage collection**: a concurrent, mostly non-moving collector whose marking runs on another core (in the spirit of Go's collector, or of the concurrent mark phase of Julia's, which is still stop-the-world for sweeping). The goal is the "does not bother the main process" property: pause times bounded by a published number, measured in the benchmark suite. Escape analysis turns non-escaping allocations into stack allocations. |
| unsafe | **Explicit management**: `alloc`/`free`, raw pointers, arenas, placement, custom allocators, as in C++. |
| safe | **An ownership proof DSL**: ownership, borrowing and lifetimes like Rust's, but as a *proof about* explicit code rather than a restriction of the language. Because it is a proof, it can be stronger than a borrow checker: separation-logic style specifications (points-to, frame rules) admit programs Rust's checker rejects, such as doubly linked lists, graph structures and arena-indexed cycles. The checker is a tool with a version; the proof is an annotation. |

- The levels can mix within a program. GC'd and manually managed memory meet at typed boundaries: a GC'd object may own a manual region with a finaliser; a manual region may hold GC roots only if they are registered.
- The DSL is described in [[Computational Trinitarianism]] §5.1.

## 6. Concurrency (SR-57)

| Level | Mechanism |
|---|---|
| autoresolve | **Task parallelism with a work-stealing scheduler**, as in Julia's `@spawn` and Rust's Rayon: data-parallel `map`/`reduce` over pure functions, chosen by the compiler where purity is known (an effect fact, §8). |
| unsafe | **Explicit threads, atomics, locks, channels, memory orderings**, with C++'s memory model. |
| safe | **A concurrency proof DSL**: data-race freedom (ownership plus `Send`/`Sync`-like predicates, extended through separation logic), deadlock freedom (a lock-order discipline, or session types for channels), linearisability of concurrent objects. |

Purity and effects are tracked as effect rows (Koka style). Autoresolved parallelism is only ever applied to code proved or inferred to be effect-free.

## 7. Compilation: JIT and AOT (SR-58)

| Level | Mechanism |
|---|---|
| autoresolve | **JIT on demand**: a call that hits an unspecialised method signature compiles it, as in Julia. The cache key is the method's content id plus the concrete argument types. A store-backed cache keyed this way is what [[Content-Addressed Precompilation]] means. |
| unsafe | **Explicit AOT**: a list of signatures to compile ahead of time. Calls outside it fall back to the interpreter or JIT, or are refused, as configured. |
| safe | **A "no surprise compilation" proof DSL**: a proof that from an entry point every reachable call site resolves to an already compiled specialisation. It amounts to a type-stability proof plus closure under dispatch: Julia's `@inferred` and JET.jl's static analyses, generalised and recorded. A section with such a proof can run under real-time constraints. |

The analytics that decide what to precompile, because it is compiled most often (SR-66), are queries over these cache facts. They come later.

## 8. Tests are not re-run without reason (SR-54)

A test result is an annotation on the test's content id, recording the content ids of everything the run depended on. A test re-runs only if one of those changed. Three cases:

1. **Static dependencies.** Because ids are Merkle hashes ([[SPEC-001 Sophia Store v0]] §4), an unchanged test id already implies unchanged static dependencies, so the cached result stands. That is free from content addressing.
2. **Dispatch dependencies** (§4). The result also records the method-table versions of every generic function it called. A new method on `+` re-runs only tests whose calls could dispatch to it. This is decided by the signature overlap of the new method with the recorded call signatures.
3. **Semantics-preserving changes.** If `f` changed to `f'` and an equivalence proof `f ≡ f'` exists (an annotation), a test of `g` that used `f` need not re-run. That is the "verifiably does not influence it" of SR-54 at its strongest. It needs the proofs of [[Computational Trinitarianism]] §5.

Effects that escape the store (files, network, clocks, randomness without a seed) make a test *non-cacheable*. It is marked as such and always re-run.

## 9. Self-hosting (SR-55)

Sophialang should be written in sophialang as soon as possible. The path:

1. The store, hashing and the first parser and printer are Rust (S-ADR-0004).
2. Once the core can **run** (an interpreter over core terms), the first sophialang programs are tools about the store: the printer for a rendering profile, then queries, then the type checker for the simple fragment.
3. The Rust code becomes a **bootstrap**: a minimal kernel (hash, store, interpreter) that runs sophialang tools stored in the store.
4. The compiler (lowering to MLIR/LLVM) is written in sophialang, and compiles itself into a minimal binary ([[Minimal Binaries]]).

The trusted computing base stays the Rust kernel plus whatever proof checkers are trusted ([[Trusted Computing Base]]).

## 10. What this asks of the store (fed back to SPEC-001)

- **Nominal entities** (generic functions, nominal types) with fresh identities, beside content-addressed definitions (SP-006).
- **Mutable relations with versions** besides names: method tables.
- **Annotations scoped to term paths** (`unsafe` marks, proofs about sections).
- **Derived facts with recorded inputs**: dispatch resolutions, compiled specialisations, test results.
- **Erasure metadata** (usage per binder), which belongs in the core term because it affects code generation, not only checking.

## Related

[[SPEC-001 Sophia Store v0]] · [[SPEC-003 Sophialang Core v0]] · [[SPEC-005 Rendering Profiles]] · [[Type Taxonomy]] · [[Computational Trinitarianism]] · [[Minimal Binaries]] · [[SP-006 Open method tables versus content addressing]]
