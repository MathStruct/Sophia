#definition #design

> Why the base Sophia's identities are built the way they are: what a content hash of a core term is, mathematically; why declared types belong inside the hash; what a hole is; why names form a lens over a store of immutable terms; and why the query language and the language's own relations might be one Datalog. The general mathematics lives in the CT-ML wiki, so this note keeps to what is specific to Sophia.
>
> Sources: Unison codebase format (hashing and cycles); de Bruijn, *Lambda calculus notation with nameless dummies*, Indag. Math. 34 (1972); Nanevski, Pfenning and Pientka, *Contextual Modal Type Theory*, ACM TOCL 9(3) (2008); Abiteboul, Hull and Vianu, *Foundations of Databases* (1995), ch. 12–13 (Datalog). code: `crates/sophia-hash`, `crates/sophia-core`, `crates/sophia-store`.
>
> Theory (CT-ML wiki): [Abstract Syntax with Binding](https://mathstruct.org/CategoryTheory-ML-Wiki/Abstract-Syntax-with-Binding) · [Polynomial Functor](https://mathstruct.org/CategoryTheory-ML-Wiki/Polynomial-Functor) · [Category with Families](https://mathstruct.org/CategoryTheory-ML-Wiki/Category-with-Families) · [Relational Lens](https://mathstruct.org/CategoryTheory-ML-Wiki/Relational-Lens) · [Least Fixed Point](https://mathstruct.org/CategoryTheory-ML-Wiki/Least-Fixed-Point) · [Conjunctive Query](https://mathstruct.org/CategoryTheory-ML-Wiki/Conjunctive-Query)

## 1. A content hash is a fold over the initial algebra

Core terms form the initial algebra of a signature with binding: a polynomial functor `Σ`, extended with a context index for variables. In the nameless (de Bruijn) presentation, α-equivalent terms are *equal* as elements of that algebra. So any function defined by structural recursion, any fold, is automatically α-invariant.

The canonical encoding is such a fold into byte strings. `BLAKE3 ∘ encode` is therefore a function on α-equivalence classes. Collision resistance makes it injective in practice, which is the "identity = structure" claim of SR-01.

References to other definitions are leaves of the signature. A leaf is labelled by the referenced digest, not by a name. That makes the whole store a **Merkle DAG**: a digest determines the entire transitive closure of what it depends on. That is "unique reproducible ids from the tree of dependencies", from [[The Original Idea]], made exact.

**Recursive groups** break the DAG. A cycle has no bottom to fold from. The SCC construction treats the group as *one* node of the DAG, whose internal references are positional (`Self(k)`). The canonical order picks one representative among the orderings of the group: the minimum serialisation over all roots. That makes the group's digest invariant under renaming its members, at the price that structurally symmetric members (automorphisms of the group's reference graph) receive equal ids. Two members with equal ids really are interchangeable, so this price is correct, not a bug.

## 2. Why the declared type belongs inside the hash

In a dependent type theory, read semantically in a category with families, a definition is a *section*: a term `t : Γ ⊢ A`. The type `A` is part of the data, not a property of `t`. The same untyped λ-term can inhabit many types, and under Curry–Howard different types are different *theorems*.

So `id : Int → Int` and `id : (A : Type) → A → A` are different definitions, and the hash must tell them apart. An *inferred* type is different: it is a judgement *about* the stored term, made by one checker in one system. Several systems can make incompatible judgements about one term (SR-07). That is why typings are separate L3 facts with provenance, not part of identity.

## 3. Holes are metavariables of contextual modal type theory

A hole `?h : T` written inside a definition stands for a term that is not yet known, of type `T`, *in the context where it occurs*. Contextual modal type theory gives it a type `[Ψ ⊢ T]`, where `Ψ` is the local context at the hole. Filling it is substitution of a closed-over term, and checking a filled hole needs only `Ψ` and `T`. That is why the store keeps the hole's expected type in the hash: two holes with different expected types are different goals.

Moonkale's North Star §6 wants exactly this view: "a hole is a node with an expected type, and filling it is an edit". A checker added later can compute `Ψ` from the term path, so v0 does not need to store it.

## 4. Names are a lens over immutable content

The store has two parts:
- an **immutable** part: terms and definitions, keyed by content;
- a **mutable** part: names, a finite partial map from paths to content ids.

Every user-visible operation (edit, rename, delete) changes only the mutable part, plus an insertion into the immutable part. The printed text of a name is a *view*: `get(store, name) = print(lookup(name))`. An edit is the *put* direction: `put(store, name, text) = store[name ↦ hash(parse(text))]`, plus the new term.

The lens laws are what the projection rules of SPEC-005 §5 demand:
- **GetPut** (printing and then saving the unchanged text writes nothing new) is the round-trip property `parse ∘ print = id`, together with hash-consing.
- **PutGet** (what you saved is what you see) holds when `print ∘ parse` is the identity on what the parser accepts, *up to the projection's normal form*.

A projection that cannot satisfy PutGet for some construct makes that construct display-only. This is the North Star's "a setting that loses information makes the affected part display-only".

Lineage (S-ADR-0003) is the history of `put`s, kept so that the mutable part can be traced. It is a *relation* on contents, not a function, because forks and convergent edits happen.

## 5. One Datalog for queries and for the language's relations

The store's tables are a relational database instance. The v0 SQL views are conjunctive queries over it, and recursion (transitive dependants, reachability) needs a least fixed point. That is Datalog's semantics: the least model of a set of Horn clauses, computed by semi-naive iteration.

Sophia's `relation` and `rule` definitions (SR-06) are also Horn clauses. If both use one evaluator:
- **a query is an unnamed rule**, and a named rule is a stored, content-addressed query;
- the North Star's "logic queries over the graph" (§8) and Lenticulum's statistical logic programs (LT-24 to LT-29) become the same mechanism, applied to two different fact bases: the code graph itself, and the program's own data;
- a weighted rule is a rule over a semiring other than the Booleans: probabilities, or provenance polynomials. Datalog over semirings is the standard generalisation.

This is a design target (SPEC-001 §14, Q4), not v0.

## Related

- [[SPEC-001 Sophia Store v0]] · [[S-ADR-0002 Hash the core term, not the text]] · [[S-ADR-0003 Identity is content, names and lineage]]
- Background cards: [[Merkle DAG]] · [[De Bruijn Index]] · [[Alpha Equivalence]] · [[Cycle Hashing]] · [[Datalog]] · [[Definitional vs Propositional Equality]]
