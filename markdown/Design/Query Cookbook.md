# Query Cookbook

Concrete queries against [[Graph Schema]], in the three dialects that match the candidate backends. The point of writing them out is that "a compiler is queries over a database" is only credible if the queries are ordinary.

## 1. Fetch a definition's full core term

**SQL (DuckDB / SQLite / Turso)** — recursive CTE over the `CHILD` edges:

```sql
WITH RECURSIVE subtree(h, depth) AS (
    SELECT ?root, 0
  UNION ALL
    SELECT e.dst, s.depth + 1
    FROM subtree s JOIN edge e ON e.src = s.h
    WHERE e.kind = :CHILD AND s.depth < 10000
)
SELECT n.h, n.kind, n.payload, e.ord
FROM subtree s
JOIN node n ON n.h = s.h
LEFT JOIN edge e ON e.dst = s.h AND e.kind = :CHILD
ORDER BY s.depth, e.ord;
```

**Cypher (FalkorDB / Neo4j)**:

```cypher
MATCH p = (root:Term {h: $root})-[:CHILD*0..]->(n:Term)
RETURN nodes(p), relationships(p)
```

**Datalog**:

```datalog
reach(R, R).
reach(R, C) :- reach(R, P), child(P, _, C).
```

The SQL version is the one to benchmark first; see the chunked-storage caveat in [[Compilation as Query]].

## 2. What breaks if I change this?

The single most valuable query in the system — exact, not heuristic.

```cypher
MATCH (changed:Decl {h: $h})<-[:DEPENDS_ON*]-(affected:Decl)
OPTIONAL MATCH (affected)<-[:TESTS]-(t:Test)
RETURN affected.h, collect(t.h) AS tests_to_rerun
```

With a materialised transitive closure this is a single index lookup. See [[Tests and Documentation as Nodes]].

## 3. Find an alternative implementation within a modulo budget

The query a compiler runs when it wants to substitute a faster implementation.

```cypher
MATCH path = (a:Decl {h: $h})-[rels:EQUIV*1..4]-(b:Decl)
WHERE ALL(r IN rels WHERE r.level IN ['defeq','rewrite','observational'])
  AND ALL(r IN rels WHERE ALL(m IN r.modulo WHERE m IN $allowed_modulo))
MATCH (b)-[:BENCHMARKS]->(bm:Bench {target: $target})
RETURN b.h, bm.median_ns, [r IN rels | r.level] AS chain
ORDER BY bm.median_ns ASC
LIMIT 1
```

Note the two `ALL` guards: they enforce the substitutability rule and the modulo budget from [[Equivalence and Witnesses]]. Dropping either is how this system would produce wrong code.

## 4. Constant folding as a rewrite rule

```datalog
equiv(App, Folded, "rewrite", []) :-
    term(App, "app"),
    child(App, 0, F), prim(F, "add.wrap.i64"),
    child(App, 1, X), lit(X, A),
    child(App, 2, Y), lit(Y, B),
    Folded = intern_lit(wrap_add_64(A, B)).
```

Compare [[LLVM Pass]]: the same transformation, expressed as a fact-producing rule rather than as an in-place mutation, and therefore both persistable and reversible.

## 5. Which definitions have no test coverage?

```sql
SELECT n.h FROM node n
WHERE n.kind = :DECL
  AND NOT EXISTS (SELECT 1 FROM edge e
                  WHERE e.dst = n.h AND e.kind = :TESTS);
```

## 6. Find duplicated logic across languages

Definitions that elaborate to the *same* core term but came from different frontends — free cross-language equivalence, discovered rather than asserted ([[Equivalence and Witnesses]]).

```cypher
MATCH (s1:SurfaceNode)-[:ELABORATES_TO]->(t:Term)<-[:ELABORATES_TO]-(s2:SurfaceNode)
WHERE s1.lang <> s2.lang
RETURN t.h, collect(DISTINCT s1.lang + '/' + s2.lang)
```

## 7. Trust audit of a build artifact

```cypher
MATCH (a:Module {h: $artifact})-[:DEPENDS_ON*]->(x)
OPTIONAL MATCH (x)-[e:EQUIV]-()
WITH collect(DISTINCT e.level) AS levels, collect(DISTINCT e.modulo) AS mods
RETURN levels, apoc.coll.toSet(apoc.coll.flatten(mods)) AS total_modulo
```

Implements the `trust(artifact) = min over derivation` rule from [[Trusted Computing Base]].

## 8. Cache lookup for precompilation

```sql
SELECT code FROM artifact
WHERE h_run = ? AND h_target = ? AND h_opts = ?;
```

Trivially simple, and that simplicity is the whole argument of [[Content-Addressed Precompilation]].

## 9. Dispatch invalidation (Julia-specific)

Which cached specialisations are contradicted by a newly added method?

```cypher
MATCH (m:Method {h: $new_method})
MATCH (spec:Decl)-[:DEPENDS_ON]->(f:DispatchFact)
WHERE f.fname = m.fname AND sigOverlaps(f.argtypes, m.sig) AND moreSpecific(m.sig, f.chosen_sig)
RETURN spec.h
```

`sigOverlaps` and `moreSpecific` are Julia's own subtyping predicates, which would need exposing as database functions — a concrete piece of work, and the crux of whether the invalidation-precision claim holds up. See [[World Age]].

## Query patterns that will be slow

Worth stating up front, since they determine the storage design:

- Unbounded `-[:CHILD*]->` over large functions ⇒ use chunked subtree blobs.
- Unbounded `-[:EQUIV*]-` closure ⇒ must be bounded and modulo-filtered; never materialise.
- Anything joining across all IR layers at once ⇒ partition by layer.
- Text search over `Doc` and `Name` ⇒ wants a separate inverted index, not the graph.

## Related

- [[Graph Schema]]
- [[Compilation as Query]]
- [[Datalog]]
- [[Property Graph]]
- [[State of the Art - Graph Databases for Code]]
