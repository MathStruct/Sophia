# State of the Art - Graph Databases for Code

## Databases under consideration (from the README)

- **LadybugDB** — an embeddable graph database; comparatively niche relative to the others below.
- **HelixDB** — a newer graph database aimed at AI/agentic workloads combining graph and vector search.
- **TypeDB** — a database built around a strongly typed, entity-relationship-style schema language rather than a plain property graph; its type system and rule/inference engine are unusually well matched to storing *typed* code and derived facts.
- **FalkorDB** — a Redis-based property graph database (a continuation of RedisGraph), using the Cypher query language and sparse-matrix graph algorithms internally; strong on speed for graph traversal/analytics.
- **DuckDB** — not a graph database but an embedded, in-process analytical (OLAP) SQL engine; relevant as a very fast option for tabular/analytical queries over flattened graph data, and for its recursive-CTE support for tree/graph queries.
- **TursoDB** — a distributed, edge-friendly reimplementation of SQLite (from the makers of libSQL); relevant for embeddability and sync rather than graph modeling specifically.

None of these were purpose-built for compiler IR storage, so the schema design (nodes = statements/definitions keyed by hash, edges = data/control-flow/type/proof relationships) is project-specific work regardless of which engine is picked.

## Prior art: code already stored as queryable graphs/facts

Two production systems already do a version of "parse code into a database and query it," though neither compiles from that representation:

- **Glean** (Meta/Facebook, open source) — indexes source code across many languages into a fact database (using its own Angle query language) for cross-referencing, code search, and static analysis. Facts are typed and schema-defined per language, closely resembling the "graph of typed statements" model.
- **Kythe** (Google, open source) — a language-agnostic schema (a graph of nodes and edges over a common "GraphStore") plus per-language extractors/indexers, built for cross-references and code navigation (used in Google's internal code search). Its node/edge schema for representing ASTs and semantic facts language-independently is a useful reference point for schema design.

Both systems demonstrate the "index into a graph, one extractor per language" half of the idea in [[Start Here]] at large scale; neither attempts the "compile back out of the graph" or "equivalency proofs between languages" half, which appears to be the more novel part of this project.

## Related

- [[Start Here]]
- [[State of the Art]]
- [[State of the Art - Content-Addressable Code Systems]]
