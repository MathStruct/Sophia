#overview

> Sophia's specifications, numbered `SPEC-nnn`. A specification answers numbered requirements ([[Requirements]]) and stays a draft until Daniel accepts it. Use [[Specification Template]] for a new one. Since 2026-10-20 the **store** (Sophia) and the **language** (sophialang) have separate specifications (SR-25).

| # | Specification | Status |
|---|---|---|
| [[SPEC-001 Sophia Store v0]] | the language-agnostic store: identity, components, nominal entities, names, lineage, annotations and derived facts, the change feed, queries, backends | draft |
| [[SPEC-002 Sophialang Principles]] | what sophialang must be: autoresolve → unsafe → safe; zero overhead; multiple dispatch; memory, concurrency, JIT; tests; self-hosting | draft |
| [[SPEC-003 Sophialang Core v0]] | the first core calculus and its normaliser | draft |
| [[SPEC-004 Moonkale Interface]] | the boundary Moonkale wraps (WP-68); the first priority | draft; its 2026-10-17 form was reviewed by the Moonkale agent |
| [[SPEC-006 Worlds, Artifacts and Evidence]] | world snapshots, artifact identity and evidence, parameter artifacts, proof trust, migration | draft (from the external review) |
| [[SPEC-005 Rendering Profiles]] | display as user settings stored in the database; the Julia-flavoured default | draft |
| [[SPEC-009 Moonkale Folder Store]] | One Sophia store per folder as Moonkale's derived code-graph cache (ADR-0029): Moonkale writes finished records, per-file replacement in one transaction, a derived mode, wasm, a first tag | accepted (Daniel, 2026-10-24); implemented (`v0.1.0` to `v0.1.4`) |
| [[SPEC-010 Store-Truth Folders and Generated Files]] | Moonkale ADR-0030 (SR-148): moving a folder's truth into the store and back, file tokens, generated files, outside edits as proposals; Moonkale normalises foreign code | accepted (Daniel, 2026-10-26); implemented 2026-10-28 except `propose_text` and the SR-149 export |
| [[SPEC-008 Module-Level Julia Resolution]] | Julia references resolved per module, not per file: a proposal for Moonkale's normaliser v5, with a prototype | proposal, for the Moonkale agent's review |
| [[SPEC-007 Sophia Query Language]] | Sophia's own storage-independent query language: stratified Datalog over base relations, Julia-flavoured syntax, content ids, result ids, native executor, magic sets, SQL translation | draft; implemented (`sophia-query`) |
