# Property Graph

The data model used by most graph databases (Neo4j, [[State of the Art - Graph Databases for Code|FalkorDB]], Memgraph, Amazon Neptune's openCypher mode): nodes and directed edges, each carrying a set of *labels* and a map of *properties*. Edges are first-class — they have their own identity, type and properties — which distinguishes the model from RDF triples, where an edge is just a subject–predicate–object statement and attaching data to a relationship requires reification.

For [[Graph Schema|Sophia's schema]] this matters concretely: an `EQUIV` edge carries a `level` and a `modulo` set, and a `LOWERS_TO` edge carries a target and a pass pipeline. In a property graph those are edge properties; in a triple store each would need its own reified node.

The query language is usually **Cypher** (now standardised as GQL), whose distinguishing feature is ASCII-art pattern matching with variable-length paths:

```cypher
MATCH (a:Decl)-[:DEPENDS_ON*1..5]->(b:Decl) WHERE b.h = $h RETURN a
```

That variable-length traversal is the thing relational engines express only with recursive CTEs, and it is the main ergonomic argument for a graph database over [[State of the Art - Graph Databases for Code|DuckDB]] here. The counter-argument is that Sophia's schema is only two tables wide and its hot queries are hash lookups and bounded traversals, both of which a columnar engine does extremely well without a server.

## Related

- [[Datalog]]
- [[Graph Schema]]
- [[Query Cookbook]]
- [[State of the Art - Graph Databases for Code]]
