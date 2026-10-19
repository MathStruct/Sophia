#design #definition

> **Draft, 2026-10-20 (Claude).** The first core calculus of sophialang: the normal form that sophialang definitions are stored and hashed in (layer L1 of [[SPEC-001 Sophia Store v0]]). It is deliberately a *fragment* of what [[SPEC-002 Sophialang Principles]] asks for: enough to store, name, display and edit real definitions, to carry multiple dispatch and erasure from the start, and to be checked and run later. It replaces §3 and §4 of the SPEC-001 of 2026-10-17.
>
> Sources: Unison (structural vs `unique` types, constructors as `(type, index)`); Julia (methods, generic functions, primitive types); Atkey 2018 and Idris 2 (usage annotations for erasure); Nanevski, Pfenning and Pientka 2008 (holes as contextual metavariables). code: `crates/sophialang-core` (planned).
>
> Theory (CT-ML wiki): [Abstract Syntax with Binding](https://mathstruct.org/CategoryTheory-ML-Wiki/Abstract-Syntax-with-Binding) · [Category with Families](https://mathstruct.org/CategoryTheory-ML-Wiki/Category-with-Families) · [Polynomial Functor](https://mathstruct.org/CategoryTheory-ML-Wiki/Polynomial-Functor). This vault: [[Type Taxonomy]] · [[Theory of the v0 Store]].

## 1. Definition kinds

| Kind | What it is | Identity | Moonkale kind |
|---|---|---|---|
| `generic` | a generic function: a name for a set of methods | **nominal** (SPEC-001 §4.3) | `sophialang:GenericFunction` |
| `method` | one method: a signature (a tuple of types, with type parameters) and a body. Joined to its generic function through the nominal relation `methods(g)` | content | shared `function`, with a `MethodOf` edge |
| `constant` | a named term with a declared type | content | shared |
| `struct` | a **nominal** product or sum type, as Julia's `struct` and Unison's `unique type`: two structurally equal structs are different types | nominal, plus a content-addressed *layout* definition | shared `type` |
| `data` | a **structural** inductive type, possibly indexed (`Vec(A, n)`): two equal declarations are one type | content | shared `type` |
| `abstract` | an abstract type in the Julia sense (`abstract type Number end`), with a supertype | nominal | shared `interface` |
| `primitive` | a machine type with a bit width (`primitive type Int64 64 end`) | nominal, built in | shared `type` |
| `relation` | **nominal** since S-ADR-0009: a predicate. Its declared type is a content-addressed `relation` signature definition, which joins it like a method; its rules join it the same way | nominal, plus a content-addressed signature | `sophialang:relation` |
| `rule` | a Horn clause with logic variables and an optional declared weight | content | `sophialang:rule` |
| `test` | a closed term of type `Test` | content | shared `test` |
| `postulate` | a declared type with no body: an axiom, or a function not yet written | content | `sophialang:postulate` |

- Constructors are `(type id, index)`. They are named by display metadata, not by separate definitions.
- Methods of a `generic` are added and removed through the nominal relation, never by editing the generic.

## 2. Terms

```
Term ::= Var(i)                         -- de Bruijn index
       | Ref(content-id)                -- a content-addressed definition outside the group
       | Member(k)                      -- member k of this definition's recursive group
       | Nominal(nominal-id)            -- a generic function, struct, abstract or primitive type
       | Con(type, j)                   -- constructor j of a data type or struct (type = Ref or Nominal)
       | Lam(Binder, Term)              | App(Term, [Term])         -- n-ary application (dispatch sees all arguments)
       | Pi(Binder, Term, Term)         -- dependent function type
       | Let(Binder, Term, Term, Term)  -- type, value, body
       | Match(Term, Term?, Term?, [Case]) -- scrutinee, optional data type (names the constructors; hashed), optional motive, cases
       | Type(level) | Prop
       | Lit(Int64 | UInt64 | Float64 | String | Bool | Char)
       | Prim(op)                       -- a built-in operation from a versioned table (add_int, …)
       | Hole(Term)                     -- expected type; numbered by first visit when hashing
       | Unresolved([segment])          -- a name that did not resolve
       | Unsafe(aspect, Term)           -- the term opts into explicit control (SPEC-002 §2)
Binder ::= (display name, usage ∈ {0, 1, ω}, implicit?)  -- the display name is not hashed
Case   ::= (constructor index, binders, Term)
```

What changed against the draft of 2026-10-17, because of SPEC-002:
- **`Nominal`** carries generic functions and nominal types. A call `f(x, y)` is `App(Nominal(f), [x, y])`. Dispatch is resolved later, as a derived fact (SPEC-002 §4).
- **n-ary `App`**, because multiple dispatch looks at all arguments together.
- **Usage on binders.** Usage `0` means erased at run time (types, proofs). Usage `1` means linear. Usage `ω` means unrestricted (SPEC-002 §3.1). Usage is **hashed**, because it changes the compiled code.
- **`Unsafe(aspect, term)`** marks a section as explicit (`memory`, `concurrency`, `compilation`). It is hashed, because it changes how the code is compiled. The proof that discharges it is an annotation, and is **not** hashed.
- **More literals**, matching Julia's common machine types.

Not in v0: effects in the type, arrays, mutation, `ccall`, macros. They need their own designs. Arrays come with the index-base work of [[SPEC-005 Rendering Profiles]] §4.

## 3. What is hashed

- **Hashed:**
  - the kind;
  - the declared type or signature;
  - the body;
  - usages;
  - implicitness;
  - `Unsafe` marks;
  - literal values (floats by their bits);
  - primitive op names (the table is versioned with the normaliser);
  - a hole's expected type (holes are numbered in first-visit order);
  - an unresolved name's path;
  - declared rule weights;
  - for `data`, the constructors' types in order.
- **Not hashed:**
  - binder names, constructor names, the definition's own name;
  - docs and comments;
  - inferred types, typings by any system, proofs;
  - test results;
  - learned weights: they are parameter artifacts, data in their own right (SPEC-006 §4).
- **Nominal entities** hash nothing about their body. Their id is fresh (SPEC-001 §4.3). A `struct` additionally has a content-addressed **layout** definition (its field types in order). The struct's nominal id is related to its layout by a relation, so that a change of fields is a new layout under the same nominal type. For a struct, that is what an edit means.

The normaliser's name is `sophialang-core`, version `0`.

## 4. Holes, unresolved names, incomplete definitions

As SPEC-001 §4.4:
- **Holes** are metavariables with an expected type ([[Theory of the v0 Store]] §3).
- **Unresolved names** stay unresolved until `resolve` creates a new, resolved definition and moves the name to it.

Both make the definition `incomplete`.

## 5. Display data

Binder names, constructor names and comments attached to term paths ([[SP-001 Comments inside a definition body]]) are stored as display data per definition, keyed by the binder's pre-order ordinal and by term path.

α-equivalent definitions share content and keep the first display data stored ([[SP-003 Display names of alpha-equivalent definitions]]).

## 6. Example

The default (Julia-flavoured) projection ([[SPEC-005 Rendering Profiles]]) prints:

```julia
abstract type Shape end
struct Circle <: Shape
    r::Float64
end
area(c::Circle)::Float64 = 3.141592653589793 * c.r * c.r       # a method of the generic `area`

data Nat
    zero
    succ(n::Nat)
end

data Vec(A::Type, n::Nat)
    nil :: Vec(A, zero)
    cons(x::A, xs::Vec(A, n)) :: Vec(A, succ(n))
end

relation parent(::Person, ::Person)
rule ancestor(X, Y) :- parent(X, Y)
rule ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z)

postulate sort(xs::List(Int64))::Sorted(xs)

@test area(Circle(1.0)) ≈ 3.14159
```

In the core:
- `area` is `Nominal(area)`, and the method above is one member of `methods(area)`;
- `Circle` is `Nominal(Circle)` with layout `[Float64]`;
- `Nat` is a content-addressed `data`;
- `≈` is a generic function.

## 7. Mandatory well-formedness and execution (SR-78)

Every stored complete definition must satisfy these rules, whatever optional checker is or is not present:
- **scoping:** every `Var(i)` is bound; `Member(k)` and `Ref` resolve;
- **constructors:** with a known data type, constructor indices are in range;
- **erasure:** a usage-0 variable occurs only in erased positions (types, proofs, other usage-0 arguments). Erasure changes the generated code, so this check is mandatory, not optional;
- **arity:** `App` heads that are known definitions get at most as many arguments as their type has binders.

Holes and unresolved names make a definition incomplete, which is allowed in the store but not executable (§4).

**Execution** is call-by-value over the erased term, with the dispatch of the cited world. Without evidence, usage 1 means ω, an unproved `Unsafe` section runs exactly as written (its foot-guns included), and no property beyond these rules is assumed. These checks will run when an interpreter exists (M4). Until then they are the contract any implementation must meet.

## 8. Effects boundary (SR-81)

v0 has no effects in the core, but it reserves the boundary:
- pure terms are what §2 describes;
- effectful computation will be separated from values, in the style of call-by-push-value (values versus computations, with `thunk`/`force`), so that effect rows (Koka style) attach to computations only;
- I/O, exceptions, state, nondeterminism, concurrency and staging are each algebraic effects with handlers ([[Computational Trinitarianism]] §4.4).

Whether the core itself becomes call-by-push-value, or only an intermediate representation does, is to be evaluated before effects are added. Nothing in v0's encoding prevents either: new constructors get new tags.

## Implementation status (2026-10-20)

- Implemented in `crates/sophialang-core` ([[sophialang_core]]) and printed and parsed by `crates/sophialang-syntax` ([[sophialang_syntax]]).
- **`Match` carries an optional data type.** Without it, a printed pattern cannot name its constructors (`zero`, `succ`), because a case holds only a constructor index.
- **In M2, methods are referenced statically.** A `function` definition is a content-addressed method that callers refer to by its id. Attaching methods to nominal generic functions, and dispatching through method tables, comes with M4 (SP-006).

## Related

[[SPEC-001 Sophia Store v0]] · [[SPEC-002 Sophialang Principles]] · [[SPEC-005 Rendering Profiles]] · [[Type Taxonomy]] · [[SP-006 Open method tables versus content addressing]]
