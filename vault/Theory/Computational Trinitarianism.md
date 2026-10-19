#definition #design

> **Computational trinitarianism** is Robert Harper's name for the observation that **logic, type theory and category theory** are three views of one subject: a proposition is a type is an object; a proof is a program is a morphism. Each view gives the same notion of equality and composition, so a fact discovered in one transfers to the others. For Sophia this is not decoration, it is the design. It explains:
> - why programs, specifications and proofs can be **one kind of node in one store**;
> - why each "safe" level of sophialang ([[SPEC-002 Sophialang Principles]] §2) is a **DSL with three faces**: a logic you prove in, a type system you check with, and a categorical semantics that says why it is sound;
> - how those DSL statements are **embedded in the code graph**: as content-addressed definitions of proposition types, linked to their subjects by annotations.
>
> Sources: Harper, *The Holy Trinity* (blog *Existential Type*, 2011) and *Practical Foundations for Programming Languages*, 2nd ed. 2016; Lambek and Scott, *Introduction to Higher Order Categorical Logic*, 1986; Wadler, *Propositions as Types*, CACM 2015; Girard, *Linear Logic*, 1987; Benton, *A Mixed Linear and Non-Linear Logic*, CSL 1994; O'Hearn, Reynolds and Yang, *Local Reasoning about Programs that Alter Data Structures*, CSL 2001; Caires and Pfenning, *Session Types as Intuitionistic Linear Propositions*, CONCUR 2010; Davies and Pfenning, *A Modal Analysis of Staged Computation*, JACM 2001; Plotkin and Power, *Algebraic Operations and Generic Effects*, 2003; The Univalent Foundations Program, *Homotopy Type Theory*, 2013; Fritz, *A Synthetic Approach to Markov Kernels*, 2020; Jung et al., *Iris from the Ground Up*, JFP 2018. code: none yet.
>
> Theory (CT-ML wiki): [Curry-Howard-Lambek Correspondence](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence) · [Category with Families](https://mathstruct.org/CategoryTheory-ML-Wiki/Category-with-Families) · [Linear-Non-Linear Adjunction](https://mathstruct.org/CategoryTheory-ML-Wiki/Linear-Non-Linear-Adjunction) · [Algebraic Effects and Handlers](https://mathstruct.org/CategoryTheory-ML-Wiki/Algebraic-Effects-and-Handlers) · [Freyd Category](https://mathstruct.org/CategoryTheory-ML-Wiki/Freyd-Category) · [Logical Relations](https://mathstruct.org/CategoryTheory-ML-Wiki/Logical-Relations) · [Institution](https://mathstruct.org/CategoryTheory-ML-Wiki/Institution)

## 0. Scope of the claims (added 2026-10-20)

The correspondence is a family of theorems, each about **specific** structures:
- the simply typed λ-calculus with products ↔ intuitionistic propositional logic ↔ cartesian closed categories;
- linear logic ↔ symmetric monoidal (closed) categories;
- dependent type theory ↔ locally cartesian closed categories and categories with families;
- probabilistic programs ↔ Markov categories.

These are **not** interchangeable, and no claim here means "every logic is every type system is every category". Where Sophia relates two systems (two type systems on one term, two languages, a language and its compiled form), it does so through an **explicit interpretation** `⟦−⟧ : S → M` that states which structure it preserves (typing, equality, effects, composition, resource use). Each such claim carries a **preservation obligation**, discharged by a proof (logical relations, simulation, a compiler-correctness theorem) or recorded as an assumption (SR-84, [[SPEC-006 Worlds, Artifacts and Evidence]] §5). The table below is a map of where to look, not a theorem.

## 1. The correspondence

| Logic | Type theory | Category theory |
|---|---|---|
| proposition *A* | type *A* | object *A* |
| proof of *A* ⊢ *B* | program `x : A ⊢ t : B` | morphism *A → B* |
| proof normalisation (cut elimination) | evaluation (β-reduction) | equality of morphisms |
| conjunction *A ∧ B* | product type `(A, B)` | product *A × B* |
| disjunction *A ∨ B* | sum type `Either A B` | coproduct *A + B* |
| implication *A ⇒ B* | function type `A → B` | exponential *Bᴬ* (cartesian closed) |
| true, false | `Nothing`/unit, `Union{}` | terminal object 1, initial object 0 |
| ∀x. P(x), ∃x. P(x) | dependent Π, Σ | right and left adjoints to substitution (categories with families) |
| equality *a = b* | identity type, path type | path objects, ∞-groupoids (HoTT) |
| linear *A ⊗ B*, *A ⊸ B* | resources used once | symmetric monoidal closed categories |
| necessity □*A* | staged code of type *A* | a comonad |
| possibility, effects | monadic types `T A` | monads / Lawvere theories |
| Horn clauses | Datalog rules | regular logic, allegories |
| probabilistic inference | distributions `Dist A` | Markov categories |

What matters is not the table but the **guarantee** behind it. Each row is a *theorem*: the three structures are equivalent, as categories of models. That makes the views interchangeable:
- a **proof** written in a logic can be **run** as a program;
- a **program** can be read as a proof of its type;
- a **categorical model** proves the logic and the type system sound, and gives the compiler a semantics to preserve.

## 2. Why this matters for Sophia

### 2.1 One store for code, specifications and proofs

Because proofs *are* programs, they need no separate storage, hashing or naming scheme:
- **A specification is a type**, so it is stored as a content-addressed definition of kind `type` or `relation`.
- **A proof is a term of that type**, so it is stored as a content-addressed definition, with its own lineage and names.
- **The claim "definition *d* satisfies *P*, by evidence *e*"** is an annotation that names *d*, *P* and *e* (SPEC-001 §8).

The hashing, the names, the change feed, the queries and Moonkale's drawing all work unchanged. Moonkale's North Star §7 ("proofs as annotations") is this, with the evidence itself in the store.

### 2.2 Proofs cost nothing at run time

The type-theory face gives erasure. A proof is a term of a type whose values carry no run-time information: a usage-0 binder, `Prop`. So proofs, specifications and dependent indices are erased by the compiler. **Correctness is stored in the graph and does not slow down the program** (SR-53), not by policy but because the proof is in a universe that the compiler is allowed to erase.

### 2.3 Several type systems on one term

The category face explains why different systems can coexist. Each system is a different *category of models*, and a definition checked under several systems has an interpretation in each one. A **functor** between two models (an [Institution](https://mathstruct.org/CategoryTheory-ML-Wiki/Institution) morphism) says when a fact proved in one transfers to another. "Exchange the dependent checker for a cubical one" (SR-53) is the question of whether such a translation exists for the facts a build relies on.

### 2.4 Equivalence, the interop dream, and reuse

The original idea's "equivalence proofs instead of an FFI" needs a notion of equality that is the same across views:
- **Logic:** a proof of `f ≡ g`.
- **Types:** an identity or path type, or (between languages) a **logical relation**: a relation indexed by types, preserved by every construct.
- **Categories:** an isomorphism, or a natural transformation between two interpretations.

Univalence (cubical) goes further: equivalent types *are* equal, so a function proved correct on one representation **transports** to an equivalent one. That is the formal version of "seriously reuse code" (SR-73): prove two data representations equivalent once, and every theorem and every function moves across.

### 2.5 The query language is logic, too

Datalog rules are Horn clauses, the regular-logic row. Sophialang's `relation`/`rule` definitions, the store's query dialect and the proof DSLs below are **one family**: logic programs over facts. That is why [[Theory of the v0 Store]] §5 proposes one Datalog for queries and for the language's relations.

## 3. The general scheme: one DSL, three faces

Every "safe" level of sophialang (SPEC-002 §2) follows one pattern:

| Face | What it provides | In the store |
|---|---|---|
| **Logic** | the propositions you state, and the proof rules | the DSL's proposition and proof types: content-addressed definitions in sophialang, or in another prover's language |
| **Type theory** | a checker, decidable where possible, that verifies evidence | a `checker` with a version; its runs produce derived facts |
| **Category theory** | a model in which the logic is sound, and which the compiler preserves | documented in the theory notes; links the DSL to the code it compiles to |

And one embedding pattern in the code graph:

```tikz
\begin{document}
\begin{tikzpicture}[x=1cm,y=1cm, every node/.style={font=\small}]
\node[draw, rounded corners] (d) at (0,0) {definition $d$ \footnotesize(content id)};
\node[draw, rounded corners] (u) at (0,-1.6) {term path $d/3/1$: \texttt{Unsafe(memory, \dots)}};
\node[draw, rounded corners] (p) at (6,0) {proposition $P$ \footnotesize(content id)};
\node[draw, rounded corners] (e) at (6,-1.6) {evidence $e : P$ \footnotesize(content id)};
\node[draw, rounded corners, fill=gray!15] (a) at (3,-3.2) {annotation \texttt{proves:memory}};
\node[draw, rounded corners, fill=gray!8] (f) at (3,-4.6) {derived fact \texttt{checked} \footnotesize(checker, version, inputs)};
\draw[->] (a) -- node[left, font=\footnotesize]{subject} (u);
\draw[->] (a) -- node[right, font=\footnotesize]{claims} (p);
\draw[->] (a) -- node[right, font=\footnotesize]{evidence} (e);
\draw[->] (f) -- (a);
\draw[->] (u) -- (d);
\end{tikzpicture}
\end{document}
```

- The **subject** is a definition, or a term path inside one: the `Unsafe` section the proof discharges.
- The **proposition** and the **evidence** are content-addressed definitions. The proposition is usually an instance of a DSL's generic proposition, such as `Owns(heap, p) ∗ …` or `RaceFree(region)`.
- The **annotation** links them. Its id is the hash of (subject, proposition, evidence, checker), so the same claim made twice is one annotation.
- The **derived fact** records that a checker run accepted it, with the checker version and the content ids it used. If the subject changes (a new content id under the same name), the annotation still refers to the *old* content. The new content has an `Unsafe` section without a proof, which is exactly the "stale proof, visible goal" of Moonkale's North Star §7.

## 4. Why each DSL is the right one: the three faces per aspect

### 4.1 Memory (SR-56): separation logic, linear and ownership types, monoidal categories

- **Logic:** separation logic. `p ↦ v` says "I own the cell at *p*, holding *v*". `P ∗ Q` says "*P* and *Q* hold of **disjoint** parts of the heap". The **frame rule** lets a proof about a small piece of code ignore the rest of the heap. Concurrent separation logic and Iris extend this to shared state.
- **Types:** linear and affine types, and ownership and borrowing (Rust). A linear value is a resource used exactly once. A borrow is a temporary, scoped permission. Rust's borrow checker is a *decision procedure* for a fragment of separation logic. It is fast and automatic, but it rejects valid programs: doubly linked lists, graphs with back-pointers, arena indices.
- **Categories:** symmetric monoidal closed categories, and the linear/non-linear adjunction (Benton). The adjunction `F ⊣ G` between a cartesian category (copyable values) and a monoidal one (resources) explains how GC'd values (copyable, managed) and owned values (linear) live in one language. That is the coexistence of the GC and explicit levels in SPEC-002 §5.
- **The DSL:** separation-logic assertions written as sophialang types (`Owns`, `↦`, `∗`, `−∗`), and pre/postconditions on `Unsafe(memory, …)` sections. Evidence comes in three forms:
  1. a borrow-checker-style automatic proof for the common case;
  2. a symbolic-execution proof (VeriFast / Prusti style);
  3. a manual Iris-style proof for the hard cases.
  
  This is "like the borrow checker, but allowing more cases" (SR-56). The automatic checker is one *evidence producer* among several.

### 4.2 Concurrency (SR-57): session types, linear logic, compact closed categories

- **Logic:** linear logic, read as communication. Caires and Pfenning show that intuitionistic linear propositions *are* session types: `A ⊗ B` means "send an *A*, continue as *B*", and `A ⊸ B` means "receive an *A*, continue as *B*". Cut elimination is communication, and deadlock freedom follows from it.
- **Types:** session types for channels; ownership (`Send`/`Sync`-like predicates) for shared memory; lock-order types for deadlock freedom.
- **Categories:** *-autonomous and compact closed categories (process interfaces, duality between the two ends of a channel); traced monoidal categories for feedback.
- **The DSL:** channel protocols as session types; `RaceFree(region)` and `DeadlockFree(lockset)` propositions; concurrent separation logic for shared state. Evidence comes from type checking (sessions), from ownership analysis, or from a model checker run recorded as a certificate.

### 4.3 Compilation and JIT (SR-58): modal logic, staging, comonads

- **Logic:** the modal logic S4. □*A*, "necessarily *A*", reads as "*A* is available at compile time".
- **Types:** staged computation (Davies and Pfenning): □*A* is the type of code of type *A*. A value of type □*A* can be compiled ahead of time. A "no surprise compilation" proof shows that from an entry point, every reachable call is a □-available specialisation. Type stability (Julia's notion) is the premise: the concrete return type is a function of the concrete argument types.
- **Categories:** □ is a comonad. Staging is the coKleisli category of that comonad.
- **The DSL:** `Compiled(entry, signatures)` and `TypeStable(method, signature)` propositions. Evidence is a static analysis (JET-like) plus the dispatch facts that close the reachable set, recorded with the method-table versions it relied on. Adding a method that breaks the closure makes the evidence stale.

### 4.4 Effects and purity: monads and Lawvere theories

- **Logic:** lax or modal logic ("possibly, after effects").
- **Types:** effect rows (`Int →{io} Int`), algebraic effects and handlers.
- **Categories:** monads; Lawvere theories (Plotkin and Power): an effect is an algebraic theory, and a handler is a model of it. Freyd categories for call-by-value.
- **In Sophia:** effect facts justify autoresolved parallelism (SPEC-002 §6), test caching (a test with external effects is never cached, SPEC-002 §8), and dead-code removal.

### 4.5 Equivalence: identity types, univalence, e-graphs

- **Logic:** equality, with the congruence rules.
- **Types:** identity types; path types (cubical); logical relations between languages.
- **Categories:** isomorphisms and natural transformations; ∞-groupoids; e-graphs as free models of an equational theory (a congruence closure).
- **In Sophia:** `equivalence` annotations with evidence: an e-graph rewrite chain, a logical-relation proof, or a test-based (weak) claim. They are used to skip re-running tests (SPEC-002 §8 case 3), to accept mined abstractions ([[Compression and Canonical Form]] §3.1), and for cross-language substitution.

### 4.6 Logic programs and probabilistic rules

- **Logic:** Horn clauses; weighted logic (Markov logic, ProbLog).
- **Types:** relations as types of their proofs (derivations).
- **Categories:** allegories and regular categories for relations; Markov categories for probability.
- **In Sophia:** `relation` and `rule` definitions. A query's answer comes with its derivations, which are proof terms of the relation. That is Lenticulum's LT-26 ("derivations as proof graphs"), here the same object as any other proof.

## 5. The DSLs, summarised

| DSL | Proposition examples | Evidence forms | Checker (fact `by`) | Discharges |
|---|---|---|---|---|
| `memory` | `Owns(h, p)`, `p ↦ v ∗ q ↦ w`, `NoLeak(region)` | automatic ownership proof; symbolic execution; manual separation-logic proof | `sophia-own`, an Iris export | `Unsafe(memory, …)` |
| `concurrency` | `RaceFree(r)`, `DeadlockFree(L)`, `Session(c, S)` | session-type check; ownership analysis; model-checker certificate | `sophia-sess`, `sophia-race` | `Unsafe(concurrency, …)` |
| `compile` | `TypeStable(m, sig)`, `Compiled(entry, S)` | inference trace + dispatch closure | `sophia-stable` | `Unsafe(compilation, …)`; enables a minimal binary |
| `effect` | `Pure(f)`, `Effects(f) ⊆ {io}` | effect inference | `sophia-eff` | autoresolved parallelism |
| `equiv` | `f ≡ g`, `f ≈_R g` (cross-language) | rewrite chain; logical relation; tests (weak) | `sophia-egraph`, provers | re-running tests; substitution |
| `spec` | any sophialang type, `{x \| P(x)}`, dependent types | a term; an SMT certificate; a Lean term (imported) | `sophialang-dependent`, SMT, Lean | correctness claims |

Each DSL lives in its own namespace of sophialang definitions (`sophia.dsl.memory`, …), so a DSL is itself code in the store: versioned, named, hashed, and eventually written in sophialang (SR-55).

## 6. What can go wrong

The honest list (SR-73):
- **The correspondences are exact only for idealised languages.** Real sophialang has general recursion, so not every program is a proof: a non-terminating term proves anything. The proof fragment must be total (a termination checker, or a separate `Prop` universe where recursion is restricted).
- **Erasure needs care.** A proof that is computationally relevant, such as transport along univalence, is not free (Type Taxonomy §6).
- **Several checkers mean several trusted bases.** A claim is only as good as its checker ([[Trusted Computing Base]]). Annotations record which checker vouched for what, so a query can restrict itself to trusted checkers.
- **Proof effort is real.** Separation-logic proofs of tricky code are hard to write. The autoresolve and unsafe levels exist precisely so that proofs are optional.

## Related

[[SPEC-002 Sophialang Principles]] · [[Type Taxonomy]] · [[Theory of the v0 Store]] · [[Equivalence and Witnesses]] · [[Trusted Computing Base]] · [[Effects Memory and Resources]]
