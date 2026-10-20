#design

> **Status: accepted (2026-10-20).** It was proposed by Daniel in his forwarded review ([[Review 2026-10-20 Query Language (external)]]: "Sophia should have its own storage-independent query algebra and query language. SQL should be treated as one possible frontend or backend translation, not as the fundamental interface"). It was implemented by Claude the same day. Specified in [[SPEC-007 Sophia Query Language]]. Related: SR-90 to SR-106, [[S-ADR-0010 Primary backend and an analytics replica]].

## Context

Until now the store offered one text query language: read-only SQL over views, on SQLite and DuckDB only (SPEC-001 §11). Three things were wrong with that:
- **The backend choice and the query language were coupled.** LMDB and redb, the fastest primaries, had no query language at all.
- **SQL is a frontend for one storage model, not a meaning for code queries.** Recursive questions ("everything that transitively uses X") are awkward in SQL and limited in SQLite: no mutual recursion.
- **Queries were not Sophia objects.** They had no identity, could not be stored, named, proved equivalent or cached by content.

## Options

1. **Keep SQL as the interface**, and add a SQL layer over the key-value backends. That means writing a SQL engine.
2. **Adopt an existing query language wholesale** (TypeQL, CQL, Soufflé Datalog). Each brings its own data model and runtime, and none of them has content identity.
3. **Sophia's own algebra:** stratified Datalog over the store's base relations, with a canonical form and a content id. It is executed natively on every backend, and translated to SQL where useful.

## Decision

Option 3.
- The algebra is defined over **base relations and access paths**, not over SQL tables or key orders.
- Queries are **definitions**: stored, named and identified like code.
- **SQL is a translation target.** What it cannot express is refused, not approximated.
- Ideas from TypeQL (roles, typed relations) and CQL (functorial migration) are adopted where they fit ([[Query Languages for Sophia]]), not the systems.

## Consequences

- `crates/sophia-query`: the IR, the Julia-flavoured syntax, canonical ids, a native executor, magic sets and the SQL translation (SPEC-007 §12).
- Moonkale gets the `query` dialect on every backend. SR-32's planned `datalog` dialect is this one.
- The planned milestone M3 splits: **M3a** is the query language (done in its first form); **M3b** is the MCP server for agents, which exposes it.
- **Measured (B-17):** goal-directed evaluation (magic sets) matters more than the backend. Transitive dependants of one definition take about 0.15 ms on any backend with magic sets, and about 6 s without (10⁴ definitions).
