#design #definition

> **Draft, 2026-10-20 (Claude), from Daniel's forwarded review of the same day ([[Review 2026-10-20 Query Language (external)]]).** Sophia has **its own query algebra and query language**. A query is stratified Datalog over the store's base relations. It has a Julia-flavoured syntax, a canonical form and a content id, and it is stored in the graph like any definition. Its results are identified by (query, snapshot, parameters). It runs on **every backend** through the backends' access paths, and on SQLite also as translated SQL. SQL is a translation target, not the definition.
>
> It answers SR-90 to SR-106 and re-scopes SR-32 (the `datalog` dialect becomes the `query` dialect). It follows [[S-ADR-0011 Sophia has its own query algebra]] and [[S-ADR-0012 The query IR is a separate IR on shared primitives]].
>
> Sources: Abiteboul, Hull and Vianu, *Foundations of Databases*, 1995 (Datalog, stratified negation); Bancilhon et al., *Magic Sets*, PODS 1986; Avgustinov et al., *QL: Object-oriented Queries on Relational Data*, ECOOP 2016 (CodeQL); Spivak and Wisnesky, *Relational Foundations for Functorial Data Migration*, DBPL 2015 (CQL); TypeDB, *TypeQL* documentation; [[Query Languages for Sophia]]. code: `crates/sophia-query` (implemented, see §12).

## 1. Purpose and principles

Daniel's ten points (SR-90 to SR-100), and how this specification meets each:

| # | Principle | Here |
|---|---|---|
| 1 | Queries are first-class statements stored in the graph | §8: a query is a record of the language `sophia-query` |
| 2 | Canonical representation, stable content ids, compressible physical encoding | §4.1: canonical form; the id uses the store's rule (SPEC-001 §4); bytes are WP-66 framing |
| 3 | Proofs, optimisations, compiled forms and benchmarks attach to queries | §8: annotations and facts attach to a query id like to any definition |
| 4 | Results identified by snapshot and parameters | §4.3 |
| 5 | Several syntaxes, Julia-like by default | §5: `julia` (parses and prints), `datalog` (prints) |
| 6 | Semantics independent of storage | §2, §3: defined over base relations; §6: backends supply access paths |
| 7 | Eventually implementable in sophialang | §9 |
| 8 | Planning, optimisation, incremental maintenance are extensible | §6: a planner and an IR → IR rewrite (magic sets); incremental maintenance is §11 |
| 9 | Query semantics stay apart from expression evaluation | §3.4 |
| 10 | Moonkale and agents use the same typed interface | §8: the `query` dialect; the MCP server (M3b) will call the same API |

## 2. Base relations (schema `store-base-v0`)

The extensional database. Every backend answers these. The schema name is hashed into every query id, so a change to this table gives new ids.

| Relation | Columns (sorts) | Meaning | Access paths (columns that can be looked up) |
|---|---|---|---|
| `def(d, kind)` | Id, Text | a definition and its kind | `d` (`kind_of`), `kind` (`records_of_kind`); scan `all_kinds` |
| `refs(a, b)` | Id, Id | `a` references definition `b` directly | `a` (`refs_of`), `b` (`dependants`); scan `all_refs` |
| `name(path, t)` | Text, Id | a name and its target (definition or nominal) | `path` (`name`), `t` (`names_of`); scan `names_under("")` |
| `member(n, d)` | Id, Id | `d` is a current member of nominal `n` (a method, a relation's rule, a library's axiom) | `n` (`relation`, wide), `d` (`memberships_of`); scan over `nominal_ids` |
| `nominal(n, kind)` | Id, Text | a nominal entity and its kind | `n` (`nominal`); scan over `nominal_ids` |
| `uses(d, n)` | Id, Id | `d` references nominal entity `n` | `d` (the record); scan over records |
| `annotation(a, kind, s)` | Id, Text, Id | annotation `a` of a kind on subject `s` | `a` (`annotation`), `s` (`annotations_of`); scan over subjects |
| `fact(f, kind, s)` | Id, Text, Id | derived fact `f` of a kind about `s` | `s` (`facts_of`); scan over subjects |

The access paths are `ReadTx` methods. A backend overrides one when it has a better index (SQLite answers `refs_of` from `t_ref` without decoding records; the key-value backends scan their `kind` and `ref_rev` tables for `all_kinds` and `all_refs`). **The algebra depends on neither SQL nor key order**, only on these methods (SR-96).

Not yet included: the log, lineages, names' versions, record sizes and statuses. Adding them is a schema change (`store-base-v1`).

## 3. The IR

### 3.1 Terms and literals

- **Sorts:** `Id` (32 bytes), `Text`, `Int`.
- **Terms:** a variable (numbered per rule), a parameter of the query, a constant.
- **Literals:**
  - `p(t…)`, a positive atom over a base relation or a local predicate;
  - `!p(t…)`, negation as failure;
  - `a == b` and `a != b`;
  - `startswith(a, b)` on text.

### 3.2 Rules and select

- A **rule** `head(t…) = l₁ && … && lₙ` adds tuples to a local predicate. Several rules for one predicate are a union. The order of rules does not matter.
- A query has exactly one **select** `select o₁, …, oₖ where body`.
- An output is a variable or `count(v)`, the number of distinct values of `v` per group. The group is the tuple of plain outputs. With no plain outputs, an empty body counts to one row of zeros, as in SQL.

### 3.3 Semantics and restrictions

- **Set semantics.** A local predicate denotes the least fixpoint of its rules, computed stratum by stratum.
- **Stratified negation.** No predicate may depend on itself through a negation. This is checked (`check`).
- **Safety (range restriction).** Every head and output variable is bound by a positive literal. Variables that occur only inside one negated literal are existential: `!refs(_, d)` means "nothing references `d`".
- **Sorts** are inferred by unification and must agree. The result type is the list of output sorts.
- **Bounds** (SR-50): 64 KiB of text, 256 rules, 64 literals per body, 16 arguments.

### 3.4 Queries are not sophialang expressions (SR-99)

`x + y == 10` in sophialang computes a Boolean once `x` and `y` have values. In a query, `x == y` is a **constraint** that may bind a variable. The query language has its own semantics (least fixpoints over finite relations), so it is a **separate typed DSL**, not an interpretation of arbitrary sophialang expressions. Arithmetic, user functions and dispatch are not in v0. They enter later as **pure functions with bound arguments** (like `startswith`), never as unification over function results: §13 (proposal).

A query has a type `Query{S, A, E}`:
- `S`: the base schema (`store-base-v0`), part of the id;
- `A`: the output sorts;
- `E`: `read`.

Every v0 query is read-only. Mutating queries would be a different effect with transaction semantics, and are not planned.

## 4. Identity

### 4.1 Canonical form

Normalised away:
- **names** of the query, its parameters, predicates and variables (stored as display data, like sophialang's names);
- **formatting and comments**;
- **predicates unreachable** from the select;
- **the order of predicates** and **the order of rules**;
- **exact duplicate rules**.

**The definition (exact since 2026-10-20, after the M3a review).** The canonical form is the **smallest encoding over every valid traversal**. A traversal:
1. numbers variables by first occurrence: outputs, then head, then body;
2. numbers predicates by first mention, starting from the select's body in literal order;
3. visits predicates in that order and lists each one's rules sorted by their bytes, with not-yet-numbered predicates masked;
4. lets rules that tie under the mask (they differ only in which unnumbered predicates they call) come in any order, and tries every such order.

So for every permutation π of predicates and rules, every renaming, every duplicated rule and every added unreachable predicate, `canon(πQ) = canon(Q)`. This is the same construction as recursive groups in `sophia-hash` (minimum over roots, SPEC-001 §4.1).

The search is exhaustive within a budget (`CANON_BUDGET` = 4096 traversals). A query more symmetric than that is **refused**, never given an unstable id.

Tested by property tests:
- 2000 random queries (20 000 in a stress run) under random predicate permutation, rule shuffling, duplication, unreachable predicates and renaming;
- plus decode(encode) and idempotence.

**Not normalised: the order of literals inside a body,** nor commutations of `==` and `!=`. Two bodies that differ only in literal order get different ids, just as `f(a, b)` and `f(b, a)` do in code. The reasons:
- full canonisation of conjunctive queries is graph canonisation;
- a quotient would cost the readability of the stored form.

Equivalences between such queries are **facts with evidence** (§8), as for programs ([[SP-009 Canonical form of queries]]).

### 4.2 Bytes and id

The canonical bytes use WP-66 tag-length-value framing:
- `sophia: query-v0`, `schema: store-base-v0`;
- the parameter sorts;
- per predicate: its arity and its rules' bytes;
- the select.

The **query id** is `member_digest(component_digest("sophia-query", "sophia-query@0", bytes), 0)`. That is the store's identity rule for a one-member component (SPEC-001 §4). So a stored query verifies with `Record::verify` like any definition.

### 4.3 Result identity and caching

`ResultId = H("query-result-v0", query id, snapshot, parameter values)`. Two runs with the same result id must return the same rows.

**Snapshot v1** (2026-11-18, B-016) = H(store instance id, the epoch of each base relation the query reads). The instance id is a random nonce kept in meta. A relation's epoch is a log seq at or after its last logged change and at or before the last seq, kept in meta and written in the transaction of the change; a store without epochs reads the last seq. The relations a query reads are those its rules and select mention, plus `def` and `nominal` with `annotation` or `fact` (their scans enumerate subjects through them). So a profile switch moves no snapshot, and an annotation moves only the snapshots of queries that read `annotation`. v0 was H(store instance id, last log seq), which any logged change moved. Still open: read sets of keys rather than relations, and a content-addressed snapshot of names and worlds (SPEC-006 §2) ([[SP-010 Query snapshots are coarse]], steps 2 and 3).

`run_cached` (corrected 2026-10-20 after the M3a review):
- **One snapshot.** The snapshot id, the cache lookup and, on a miss, the evaluation run inside **one read transaction**. So rows stored under a result id were computed from exactly that snapshot. The first version used two transactions, and could store rows of a newer state under an older snapshot's id.
- **Semantic result versus execution policy.** The result id is semantic: H(query, snapshot, parameters). Only **complete** results are cached, so a cached result is the answer regardless of the limits it was computed under. The caller's row cap is applied when the result is returned, with `truncated` set as if it had been evaluated. Step, tuple, byte and time limits are resource limits: a hit costs less than any of them. If execution policy ever changes the answer (sampling, approximate evaluation), it becomes part of an **execution id** next to the result id; nothing does today.
- Cached rows live in meta rows, which are not logged, so writing them does not move the snapshot. Truncated and failed runs are not cached.

## 5. Syntax

**The default profile, `julia`,** parses and prints. `print(parse(t))` is a fixed point, and parsing the printed text gives the same IR (tested on every construct):

```julia
query transitive_dependants(target::Id)
    # direct and transitive users of `target`
    dep(a, b) = refs(a, b)
    dep(a, c) = refs(a, b) && dep(b, c)
    select a where dep(a, target)
end
```

A bare `select … where …` is an anonymous query without parameters or rules:

```julia
select k, count(d) where def(d, k)
select p where name(p, d) && startswith(p, "core.") && !refs(_, d)
```

Constants:
- `"text"`;
- `:Symbol` (also text; the printer uses it for identifier-like text);
- integers;
- `0x` followed by 64 hex digits for an id.

`_` is a fresh variable at each occurrence. Rules use `=` because a rule reads like a method definition: several rules for one name add up, as Julia's methods do.

**The `datalog` profile** prints only. Variables are capitalised, parameters are `$name`, a fact is `head.`, and the select is `?- select(…) :- body.`. It is also how `explain` shows the program the executor runs.

Profiles for queries live beside the code profiles (SPEC-005). Choosing the query profile by the reader's setting is part of §8's open work.

## 6. Execution

### 6.0 The native executor

The native executor runs on every backend, inside one read transaction:
- **Strata** run in dependency order. Recursive strata use **semi-naive** iteration: each round, every rule runs once per recursive literal, with that literal reading only the last round's new tuples.
- **Base literals** go through an access path when a lookup column is bound (memoised per run), through a **prefix range** for `name(p, _)` with a bound `startswith(p, s)`, and through a full scan otherwise.
- **Negated base literals** are probed per solution; after 256 probes on one column set, the executor scans the relation once and probes a hash set of its projection (an **anti-join**), when the literal's unbound arguments are distinct variables (B-014).
- **Local relations** keep hash indexes on the column sets the plans look them up by.
- **The resource budget (`Limits`):**
  - join steps;
  - a row cap: the select stops early and marks the result truncated;
  - derived tuples (5 M by default): a logical limit;
  - **estimated bytes** (1 GiB by default): derived tuples with their list, set and index copies, memoised and scanned base tuples, and select rows, estimated from value sizes;
  - a **wall-clock deadline** (30 s by default);
  - a **cancel flag** (`run_cancellable`) that another thread can set.
  
  The step budget alone did not bound memory: without magic sets, a closure over 10⁵ definitions passed 14 GB before reaching 50 M steps. The byte budget is an **estimate, not a guarantee**: it counts what the executor keeps, not every temporary or allocator overhead. **The hard bound** for untrusted queries is a separate process (B-017, Daniel's choice on 2026-11-18): `bounded::run_bounded` runs the query in `sophia-query-worker` under `RLIMIT_DATA`, and going over it returns `Error::MemoryBound` without harming the caller ([[bounded]]). Linux only. The worker opens the store read-only; redb and DuckDB lock their file, so it can read them only while no other process holds them for writing. The agent server (M3b) is to run untrusted queries this way.
- `Stats` report lookups, scans, derived tuples, rounds and estimated bytes.

### 6.1 Planning

A greedy planner orders each body:
1. filters as soon as their variables are bound;
2. then equations that bind;
3. then base literals with an access path on a bound column;
4. then local literals with bound arguments;
5. full scans last.

A `name(p, _)` with a bound `startswith(p, s)` in the body is a prefix range and ranks like a wide lookup (B-014).

The same planner decides the sideways information passing of §6.2.

**Selectivity classes (2026-10-20, from set.mm).** An access path is *narrow* (by id or path) or *wide*: every definition of a kind (`def(_, k)`), every member of a collection (`member(c, _)`). Wide lookups come after local literals, bound or free. A free local predicate is computed once from the query's parameters, while binding a goal from a wide lookup makes magic sets search backwards from every match. On set.mm that was the difference between 2 ms and over 30 s. This is a static guess; cost-based planning replaces it.

This is the extensible point (SR-98). A cost-based planner using backend statistics replaces `plan::order` without touching the IR.

### 6.2 Magic sets: an optimisation as IR → IR

**The problem.** Bottom-up evaluation of `dep(a, target)` computes `dep` for the whole store before filtering. That is the transitive closure of every definition: O(n²).

**The fix.** The magic-set rewrite turns the query into another query:
- each predicate called with bound arguments gets an adorned copy and a magic predicate holding the values asked about;
- evaluation then explores only what the target reaches.

**Its result is a query again.** So:
- both executors run it;
- the SQL translation becomes goal-directed too;
- its equivalence to the original is a checkable claim, tested today and provable later.

**Negation:**
- Negated calls with bound arguments get magic sets too (since 2026-10-20). Without them, `!proved(a)` computed `proved` for all 64 000 proofs of set.mm.
- Negations are dropped from magic-rule bodies, which only enlarges the magic set and is sound.
- If the result is not stratified, the rewrite is retried with negated calls unadorned. If that fails too, the query runs as written.
- The differential property test covers all three cases.

`Strategy::Auto` uses the rewrite when it applies; `Strategy::Plain` runs the query as written. B-17 measures the difference: about 0.17 ms against 6 s per probe at 10⁴ definitions ([[Benchmark Results]]).

## 7. SQL translation

`to_sql` translates a program (plain or magic) into one SQLite statement:
- `WITH RECURSIVE`, with one CTE per local predicate, in dependency order;
- positive literals become `FROM` items over the base tables, and shared variables become equalities;
- negation becomes `NOT EXISTS`;
- `count` becomes `count(DISTINCT …)` with `GROUP BY`;
- parameters are inlined as literals.

It runs through `Sqlite::query_compiled`:
- it may read the base tables `t_*`, so their indexes apply;
- it keeps the authorizer (only `SELECT`, no writes, no `sqlite_master`) and the step budget;
- it is not for user text: user SQL still goes through the view-restricted `sql` dialect.

**What SQL cannot express is refused,** not approximated: mutual recursion and non-linear recursion return `Unsupported`. The native executor runs those queries. This is the concrete sense of "SQL is a target, not the definition" (SR-90).

DuckDB has no translation yet; it uses the native executor. A DuckDB translation (§11) would make it the analytics engine for queries too.

## 8. Queries in the graph, in Moonkale, and for agents

**Stored.** `save(store, query, name)` stores the query as a record:
- lang `sophia-query`, kind `Query`;
- its id constants that are definitions or nominals become `refs` and `nominal_refs`, so a query **is a dependant** of what it names;
- `load` reads it back by id or name and checks that it is canonical.

Queries are queryable themselves: `select q where def(q, :Query)`.

**Attached.** Annotations and facts attach to a query id like to any definition:
- a proof that two queries are equivalent;
- a compiled plan for a backend (an artifact, SPEC-006 §3);
- benchmark results;
- a cached result: v0 keeps it in meta (§4.3); a fact form comes with precise snapshots.

**Moonkale (SPEC-004).**
- **The `query` dialect.** `Interface::query(text, limits)` returns the table and a subgraph of the definitions in id cells, on every backend. `classify("query")` is `ReadOnly`.
- **Creating.** Text that starts with `query` or `select`, created under a namespace, is stored as a query under that name.
- **Reading and editing.** Fetching a query's name prints it in the query syntax. Writing it re-parses it, checks the edit token, and moves the name to the new query id, like any edit. Parse errors are diagnostics with byte positions.

**Agents** reach the same language through the MCP server's `query` tool (`sophia-mcp`, B-062, [[sophia_mcp]]), not through backend SQL (SR-100). Every agent query runs in a worker under a hard memory bound ([[bounded]]). Agents store queries with `save_query` (behind `--allow-edits`) and run them by name with parameters through `run` (B-063).

## 9. Relation to sophialang ([[S-ADR-0012 The query IR is a separate IR on shared primitives]])

**Now: a separate IR on shared primitives.** It shares:
- WP-66 framing and the canonical writer;
- the component and member digest rule;
- Tarjan's components (used for strata);
- records, names, lineage and the change log;
- the rendering-profile approach.

Its own are the evaluation semantics (fixpoints, not reduction) and the sorts.

**Later: also a quoted DSL node in the core.** A sophialang term will be able to contain a query as a value, something like `Query{S, A}` with the query IR as payload. sophialang code can then build, pass, compose and run queries. The IR stays the meaning; the node is how programs hold one.

**Self-hosting (SR-97).** The parser, canonicaliser, planner, magic-set rewrite and executor are ordinary code over finite maps. Writing them in sophialang is a milestone-6 target, after the interpreter (M4). The Rust version is the reference.

## 10. Relation to TypeQL, CQL and Datalog

Summarised from [[Query Languages for Sophia]]:

- **From Datalog:** the core: recursion, stratified negation, semi-naive evaluation, magic sets. CodeQL and Glean show that Datalog-family languages are what code-analysis systems converge on.
- **From TypeQL:**
  - typed n-ary relations with **roles**: a candidate for `store-base-v1`, where `annotation(a, kind, s)` would become a relation with named roles;
  - polymorphic queries over a type hierarchy: kinds as a hierarchy rather than flat text.
- **From CQL:**
  - schemas as categories, and queries as functors;
  - **Σ/Δ/Π migrations** as the model for moving data between schema versions (`store-base-v0` → `v1`) and between language normal forms, kept separate from semantic equivalence, which needs its own evidence (SR-84).

## 11. Open

- **Precise snapshots** (SP-010): per-relation epochs are done (§4.3, B-016); content-addressed name and world snapshots and key-level read sets are not.
- **Incremental view maintenance:** semi-naive evaluation is already the delta rule; DBSP or differential dataflow keep results up to date as the change feed arrives.
- **Cost-based planning** from backend statistics, and a **DuckDB translation**.
- **Aggregates** beyond `count` (`sum`, `min`, `max`); arithmetic is a query function (§13.8).
- **Query functions** (review of M3a, §6; SR-111): designed in §13 (proposal, 2026-11-18): one mechanism for typed, pure, deterministic, cost-annotated functions called with bound arguments, replacing the `startswith` literal; sophialang functions join it after M4. Not built (B-067 to B-070).
- **More base relations:** log, lineage, sizes, statuses; roles (TypeQL).
- **Proof-backed rewrites:** query equivalences as facts with evidence (e-graphs, as in egglog), so that an optimiser's rewrite needs no trust in the optimiser.
- **Queries as compiled functions,** partial evaluation against database state, materialised views as reusable computations (SR-106). This waits until the small algebra works on at least two backends, which it now does.
- **Zero-copy reads** ([[Zero-Copy Storage of Canonical Graphs]]).

## 12. Implementation status (2026-10-20)

`crates/sophia-query`:
- the parser, the printers (`julia`, `datalog`), canonical form and ids;
- the checks: sorts, safety, stratification;
- the planner, the magic-set rewrite, the native executor, the SQL translation;
- result ids and caching; storing and loading queries.

`sophia-interface`: the `query` dialect; stored queries print and edit through `fetch_text` and `write_text`.

`sophia` CLI: `query Q [ARG..] [--explain|--sql|--plain|--save NAME]`.

Tests: 11 identity tests, an execution suite, and property tests.
- **Property tests (after the M3a review):**
  - canonical-form invariance on 2000 random queries;
  - a differential test of magic sets on random safe programs (repeated variables, mutual recursion, stratified negation, several bound arguments, `count`): plain, magic and SQL evaluation agree. The default run checks 719 valid programs, 476 of them rewritten, in 3051 SQL runs; a stress run checked 9561 programs.
  - It found one bug: SQLite reports a read of a CTE that a rule uses without reading a column of it, and the compiled-SQL guard refused it. The guard now also allows the translation's CTE names.
- **Execution suite:** plain and magic programs agree on every backend; SQL agrees with native for both programs; reach matches a graph walk; magic sets do no full scans; caching per snapshot; a change outside a query's relations keeps its cached result (`changes_elsewhere_keep_cached_results`); stored queries are definitions and dependants; step limits.
- **Interface:** an acceptance test on every backend.

Benchmarks: B-17 in [[Benchmark Results]].

## 13. Query functions (proposal, 2026-11-18, B-019)

**A design by Claude for SR-111, not yet accepted by Daniel and not built.** It answers point 6 of [[Review 2026-10-20 M3a (external)]]: rather than adding a special literal for each built-in (today there is one, `startswith`), queries call **typed, pure, cost-annotated functions through one mechanism**. Everything here is a proposal; nothing in it has been measured.

### 13.1 What a query function is

A query function is `f: A₁ × … × Aₙ → B`, where each `Aᵢ` is a sort (`Id`, `Text`, `Int`) and `B` is a sort or `Bool`. A `Bool` function is a **test**; any other is a **map**. Every function carries a **signature record**:

| field | meaning | used by |
|---|---|---|
| `name` | its name in the query syntax (`startswith`, `length`) | parser, printers |
| `params`, `result` | the sorts above | `check` (sorts) |
| `partial` | whether it is undefined on some inputs (`div` by zero, overflow) | executor, SQL translation |
| `cost` | a cost class: `Const` (O(1)), `Linear` (O(size of the arguments)), `Heavy` (anything else, with a declared bound in steps per call) | planner, budget |
| `recursion` | `Safe` or `Inventive` (defined in §13.4) | `check` (termination, §13.4) |
| `sql` | an optional SQLite expression template that computes exactly the same value | SQL translation (§13.6) |

**Required of every function, not declared:** it is **pure** (no I/O, no mutation, no reading of the store, no clock, no randomness) and **deterministic** (the same arguments give the same result on every platform and in every run). The signature does not offer an "impure" setting, because an impure function would break result identity: `ResultId = H(query, snapshot, parameters)` (§4.3) is only sound if a query's answer is fixed by the store's state and the parameters. Anything that reads the store (a record's size, its status) is a **base relation** and enters through a schema version (`store-base-v1`), never as a function.

Determinism rules out floating point whose result depends on the platform, and Unicode operations whose result depends on a Unicode version (case folding, normalisation) unless the version is part of the function's name. Text functions work on bytes or on Unicode scalar values, and each one says which.

### 13.2 In the IR and its identity

`Literal::StartsWith` is replaced by one literal:

```rust
Literal::Call { f: FnRef, args: Vec<Term>, out: Option<Term>, neg: bool }
```

- a **test** has `out = None`; `neg` negates it (`!startswith(p, "core.")`);
- a **map** has `out = Some(t)` and `neg = false`: `t == f(args…)`. If `t` is an unbound variable the call **binds** it; otherwise the call checks it.

`FnRef` names a function:
- `Builtin(name)`: a function of Sophia's built-in library, versioned as a whole (`sophia-fn-v0`);
- `Def(id)` (later, §13.7): a stored sophialang definition, by content id.

**Identity.** The canonical bytes become `query-v1`: the header adds `functions: sophia-fn-v0` next to `schema: store-base-v0`, and a call is encoded as tag `call`, the reference (a builtin's name, or a definition id), `neg`, the arguments and the output. So a change to the meaning of any built-in function is a new library version and gives new query ids, as a schema change does (§2). The library version is in every query's header, whether or not it calls a function, so that one rule fixes the meaning of every name.

This changes every query id: a **migration**, carried out as SPEC-006 §6 says: `migrated_to(old id, new id, query-v0, query-v1)` for every stored query, a new world, and facts or proofs on the old ids marked stale. `decode` keeps reading `query-v0` (its `startswith` tag becomes a call of the builtin), and `load`'s canonicity check is made version-aware (today `canon::decode` refuses anything but `query-v0`), so a v0 record still loads and runs. Nobody uses Sophia yet (SR-118), so the migration can run once per store when B-067 lands. Cached results are keyed by query id and simply miss.

A `Def(id)` reference is to be a `ref` of the stored query, so that a query **is a dependant of the functions it calls** (§8), as it is of the definitions it names. Today `Prepared::record` takes refs from `canon::constants`, which sees only id constants in terms; B-070 must add `FnRef::Def` there.

### 13.3 Semantics

A call adds no new semantics beyond Datalog with a finite set of built-in relations: the map `f` is the (possibly infinite) relation `{(x₁, …, xₙ, f(x…))}`, and a call is a positive or negated literal over it whose input columns must be bound. So:

- **Bound inputs only** (as §3.4 required): every argument of a call must be bound before it, in the sense `plan::bind` already uses: by a positive literal, a parameter, a constant, an equation with a bound side, or an earlier call's output. A call is never unified against: `x == f(y)` with `y` unbound and `x` bound is refused, not inverted. Safety (§3.3) is extended: a variable is bound by a positive literal **or by being the output of a call whose inputs are bound**.
- **Partial functions:** where `f` is undefined, the call's literal is false and the row is dropped, as for a relation with no tuple. For a test this holds **negated or not**: a partial test has three outcomes, `f(x…)` holds only where it is defined and true, `!f(x…)` only where it is defined and false. This is never an error. `Stats` counts undefined calls, so that a query which drops rows by overflow can be seen to do so.
- **Negation:** `!f(x…)` is allowed only for tests. It needs no stratification, because a function does not depend on any predicate.
- **Errors** are only resource limits (§6.0).

### 13.4 Termination

A map in a recursive rule can **invent values**: `n(y) = n(x) && y == x + 1` never reaches a fixpoint. The derived-tuple, byte and time limits stop such a query, but they report a limit, not an answer.

**`Safe` (corrected after Mistral's review, 2026-11-18).** A function is `Safe` when there is a closure `C` from values to **finite** sets of values of its result sort such that `f(x₁, …, xₙ) ∈ C(x₁) ∪ … ∪ C(xₙ)` for all arguments, and `C` is closed under itself (`w ∈ C(v)` implies `C(w) ⊆ C(v)`). For text, `C(t)` is the set of substrings of `t`: `parent`, `basename` and prefixes are `Safe`. For ints, `C(i)` is `{0, …, i}` for `i ≥ 0`. Then from finitely many starting values, iterating `Safe` calls reaches only finitely many values, so recursion through them terminates. Every other map is `Inventive` (`+`, `sizeof`, `concat`). Each builtin declared `Safe` states its `C`, and a property test checks `f(x…) ∈ ⋃ C(xᵢ)` on random inputs.

**The check.** In each rule whose head is in a recursive stratum (it depends on itself), build the rule's flow graph over variables: a call's inputs flow to its output, and an equation `a == b` flows both ways. `check` refuses the rule if a variable bound by an `Inventive` call reaches a head argument through this graph (so `z == x + 1 && y == z` with `y` in the head is refused, as is passing `z` through a `Safe` call). A non-recursive rule may call anything (`length(p)`, `x + 1`).

### 13.5 Planning, execution and the budget

- **Planner** (§6.1): a call is ready once its inputs are bound. A ready `Const` or `Linear` call scores like `!=` today (100, `Const` before `Linear`), a map that binds an output like a binding equation (90), and a ready `Heavy` call below every narrow lookup (under 60), so it runs after them. The `name(p, _)` prefix range of §6.0 (`plan::name_prefix`) and the parser's `startswith` keyword are re-keyed on the call of the builtin `startswith`; B-067's test is that the prefix range still applies, since without it a name scan is wide (§6.1's 2 ms against 30 s).
- **Magic sets** (§6.2): unchanged in principle. Calls are kept in magic-rule bodies, since the prefix of a planned body binds their inputs by construction; the sideways passing treats a call's output as bound after it.
- **Executor:** a call evaluates its function on the bound arguments. Each call costs steps from the budget: one for `Const`, the argument bytes divided by a constant for `Linear`, the declared bound for `Heavy`. Heavy calls are memoised per run by their arguments, and the deadline and cancel flag are checked between calls. So no function escapes `Limits`.
- **Bounded runs** (`run_bounded`, B-017) need nothing new: the worker is the same executor.

### 13.6 SQL translation

A call translates only if its function has a `sql` template **and** the template computes exactly the native value on every input, including where the function is undefined: a partial function's template yields `NULL` there. A test translates as `tpl IS NOT NULL AND tpl`, a negated test as `tpl IS NOT NULL AND NOT tpl` (not `NOT tpl`, which SQLite's three-valued logic turns into `NULL`), and a map's output adds `tpl IS NOT NULL`. A map's output variable is replaced by its expression in the statement. A call without an exact template makes the query `Unsupported`, as mutual recursion does (§7), and the native executor runs it. Exactness is a claim, so the differential property test (§12) runs plain, magic and SQL evaluation on random programs **with calls**, and compares.

Known traps that decide which templates exist: SQLite's `length` counts characters, not bytes; its integer arithmetic turns overflow into floating point; `LIKE` is case-insensitive for ASCII. So in the first library, integer arithmetic has no SQL template, and text functions are defined to match what SQLite can compute exactly.

### 13.7 sophialang functions (later)

A stored sophialang definition can be a query function, `FnRef::Def(id)`, when:
- its type maps to query sorts (`Id`, `Text`, `Int`, `Bool`);
- its effect row is empty and it is total, or partial in the sense of §13.3, shown by a **fact with evidence** from sophialang's effect checker (SPEC-002, [[Effects Memory and Resources]]); an annotation (trust) is not enough;
- its cost class and recursion safety are facts or annotations on it; without them it is `Heavy` with the default bound, and `Inventive`.

It is evaluated by the sophialang interpreter (M4), inside the query's budget, and never gains I/O or mutation by being called from a query. Its content id is in the query's bytes, but that pins only its code: a change in the interpreter's semantics would change answers under the same result id. So a query with a `Def` call also carries the evaluator's semantic version (`eval: sophialang-eval-vN`) in its header, as builtins carry `sophia-fn-v0`. This is where the query language and sophialang meet without merging their semantics (§3.4). It waits for M4. Functions in Julia, Rust or Python stored through Moonkale are **not** callable: Sophia does not run foreign code in queries.

### 13.8 The first library, `sophia-fn-v0` (proposal)

Only what a stored or planned query needs, each with its stated meaning and its SQL template where exact. `C` is the closure of §13.4 for `Safe` maps:

| function | sig | cost | recursion | SQL |
|---|---|---|---|---|
| `startswith(t, p)` | Text × Text → Bool | Linear | — | `substr(t, 1, length(p)) = p` (as today) |
| `endswith(t, s)`, `contains(t, s)` | Text × Text → Bool | Linear | — | to be checked byte-exact |
| `sizeof(t)` | Text → Int (bytes, Julia's name) | Const | Inventive | none (SQLite counts characters) |
| `parent(p)`, `basename(p)` | Text → Text, on dotted names; undefined at the root | Linear | Safe, `C` = substrings | to be checked |
| `<`, `<=` on Int and on Text (bytewise) | → Bool | Const / Linear | — | `<`, `<=` (Text under `BINARY` collation) |
| `+`, `-`, `*` on Int, checked | Int × Int → Int, undefined on overflow | Const | Inventive | none |

Aggregates (`sum`, `min`, `max`) are not functions: they belong to `select` beside `count` (§11).

### 13.9 Syntax

`julia` profile: a test is written as a call, `startswith(p, "core.")`, `!contains(p, "test")`; a map as an equation, `n == sizeof(p)`; comparisons and arithmetic infix, `n < 10`, `m == n + 1`. Calls do not nest in v1 (`f(g(x))` is a parse error; write `y == g(x) && f(y)`), so that print(parse(t)) stays a fixed point. A local predicate may not take a function's name, and a function may not take a base relation's name. `datalog` profile: `Y = f(X)` and `f(X)`.

### 13.10 Open, for Daniel

1. **The migration:** bump every query id to `query-v1` (proposed), or keep `query-v0` bytes for queries without calls? The proposal bumps, because one rule for every query is simpler than two encodings that agree.
2. **Undefined is false** (proposed) or an error? False keeps Datalog's semantics and SQL's `NULL`; an error would surface overflow.
3. **The first library:** is §13.8 the right set, or should the first build carry only `startswith` and comparisons?
4. **Inventive recursion refused** (proposed), or allowed under the budget with a warning?

### 13.11 Build plan

Rows B-067 to B-070 in [[Backlog]]. B-067 is the mechanism with `startswith` as its only function and no change of behaviour except the ids; the library, the termination check and sophialang functions follow.
