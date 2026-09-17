# Tests and Documentation as Nodes

[[Start Here]] says "tests would be inserted as nodes on a piece of code", and that annotations may "just contain comments, markdown, etc.", and that a benefit is being able to "query the exact context of a piece of code". This note takes the non-compiled half of the database seriously, because it is the part most likely to be immediately useful.

## The general shape

Anything that says something *about* code, rather than *being* code, is a node attached by an edge:

| Node | Edge | Attached to |
| --- | --- | --- |
| `Test` | `TESTS` | a `Decl`, or a `Term`, or an `EQUIV` edge |
| `Bench` | `BENCHMARKS` | a `Decl` + a `Target` |
| `Doc` | `DOCUMENTS` | anything, including other `Doc`s |
| `Prop` / `Witness` | `PROVES` | a `Decl` or an `EQUIV` edge |
| `Example` | `EXEMPLIFIES` | a `Decl` |
| `Deprecation` | `ANNOTATES` | a `Name` binding |
| `Provenance` | `ANNOTATES` | anything |

All of these are content-addressed like everything else, which produces a property that file-based projects cannot have.

## Tests bind to hashes, so test results never go stale

A test result is a fact about a *hash*, not about a file at a point in time:

$$ "result" : (h_"test", h_"subject", h_"target", "seed") → {"pass", "fail", "error"} $$

This function is pure, so the result can be cached forever. The consequences are large:

- **A test never needs re-running for an unchanged definition**, at any granularity, across machines and across time. Not "unchanged file" — unchanged *definition*, so reformatting, renaming, moving between files and editing a neighbouring function all leave the cache intact.
- **Test selection is exact, not heuristic.** The set of tests to run after a change is the tests attached to anything in the transitive dependents of the changed hashes. CI test-selection tools today approximate this from file dependencies; here it is a graph query, and it is exact.
- **Flakiness is visible.** Same input hash, different results ⇒ the test is nondeterministic, detected automatically rather than by folklore.
- **A test can be attached to an `EQUIV` edge**, which is precisely the `tested` evidence level in [[Equivalence and Witnesses]]. Differential testing of two implementations is the natural cheap approximation to a proof, and the graph gives it a home.

## Documentation gains structure

- A docstring referencing a function references it *by hash*, so the reference cannot rot. If the function changes, the doc is visibly attached to the old version and appears in a "documentation todo" query — the same propagation mechanism as [[Naming and Change Propagation]].
- Markdown notes can be attached to intermediate IR nodes. "Why is this loop not vectorised?" can be answered next to the `Op` node in question.
- **This vault is an instance of the idea.** These markdown files are documentation attached to code; the [[Repository Layout|paired `.rs`/`.md` and `.jl`/`.md` files]] are a manual, filesystem-level approximation of the `DOCUMENTS` edge. If the system ever works, the vault should be ingested into it.

## Querying context

The motivating query from [[Start Here]] — "the exact context of a piece of code" — becomes a concrete traversal:

```cypher
MATCH (d:Decl {h: $h})
OPTIONAL MATCH (d)<-[:TESTS]-(t:Test)
OPTIONAL MATCH (d)<-[:DOCUMENTS]-(doc:Doc)
OPTIONAL MATCH (d)-[:HAS_TYPE]->(ty:Type)
OPTIONAL MATCH (d)-[:DEPENDS_ON*1..3]->(dep:Decl)
OPTIONAL MATCH (d)<-[:DEPENDS_ON*1..2]-(rdep:Decl)
OPTIONAL MATCH (d)-[e:EQUIV]-(alt:Decl)
OPTIONAL MATCH (d)<-[:ELABORATES_TO]-(s:SurfaceNode)-[:HAS_SPAN]->(sp:Span)
RETURN d, ty, collect(t), collect(doc), collect(dep), collect(rdep),
       collect({alt: alt, level: e.level, modulo: e.modulo}), sp
```

That single query returns: the definition, its type, its tests, its docs, what it uses, what uses it, every known alternative implementation with the strength of each claim, and where it came from in the original source. Assembling that today requires an IDE, a test runner, a coverage tool, a documentation generator and grep — and still misses the alternatives, because nothing records them.

## Property tests as near-specifications

A property test is a `Prop` with an unproved status — it states a universally quantified claim and provides a falsification procedure rather than a proof. That makes it a natural stepping stone: the same statement can start as a property test, accumulate test evidence, and later acquire a kernel proof without changing its identity or its attachment. The `Witness` node changes; the `Prop` does not.

This gradual path is probably the only realistic way proofs ever appear in a codebase of real size.

## Related

- [[Equivalence and Witnesses]]
- [[Graph Schema]]
- [[Query Cookbook]]
- [[Naming and Change Propagation]]
- [[Content-Addressed Precompilation]]
