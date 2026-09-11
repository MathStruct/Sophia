# Graph Schema

What the nodes and edges actually are. This is the schema [[Start Here]] invites anyone to disagree with and migrate away from, so it is written to be replaceable: nothing outside this note and [[schema]] should know the concrete labels.

## Universal node properties

Every node carries:

| Property | Meaning |
| --- | --- |
| `h` | the 256-bit identity from [[Hashing and Identity]] — primary key |
| `kind` | node tag (see below), also mixed into `h` |
| `ver` | hash schema version |
| `payload` | kind-specific attributes, canonically encoded |

Nodes are **immutable and append-only**. There is no UPDATE in this system; there are only new nodes and new edges, and names that point somewhere else ([[Naming and Change Propagation]]). This is what makes the store re-verifiable without being trusted ([[Trusted Computing Base]]).

## Node kinds

**Core layer** — the language of [[Core Calculus]]:

- `Term` — a core term node (`app`, `lam`, `pi`, `let`, `con`, `elim`, `prim`, `var`, …, discriminated in `payload`)
- `Type` — a `Term` occurring in type position; kept as a separate label purely so type-level queries are cheap
- `Decl` — a named, top-level definition: binds a `Term` to a `Type` with an effect row
- `Sig` — a type signature/interface with no body, for declarations from headers or `extern`
- `Effect` — an effect row or a single effect constructor
- `Universe` — a universe level

**Surface layer** — per-frontend, one sub-schema each:

- `SurfaceNode` — a node of a source language's own AST, before elaboration (`lang` in payload)
- `Token` / `Span` — lexical provenance, never hashed into the core identity
- `Frontend` — the extractor version that produced a surface tree

**IR layers**:

- `Op` — an [[MLIR Operation]]: dialect-qualified name, operands, results, attributes, regions
- `Region`, `Block`, `Value` — MLIR's nesting structure
- `Instr`, `BasicBlock`, `Func`, `Module` — [[LLVM IR]] level
- `Target` — a target triple + datalayout + CPU features; lowering is only meaningful relative to one

**Knowledge layer** — nodes that are *not* compiled:

- `Witness` — evidence for an equivalence or a property ([[Equivalence and Witnesses]])
- `Prop` — a proposition (a `Type` in the `Prf` fragment)
- `Test` — an executable test attached to a definition ([[Tests and Documentation as Nodes]])
- `Bench` — a benchmark plus recorded results
- `Doc` — markdown, a comment, a docstring
- `Name` — a human-readable label in a namespace; *mutable pointer*, the one exception to immutability
- `Author` / `Signature` — provenance for asserted (unproved) claims

## Edge kinds

Edges are themselves content-addressed: $h("edge") = H("ver" ‖ "EDGE" ‖ "kind" ‖ h("src") ‖ h("dst") ‖ "ord" ‖ "attrs")$. This matters because a witness needs to *cite* edges, and you cannot cite what has no identity.

| Edge | From → To | Notes |
| --- | --- | --- |
| `CHILD(i)` | `Term` → `Term` | ordered structural child; the spine of every AST |
| `HAS_TYPE` | `Term` → `Type` | |
| `DEPENDS_ON` | `Decl` → `Decl` | transitive closure precomputed and materialised |
| `ELABORATES_TO` | `SurfaceNode` → `Term` | the frontend's output, one per (node, frontend version) |
| `LOWERS_TO` | `Term` → `Op` → `Instr` | carries the `Target` and pass pipeline in attributes |
| `SPECIALIZES` | `Decl` → `Decl` | Julia's per-signature specialisation, C++ template instantiation |
| `INSTANCE_OF` | `Decl` → `Sig` | implementation of an interface |
| `EQUIV(level, modulo)` | `Term` ↔ `Term` | the load-bearing one; see below |
| `REFINES` | `Term` → `Term` | one-directional: `a` may be used where `b` was, not conversely |
| `WITNESSED_BY` | `EQUIV` edge → `Witness` | edges pointing at edges; hence hashed edges |
| `PROVES` | `Witness` → `Prop` | |
| `TESTS` | `Test` → `Decl` | |
| `DOCUMENTS` | `Doc` → any | |
| `NAMED` | `Name` → any | mutable; the only mutable relation |
| `MIGRATES_TO` | node@ver1 → node@ver2 | hash-schema migration ([[Hashing and Identity]]) |
| `EMITS` | `Module` → artifact blob | |

```tikz
\usepackage{tikz-cd}
\begin{document}
\begin{tikzcd}[column sep=large, row sep=large]
\text{SurfaceNode} \arrow[r, "\text{ELABORATES\_TO}"] & \text{Term} \arrow[r, "\text{LOWERS\_TO}"] \arrow[d, "\text{HAS\_TYPE}"'] \arrow[dr, "\text{EQUIV}" description] & \text{Op} \arrow[r, "\text{LOWERS\_TO}"] & \text{Instr} \\
\text{Doc} \arrow[u, "\text{DOCUMENTS}"] & \text{Type} & \text{Term}' \arrow[l, "\text{HAS\_TYPE}"] \arrow[u, "\text{WITNESSED\_BY}"', dashed] &
\end{tikzcd}
\end{document}
```

## The EQUIV edge in detail

`EQUIV` carries two attributes that do all the work:

- `level ∈ {alpha, defeq, rewrite, observational, tested, asserted}` — strength, totally ordered
- `modulo ⊆ {alloc, timing, fp_assoc, fp_contract, exception_identity, gc_pressure, ordering}` — what the claim ignores

Composition of two `EQUIV` edges takes the **minimum level and the union of moduli**. A transitive chain therefore degrades monotonically, which is the honest behaviour; see [[Equivalence and Witnesses]] for why union-of-moduli is a real problem and not a formality.

## Mapping onto concrete backends

The schema above is abstract. Three plausible materialisations, matching the candidates in [[State of the Art - Graph Databases for Code]]:

**Relational (DuckDB / Turso / SQLite)** — two wide tables and a name table:

```sql
CREATE TABLE node (h BLOB PRIMARY KEY, kind SMALLINT, ver SMALLINT, payload BLOB);
CREATE TABLE edge (h BLOB PRIMARY KEY, src BLOB, dst BLOB, kind SMALLINT,
                   ord INTEGER, attrs BLOB);
CREATE TABLE name (ns TEXT, name TEXT, target BLOB, valid_from BIGINT, valid_to BIGINT);
CREATE INDEX edge_src ON edge(src, kind, ord);
CREATE INDEX edge_dst ON edge(dst, kind);
```

Structural queries become recursive CTEs. This is unglamorous and probably the right first choice: a columnar engine over two tables with a hash primary key is extremely fast for the "fetch a whole subtree" and "which definitions transitively depend on `h`" queries that dominate, and it has no server. See [[Query Cookbook]].

**Property graph (FalkorDB / Neo4j-compatible)** — labels map to node kinds directly, Cypher expresses traversal natively, and variable-length path queries (`-[:DEPENDS_ON*]->`) are first class. Better ergonomics, worse embeddability.

**Typed/deductive (TypeDB, or Datalog over any store)** — the schema's type discipline is enforced by the database, and derived facts (transitive dependency, equivalence closure, dead-code reachability) are *rules* rather than materialised tables. Architecturally the closest fit to [[Compilation as Query]], and the closest relative of [[State of the Art - Equality Saturation and E-Graphs|egglog]].

The recommendation is to implement [[sophia_store|a backend trait]] and start with DuckDB, because the schema is simple enough that the graph-specific features earn their keep only later.

## Scale estimate

A 100k-line Julia package elaborates to roughly $10^6$–$10^7$ core `Term` nodes. Lowering to MLIR multiplies by ~3–10; LLVM IR by another ~2–5. So a mid-sized package is $10^7$–$10^8$ nodes if all IR layers are persisted for one target. That is large but not exotic for a columnar store — and it is the main argument for making IR-layer persistence *optional and cache-like* rather than mandatory. See [[Open Problems and Risks]].

## Garbage collection

Append-only stores grow. Reachability is from the `Name` table (the roots) plus pinned hashes; anything unreachable *and* older than a retention window is collectable. Note that deleting a node is safe precisely because identity is content-derived: if it is ever needed again it can be regenerated with the same hash.

## Related

- [[Hashing and Identity]]
- [[Equivalence and Witnesses]]
- [[Query Cookbook]]
- [[Property Graph]]
- [[Datalog]]
- [[State of the Art - Graph Databases for Code]]
- [[schema]] — the Rust module that would define this
