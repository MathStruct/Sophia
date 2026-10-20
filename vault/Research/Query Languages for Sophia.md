#comparison #design

> Daniel asked (SR-103) for a study of **TypeQL, CQL and Datalog** as inputs to Sophia's own query language, without adopting any of them wholesale. This note compares them, adds the systems closest to Sophia's problem (code queries over a key-value store: CodeQL, Glean, Datomic, Soufflé, egglog), and says what [[SPEC-007 Sophia Query Language]] takes from each. **Short answer:**
> - **Datalog is the core.** Every serious code-query system converges on it.
> - **TypeQL** contributes typed relations with roles and polymorphic queries over a type hierarchy.
> - **CQL** contributes the right model for schema evolution and translation between representations (Σ/Δ/Π), kept separate from semantic equivalence.
>
> Sources:
> - Abiteboul, Hull and Vianu, *Foundations of Databases*, 1995, chs. 12–15.
> - Green, Huang, Loo and Zhou, *Datalog and Recursive Query Processing*, FnT Databases 2013.
> - Avgustinov, de Moor, Peyton Jones and Schäfer, *QL: Object-oriented Queries on Relational Data*, ECOOP 2016.
> - Meta, *Glean* and its *Angle* language (glean.software).
> - Hickey et al., *Datomic* documentation (indexes, Datalog).
> - Jordan, Scholz and Subotić, *Soufflé: On Synthesis of Program Analyzers*, CAV 2016.
> - Zhang et al., *Better Together: Unifying Datalog and Equality Saturation* (egglog), PLDI 2023.
> - Spivak, *Functorial Data Migration*, Information and Computation 2012.
> - Schultz, Spivak, Vasilakopoulou and Wisnesky, *Algebraic Databases*, TAC 2017.
> - Spivak and Wisnesky, *Relational Foundations for Functorial Data Migration*, DBPL 2015.
> - TypeDB, *TypeQL* documentation (TypeDB 3.x).
> - McSherry et al., *Differential Dataflow*, CIDR 2013.
> - Budiu et al., *DBSP*, VLDB 2023.
>
> code: `crates/sophia-query`. Theory (CT-ML wiki): [E-Graph](https://mathstruct.org/CategoryTheory-ML-Wiki/E-Graph).

## 1. The candidates

| | **Datalog** (family) | **TypeQL** (TypeDB) | **CQL** (categorical query language) |
|---|---|---|---|
| Data model | relations (sets of tuples) | entities, relations with **roles**, attributes; a type hierarchy | a schema is a finitely presented category; an instance is a functor into sets (with path equations as constraints) |
| Query | Horn clauses; least fixpoint | pattern matching (`match … fetch`), typed and polymorphic over subtypes; functions (TypeDB 3 replaced rules with functions) | queries are (co)limits and data migrations along schema mappings |
| Recursion | central | through functions | not the focus |
| Negation | stratified (or well-founded) | `not { … }` | via the categorical constructions; not a primitive |
| Optimisation | semi-naive, magic sets, join planning, incremental (DDlog, DBSP) | engine-internal | algebraic: rewrite the migration, then evaluate |
| Theory | first-order logic with fixpoints; with negation and a linear order it captures PTIME | typed hypergraph patterns | category theory: the adjoint triple Σ_F ⊣ Δ_F ⊣ Π_F of data migrations along a schema mapping F |
| Strength for Sophia | recursive code queries (dependants, reachability, dispatch closure, cycles), evaluation strategy | modelling n-ary facts (an annotation with a subject, a checker, evidence) and kind hierarchies | moving data between schema versions and between representations without ad-hoc scripts |
| Weakness for Sophia | untyped classically; roles positional | a whole server and data model to adopt; no content identity | not built for recursion or code-sized graph traversal; few implementations |

## 2. The systems closest to Sophia's problem

**CodeQL (QL).**
- Code is extracted into a relational database, and queried in QL: an object-oriented surface (classes are characteristic predicates, methods are predicates) that compiles to Datalog.
- *Lesson:* a friendly surface over Datalog works for code at scale. Sophia's Julia-flavoured syntax is the same move, with Julia instead of Java-like classes.

**Glean (Meta, the Angle language).**
- Facts about code are stored in RocksDB, a key-value store, and queried in Angle, a Datalog-like language with derived predicates.
- *Lesson:* a key-value store plus a Datalog-family language is a proven architecture for code-scale indexes. That is exactly Sophia's LMDB and redb case. Glean's facts are identified by content within a database, which is close to Sophia's ids.

**Datomic.**
- Datalog over four covering indexes (EAVT, AEVT, AVET, VAET) on a key-value storage service.
- *Lesson:* **the indexes are the access paths.** A Datalog engine needs nothing from the store except ordered lookups by each bound column. SPEC-007 §2 is the same design: each base relation lists the columns it can be looked up by.

**Soufflé.**
- Datalog compiled to parallel C++ for static analysis (Doop's points-to analysis).
- *Lesson:* compiling a query to native code is how Datalog gets fast. That matches "queries as compiled functions" (SR-106), and Sophia's own compiler (M5) could do it.

**egglog.**
- Datalog unified with equality saturation (e-graphs).
- *Lesson:* this is the natural home for **proof-backed query optimisation** and for the program/query convergence of SR-106. Rewrites are equalities, and the e-graph keeps the evidence.

**Differential Dataflow and DBSP.**
- Incremental evaluation of recursive queries under changes.
- *Lesson:* Sophia's change feed is the input stream. Results stay current without re-running (SPEC-007 §11, SP-010).

## 3. What Sophia takes from each

| From | Taken | Where |
|---|---|---|
| Datalog | Horn clauses, least fixpoints, stratified negation, safety, semi-naive evaluation, magic sets | SPEC-007 §3, §6 (implemented) |
| CodeQL | a friendlier surface over Datalog; queries as reusable library code | the Julia-flavoured syntax (§5); stored, named queries (§8) |
| Datomic, Glean | access paths per bound column; Datalog over a key-value store | §2, §6 (implemented on all four backends) |
| TypeQL | **roles** for n-ary relations; queries polymorphic over a **kind hierarchy** | proposed for `store-base-v1`: `annotation(a, kind, subject)` becomes a relation with named roles, and kinds form a hierarchy (`Method <: Definition`) |
| CQL | schemas as categories, data migration as Σ/Δ/Π along a schema mapping | proposed for schema evolution (`store-base-v0` → `v1`) and for mapping a language's normal form into Sophia's (SR-84), always with a separate **semantic** equivalence obligation |
| Soufflé | compiled queries | later: queries lowered to native code by Sophia's own compiler (M5+, SR-106) |
| egglog | equalities with evidence; Datalog plus e-graphs | later: query equivalences as facts, proof-backed rewrites (SPEC-007 §11) |
| Differential Dataflow, DBSP | incremental maintenance over a change stream | later: SP-010, SPEC-007 §11 |

## 4. What CQL does and does not give

The review stressed this, and it is right:
- **CQL's guarantee:** a schema mapping `F: S → T` and its migrations preserve **the schema's structure and equations**.
- **What it does not guarantee:** a Julia function migrated into a Rust-shaped schema still *behaves* the same. Behaviour is a property of the language semantics, not of the database schema.

So Sophia uses CQL ideas for **moving facts between representations**, and keeps **semantic equivalence** as a separate, evidenced relation (SPEC-006, [[Equivalence and Witnesses]]).

## 5. What was not chosen, and why

- **SQL as the definition.**
  - Recursion is limited: SQLite has no mutual recursion in CTEs.
  - Its meaning depends on bag semantics and NULLs.
  - It ties the query language to one storage model.
  
  It stays as a translation target (SPEC-007 §7) and as the `sql` dialect for ad hoc analytics.
- **GQL and Cypher** (property-graph pattern languages). They are good for path patterns, but weaker on recursion with negation and aggregation, and their pattern semantics (trail and walk modes) are complex. Moonkale draws graphs, but Sophia's facts are relational. Path patterns can be sugar over Datalog later.
- **TypeDB itself.** A separate server, data model and runtime, without content identity or Sophia's records. The ideas are taken, not the system.
