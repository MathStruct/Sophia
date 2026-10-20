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
| `member(n, d)` | Id, Id | `d` is a current member of nominal `n` (a method, a relation's rule) | `n` (`relation`); scan over `nominal_ids` |
| `nominal(n, kind)` | Id, Text | a nominal entity and its kind | `n` (`nominal`); scan over `nominal_ids` |
| `uses(d, n)` | Id, Id | `d` references nominal entity `n` | `d` (the record); scan over records |
| `annotation(a, kind, s)` | Id, Text, Id | annotation `a` of a kind on subject `s` | `s` (`annotations_of`); scan over subjects |
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

`x + y == 10` in sophialang computes a Boolean once `x` and `y` have values. In a query, `x == y` is a **constraint** that may bind a variable. The query language has its own semantics (least fixpoints over finite relations), so it is a **separate typed DSL**, not an interpretation of arbitrary sophialang expressions. Arithmetic, user functions and dispatch are not in v0. They enter later as **pure functions with bound arguments** (like `startswith`), never as unification over function results.

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
- **the order of predicates**: they are numbered by first visit from the select;
- **the order of rules**, and duplicate rules: rules are sorted by their canonical bytes.

Renumbering and sorting repeat until they settle (at most 8 rounds, keeping the smallest form seen).

**Not normalised: the order of literals inside a body,** nor commutations of `==` and `!=`. Two bodies that differ only in literal order get different ids, just as `f(a, b)` and `f(b, a)` do in code. The reasons:
- full canonisation of conjunctive queries is graph canonisation;
- a quotient would cost the readability of the stored form.

Equivalences between such queries are **facts with evidence** (§8), as for programs. The remaining corner (symmetric rule sets that need more than eight rounds) is [[SP-009 Canonical form of queries]].

### 4.2 Bytes and id

The canonical bytes use WP-66 tag-length-value framing:
- `sophia: query-v0`, `schema: store-base-v0`;
- the parameter sorts;
- per predicate: its arity and its rules' bytes;
- the select.

The **query id** is `member_digest(component_digest("sophia-query", "sophia-query@0", bytes), 0)`. That is the store's identity rule for a one-member component (SPEC-001 §4). So a stored query verifies with `Record::verify` like any definition.

### 4.3 Result identity and caching

`ResultId = H("query-result-v0", query id, snapshot, parameter values)`. Two runs with the same result id must return the same rows.

**Snapshot v0** = H(store instance id, last log seq). The instance id is a random nonce kept in meta. This is **coarse**: any logged change gives a new snapshot, even a profile switch. A precise snapshot would cover only the base relations the query reads, and later a content-addressed snapshot of names and worlds (SPEC-006 §2) ([[SP-010 Query snapshots are coarse]]).

`run_cached` stores rows in a meta row keyed by the result id. Meta rows are not logged, so caching does not move the snapshot. Truncated results are not cached.

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
- **Base literals** go through an access path when a lookup column is bound (memoised per run), and through a full scan otherwise.
- **Local relations** keep hash indexes on the column sets the plans look them up by.
- **Limits:** a join-step budget, a row cap (the select stops early), and a **derived-tuple cap**, which is the memory bound (5 M tuples by default). The step budget alone did not bound memory: without magic sets, a closure over 10⁵ definitions passed 14 GB before reaching 50 M steps. `Stats` report lookups, scans, derived tuples and rounds.

### 6.1 Planning

A greedy planner orders each body:
1. filters as soon as their variables are bound;
2. then equations that bind;
3. then base literals with an access path on a bound column;
4. then local literals with bound arguments;
5. full scans last.

The same planner decides the sideways information passing of §6.2.

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

**Negation:** negated calls use the all-free adornment, and negations are dropped from magic rules, which only enlarges the magic set and is sound. If the rewrite would break stratification, the query runs as written.

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

**Agents** will reach the same API through the MCP server (M3b), not through backend SQL (SR-100).

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

- **A prefix access path** on names: `startswith(p, "core.")` with a constant prefix should become a range scan. B-17 shows the scan costs 3–4 ms at 10⁴.
- **Precise snapshots** (SP-010): per-relation epochs, then content-addressed name and world snapshots.
- **Incremental view maintenance:** semi-naive evaluation is already the delta rule; DBSP or differential dataflow keep results up to date as the change feed arrives.
- **Cost-based planning** from backend statistics, and a **DuckDB translation**.
- **Aggregates** beyond `count` (`sum`, `min`, `max`), arithmetic, and pure functions with bound arguments.
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

Tests: 11 identity tests and an execution suite.
- **Execution suite:** plain and magic programs agree on every backend; SQL agrees with native for both programs; reach matches a graph walk; magic sets do no full scans; caching per snapshot; stored queries are definitions and dependants; step limits.
- **Interface:** an acceptance test on every backend.

Benchmarks: B-17 in [[Benchmark Results]].
