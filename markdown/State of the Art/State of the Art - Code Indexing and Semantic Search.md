# State of the Art - Code Indexing and Semantic Search

[[Start Here]] notes it would be "very beneficial as one could query the exact context of a piece of code in the database". That capability — as distinct from compiling out of the database — already exists in production at very large scale, and is worth separating out because it is the part of the idea that is *known to work*.

- **Kythe** (Google) — a language-agnostic node/edge schema plus per-language indexers, feeding Google's internal code search. The schema design (anchors, semantic nodes, `ref`/`defines` edges) is the most directly reusable prior art for [[Graph Schema]].
- **Glean** (Meta) — a fact database with its own typed schema language and the Angle query language; indexes many languages, used for code search, navigation and large-scale analysis. Notable for taking the "code is a database of typed facts" position seriously and making it fast.
- **LSIF** and its successor **SCIP** (Sourcegraph) — standardised, language-server-derived index formats for cross-repository code navigation. SCIP replaced LSIF largely because LSIF's graph model was awkward to produce and consume incrementally — a cautionary note for anyone designing a code graph format.
- **Stack graphs** (GitHub, building on Barrett/Visser's **scope graphs**) — a graph formalism for *name resolution* that is incremental and file-local: each file's graph is computed independently and resolution is a path query over the union. This is a genuinely elegant result and is the right answer to "how do you resolve names in a code graph without reanalysing the world".
- **tree-sitter** — incremental parsing producing concrete syntax trees, with grammars for essentially every language. The realistic first step for any frontend that does not need full semantics, and the standard tool for syntax-level ingestion.
- **CodeQL** — code as a relational database queried in a [[Datalog]]-like language, used for security analysis at scale. The strongest demonstration that non-trivial semantic analysis can be expressed as queries over a code database.
- **Sourcegraph, OpenGrok, Zoekt** — the search layer; Zoekt's trigram index is the standard approach to fast regex search over large corpora.
- **Semantic code embeddings** — the vector-search end of the space, which is why [[State of the Art - Graph Databases for Code|HelixDB]]'s combination of graph and vector search is a plausible fit if natural-language search over the store is ever wanted.

## What this establishes and what it does not

**Established, at scale:** parsing many languages into one schema; cross-references and navigation as graph queries; typed fact schemas; incremental per-file indexing (stack graphs); analysis-as-query (CodeQL). The ingestion half of [[Start Here]] is not speculative — it is a solved problem with multiple production implementations.

**Not attempted by any of them:** compiling *out* of the store, storing lowered IR, storing equivalences or proofs, or using the index as the source of truth rather than a derived artifact. Every system here is a read-only, regenerable index over authoritative source files.

That boundary is precisely where Sophia's novelty and Sophia's risk both begin. It also suggests a tactical point for [[Roadmap|M0–M1]]: the ingestion layer should not be invented from scratch. tree-sitter for syntax, Kythe's schema as a reference for cross-references, and stack graphs for name resolution are all directly adoptable, which lets early effort go to the parts nobody has built.

## Related

- [[State of the Art - Graph Databases for Code]]
- [[Graph Schema]]
- [[Query Cookbook]]
- [[Datalog]]
- [[Tests and Documentation as Nodes]]
