#design #definition

> **Draft, 2026-10-20 (Claude).** It restructures the SPEC-001 of 2026-10-17 ("Base Sophia v0"), which the Moonkale agent reviewed that day. Daniel asked on 2026-10-20 to separate **Sophia, the store** from **sophialang, its language** (SR-25). This page is the store only, and it is **language-agnostic**: every language eventually lives here. The language is in [[SPEC-002 Sophialang Principles]] and [[SPEC-003 Sophialang Core v0]]. The boundary Moonkale wraps is [[SPEC-004 Moonkale Interface]]. How code is displayed is [[SPEC-005 Rendering Profiles]].
>
> Sources: Unison's codebase format and hashing ([[Adopting from Unison]]); Moonkale `markdown/architecture/Code Graph Schema.md` (WP-65/66/67, and the WP-18 result), `ADR-0028`. code: `crates/sophia-hash`, `crates/sophia-store`.
>
> Theory (CT-ML wiki): [Relational Lens](https://mathstruct.org/CategoryTheory-ML-Wiki/Relational-Lens) · [Conjunctive Query](https://mathstruct.org/CategoryTheory-ML-Wiki/Conjunctive-Query) · [Least Fixed Point](https://mathstruct.org/CategoryTheory-ML-Wiki/Least-Fixed-Point) · [Attributed C-Set](https://mathstruct.org/CategoryTheory-ML-Wiki/Attributed-C-Set). Reasoning: [[Theory of the v0 Store]].

## 0. Scope

Sophia v0 is a Rust library over an embedded database. It stores **definitions of any language** as canonical bytes, each keyed by a BLAKE3 hash. A language supplies a **normaliser** that turns its definitions into those bytes. The store adds:
- names and namespaces over the hashes;
- nominal entities with versioned relations (method tables);
- lineages;
- annotations and derived facts with recorded inputs;
- a change log;
- read-only queries.

The first language is sophialang ([[SPEC-003 Sophialang Core v0]]). Others follow through frontends (SR-03).

**Non-goals for v0:** execution, checking, compilation, e-graphs, sync, servers. Their structure is fixed below so that v0 does not block them.

## 1. Requirements met

SR-01, 12, 13, 25, 37, 40–43, 45, 48, 54 (static and dispatch cases), 64 in v0. The structure only of SR-07, 08, 10, 11, 35, 36, 44, 46, 47. The Moonkale-specific SR-30 to 34, 38, 39, 49, 50 and 63 are in [[SPEC-004 Moonkale Interface]].

## 2. Layers

Moonkale's layers are L0 text, L1 syntax, L2 definitions, L3 elaborated and L4+ IR. For a **store-backed language** such as sophialang:

| Layer | In the store | Stored? |
|---|---|---|
| L0 Text | a projection, printed on demand through a rendering profile ([[SPEC-005 Rendering Profiles]]) | no |
| L1 Syntax | the language's **normal form**: for sophialang, the core term | yes, as the truth |
| L2 Definitions | definitions, names, namespaces, nominal entities, references | yes: the truth (names) plus a derived index (references) |
| L3 Elaborated | typings, dispatch resolutions, inferred effects, test results | derived, with recorded inputs (§8) |
| L4+ IR | lowered forms, compiled specialisations | derived, with recorded inputs (§10) |

For a **file-backed language** stored later (Rust, Julia, Python, as Moonkale's WP-66 normaliser output; SR-74), L0 stays the truth outside the store. The store holds its normal forms as derived data, with a `source` occurrence. The same tables serve both. Only the `truth` of the language's entries differs.

A store-backed definition differs from a file-backed one in four ways:
- it has no `Defines` edge;
- its id *is* its content id;
- it can be `incomplete`;
- it has a lineage.

## 3. Languages in the store

A **language** is registered with:
- `lang`: an id such as `sophialang`, `rust` or `julia`;
- `normaliser`: a name plus a version;
- its **definition kinds**: shared kinds where they fit, namespaced ones otherwise;
- a **member encoder**: given a definition and the canonical indices of the other members of its recursive group, it writes the canonical bytes (§4.1);
- a **reference extractor**: the content ids and nominal ids the definition refers to;
- a **display codec**: opaque, non-hashed display data, such as binder names and comments.

The store never interprets canonical bytes beyond the tag-length-value framing. **Status:** the `Language` trait is not implemented yet. sophialang is wired in directly (its normaliser produces `Record`s that the store keeps). The trait comes with the second language, so that it is designed from two real cases rather than one.

## 4. Content identity

### 4.1 Framing and digests

**Canonical framing** is shared with Moonkale's WP-66 normaliser v1. A field is a little-endian u32 tag length, the tag bytes, a little-endian u32 payload length, then the payload. A field's payload may itself be a sequence of fields.

**Every definition is a member of a component**, a strongly connected group of the reference graph. A definition that is not recursive is a component of size 1. Then:

```
member_bytes(m)   = language's encoding of m, with references to group members as their canonical index
component_bytes   = field("sophia", "store-v0") ‖ field("lang", lang) ‖ field("normaliser", name@version)
                    ‖ field("component", member_bytes(0) ‖ … ‖ member_bytes(n−1))
component_digest  = BLAKE3(component_bytes)
member_digest(i)  = BLAKE3(field("component", component_digest) ‖ field("index", u32_le(i)))
content id        = member_digest(i)
```

- The **canonical member order** is the WP-66 v1 algorithm. For each candidate root, a first-visit traversal over references in stream order assigns indices, and the lexicographically smallest complete serialisation wins. When several roots tie, each member takes its smallest index across them. Candidates share one work budget.
- **Symmetric groups are quotiented** (2026-10-20, after the external review). Members related by an automorphism (structurally indistinguishable, like `f() = g(); g() = f()`) form one **class**. The component is serialised over its classes, and its stored size is the number of classes. Such members compute the same thing, so they are one definition: `f`, `g` and `h() = h()` share an id. Before this rule, the store kept one record for two "members" and verification failed. Regression tests run on both backends.
- **Shared algorithm, never shared ids:** a Sophia id and a Moonkale id are never equal, because the prefixes differ.
- References **outside** the group are encoded by the referenced content id, so the store is a Merkle DAG ([[Theory of the v0 Store]] §1).
- References to **nominal entities** are encoded by their nominal id (§4.3).

### 4.2 What the language decides

Each language specifies, in its normaliser's specification:
- what is hashed: what is observable in that language;
- what is display-only;
- what goes into annotations.

For sophialang this is [[SPEC-003 Sophialang Core v0]] §3. The rule for all languages: anything that changes the compiled program's behaviour is hashed, and nothing else is.

### 4.3 Nominal entities

Some things are identified by a **fresh identity**, not by content:
- generic functions under multiple dispatch ([[SPEC-002 Sophialang Principles]] §4);
- nominal types: two structurally equal `struct`s that are distinct types ([[Type Taxonomy]] §3).

```
nominal_id = BLAKE3(field("sophia", "nominal-v0") ‖ field("lang", lang) ‖ field("kind", kind) ‖ field("nonce", 32 random bytes))
```

- A nominal entity has **versioned relations**. A generic function's method table is a set of method content ids, and each change is a log entry with a `seq`, like a name change.
- References to a nominal entity are by its id, so adding a method does **not** change the content id of any caller ([[SP-006 Open method tables versus content addressing]]).
- Derived facts that depend on a relation's current state (a dispatch resolution, a test result) record the relation's version (§8).

### 4.4 Incomplete definitions

A language may allow holes and unresolved references. They are hashed (the language decides how), and the definition is stored with `status = incomplete`. Incomplete definitions can be named, drawn, printed and edited. They cannot be lowered, run or used as dependencies of complete builds.

### 4.5 Versioning and migration (SR-13)

The framing version (`store-v0`) and the normaliser version are both in every component's bytes. A change to either gives new ids, never silently equal ones.

Migration:
1. re-encode every definition under the new version, in dependency order;
2. write `migrated_to(old, new, version)`;
3. move the names;
4. re-point annotations, marking proofs `stale`.

Old ids keep resolving through the table. "Replace the core and migrate" (SR-13) is this operation with a different normaliser.

## 5. Names, namespaces and handles

### 5.1 Names

- A **name** is a path, `lenticulum.factor.gaussian`. It points at **one** target: a content id or a nominal id.
- A **namespace** is the set of names under a prefix. It is not a definition.
- Several names can point at one target (aliases). A re-export is an alias with a `visibility`.
- **Rename** moves a name; no id changes.
- **Update** points a name at new content. Dependants are not updated automatically in v0, but the store reports them ([[SP-002 Propagating an update to dependants]]).
- **Delete** removes a name. The definition stays. `gc` removes definitions that are neither named nor reachable from named ones, on request only.

### 5.2 Log, history, branches

Every change to a name, a nominal relation or a lineage is a row in an append-only **log**: `seq, op, subject, old, new, actor, time`. That log is the history and the change feed (§7).

Branches are later. A branch is a named log, and a namespace snapshot can be hashed as a Merkle tree of `(path, target)` pairs, which is Unison's causal namespace ([[Adopting from Unison]]).

### 5.3 Three handles

| Handle | Stable under | Changes on | For |
|---|---|---|---|
| content id | moves, renames, display changes | any change in meaning | proofs, caches, equality |
| name | edits (the name moves) | rename | navigation, links, tabs |
| lineage | edits and renames | never; it only grows | comments, history, a tab that survives renames |

- An edit through a name adds the new content to the lineage of the old content, with a `Supersedes` link.
- A copy that is then changed starts a new lineage with `ForkedFrom`.
- Lineages are many-to-many ([[S-ADR-0003 Identity is content, names and lineage]], [[SP-005 Lineages under merge and fork]]).

## 6. Projections

The store holds no text. Each language registers printers and parsers for its **rendering profiles**. Profiles and the user's choice of profile are **stored in the database** ([[SPEC-005 Rendering Profiles]], SR-28).

The store guarantees what a projection needs from it:
- display data per definition, outside the hash;
- source maps from text spans to term paths;
- writes that go through parse → normalise → hash, so that a projection can never write anything other than a definition.

## 7. The change feed

- `changes_since(seq)` returns the log rows after `seq`, plus definitions and annotations added after `seq`, including unnamed ones.
- `seq` is a u64 that increases per store. It is the `Version` of names and nominal relations.
- Polling in v0. A push channel later, driven by the backend's commit hook.

## 8. Annotations and derived facts

There are two kinds of non-hashed statement **about** definitions. Both are content-addressed themselves.

```
annotation {                         derived fact {
  id        -- BLAKE3 of the fields    id        -- BLAKE3 of the fields
  kind      -- doc | comment |          kind      -- typing | dispatch | effect | test-result |
               equivalence | proves:*                compiled | boxed | …
               | unsafe:* | …           subject   -- content id or (content id, term path)
  subjects  -- content ids / paths      value     -- the result (a term, a verdict, a blob id)
  payload   -- text or evidence         by, by_version -- the tool that derived it
  checker, checker_version             inputs    -- content ids + (nominal id, relation version) pairs
  status    -- asserted | checked |     derived_at
               failed | stale         }
  inputs    -- what a check used
}
```

- **An annotation is something a person, an agent or a tool *claims***: docs, proofs, `unsafe` marks, equivalences.
- **A derived fact is something a tool *computed***, reproducibly from its inputs: a typing (one per type system, SR-44), a dispatch resolution, an inferred effect row, a test result, a compiled specialisation.
- **Staleness:** a derived fact is **stale** when one of its inputs no longer holds. Either a content id is no longer what the subject's names resolve to, or a relation's version moved and the change can affect it. A fact about immutable content is never *wrong*, only possibly irrelevant.
- **Test results (SR-54):** a test re-runs only when its result is stale. The static case is free, because the test's content id covers its static dependencies. The dispatch case uses the recorded relation versions: a new method affects a test only if its signature overlaps a recorded call signature. The equivalence case uses proofs ([[SPEC-002 Sophialang Principles]] §8).
- **Performance and identity (SR-53, SR-79):** annotations and derived facts never feed back into a definition's bytes. Compiled **artifacts** may depend on facts and evidence, but each artifact records exactly which ones in its identity ([[SPEC-006 Worlds, Artifacts and Evidence]] §3).

## 9. Typings

Typings are derived facts of kind `typing`, carrying `system` (for example `sophialang-simple`, `sophialang-dependent`, `cubical`) and `status`. There are several per definition, never merged. Moonkale draws each one as a shared `TypeOf` edge from the definition to its type's node, with the fact's provenance as edge properties ([[SPEC-004 Moonkale Interface]] §2).

## 10. E-graphs and IR (later)

- **E-classes** are derived facts with member content ids. Rewrite rules are definitions (`sophia:rewrite`). A rewrite step is a derived fact `rewrote(from, to, rule)`.
- **IR units** (a lowered function, an MLIR region, an LLVM function) are content-addressed definitions of languages `mlir:*` and `llvm`. `lowered(core id → IR id)` is a derived fact naming the lowering tool and its version.
- Whether IR is persisted or regenerated is decided by measurement ([[Benchmark Plan]]).

## 11. Queries

- **v0: `sql`, read-only**, over documented views. The result is a table, plus nodes for id columns.
  - `definitions(id, lang, kind, status, normaliser)`
  - `names(path, target_id, visibility, seq)`
  - `refs(from_id, to_id)`
  - `nominals(id, lang, kind)`
  - `relation(nominal_id, member_id, seq)`
  - `lineage(lineage_id, id, seq)`
  - `annotations(id, kind, subject_id, status)`
  - `facts(id, kind, subject_id, by, stale)`
  - `log(seq, op, subject, old, new)`
- How it is enforced read-only is in [[SPEC-004 Moonkale Interface]] §5.
- **Later: `datalog`**, evaluated by Sophia (semi-naive), over the same relations. The design target (Q4) is for it to be the same Datalog as sophialang's own relations and rules ([[Theory of the v0 Store]] §5).

## 12. Storage backends (SR-17, 39, 64)

The store is a trait, `sophia_store::Backend`. Its operations are:
- put a component (atomic);
- get a definition;
- set, move and remove a name, with an optimistic check against `seq`;
- add to a nominal relation;
- append to the log;
- put annotations and facts;
- read the log since `seq`;
- scan by kind;
- reference lookups in both directions;
- run a read-only query in the backend's dialect.

**Several backends are implemented and compared** (SR-64, [[Database Comparison]]):

| Backend | Why it is in the comparison |
|---|---|
| **SQLite** (rusqlite, bundled) | the reference: embedded on every platform, transactional, SQL views; Moonkale already ships it |
| **redb** | pure-Rust embedded key-value store; no C toolchain; a test of whether a typed key-value layout beats SQL for hash lookups |
| **DuckDB** | columnar analytics: the precompilation statistics (SR-66) and bulk scans |
| **LMDB** (via heed) | memory-mapped B+tree, very fast reads; the classic choice for content-addressed stores |
| **LadybugDB** (Cypher) | a real graph query language; Linux only for now (Moonkale P-144), so not the default |

The same conformance tests and the same benchmark suite run on every backend ([[Benchmark Plan]]). Which one is the default is decided by those numbers. Until then SQLite is the default, because Moonkale and every platform can use it.

The SQLite layout (v0):

```sql
definition(id BLOB PRIMARY KEY, lang TEXT, normaliser TEXT, kind TEXT,
           component BLOB, idx INT, bytes BLOB, display BLOB, status TEXT)
component(digest BLOB PRIMARY KEY, size INT)
ref(src BLOB, dst BLOB, PRIMARY KEY(src, dst))            -- derived from bytes
nominal(id BLOB PRIMARY KEY, lang TEXT, kind TEXT, created_seq INT)
relation(nominal BLOB, member BLOB, added_seq INT, removed_seq INT)
name(path TEXT PRIMARY KEY, target BLOB, visibility TEXT, seq INT)
lineage(lineage BLOB, id BLOB, seq INT, supersedes BLOB, PRIMARY KEY(lineage, id))
annotation(id BLOB PRIMARY KEY, kind TEXT, subjects BLOB, payload BLOB,
           checker TEXT, checker_version TEXT, status TEXT, inputs BLOB)
fact(id BLOB PRIMARY KEY, kind TEXT, subject BLOB, value BLOB, by TEXT, by_version TEXT,
     inputs BLOB, derived_at INT)
log(seq INTEGER PRIMARY KEY, op TEXT, subject TEXT, old BLOB, new BLOB, actor TEXT, at INT)
profile(...) , setting(...)                                -- SPEC-005
migrated_to(old BLOB, new BLOB, version TEXT)
meta(key TEXT PRIMARY KEY, value TEXT)
```

## 13. Acceptance

1. **Conformance tests**, run on every backend:
   - re-putting a component writes nothing;
   - rename keeps the content id and lineage;
   - an edit gives a new id, extends the lineage and reports dependants;
   - a recursive group shares a component;
   - an incomplete definition stores;
   - adding to a nominal relation changes no definition id, and marks dependent facts stale;
   - a test result is reused when nothing it recorded changed;
   - `changes_since` returns exactly the new log rows;
   - the read-only query rejects writes.
2. **Benchmarks** on generated stores of 10³ to 10⁶ definitions ([[Benchmark Plan]]).
3. [[SPEC-004 Moonkale Interface]] §9 (WP-68) passes on the default backend.

## 14. Open questions

| # | Question | Proposal |
|---|---|---|
| Q4 | One Datalog for queries and for sophialang's relations? | Yes, as the target |
| Q5 | Is lineage worth having in v0? | Yes. The Moonkale agent wants it for comments |
| Q6 | Share a crate with Moonkale for framing and the SCC algorithm? | Later |
| Q7 | One store per project, per user, or one for everything? | Per project in v0. A user-level store for rendering profiles can be added (SPEC-005 §2) |

## Related

[[SPEC-002 Sophialang Principles]] · [[SPEC-003 Sophialang Core v0]] · [[SPEC-004 Moonkale Interface]] · [[SPEC-005 Rendering Profiles]] · [[Requirements]] · [[Milestones]] · [[Database Comparison]]
