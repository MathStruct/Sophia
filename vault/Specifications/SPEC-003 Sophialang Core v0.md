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
| `relation` | a predicate with argument types | content | `sophialang:relation` |
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
       | Match(Term, Term?, [Case])     -- scrutinee, optional motive, cases
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
  - learned weights (they are derived facts from a training run).
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

## Related

[[SPEC-001 Sophia Store v0]] · [[SPEC-002 Sophialang Principles]] · [[SPEC-005 Rendering Profiles]] · [[Type Taxonomy]] · [[SP-006 Open method tables versus content addressing]]
