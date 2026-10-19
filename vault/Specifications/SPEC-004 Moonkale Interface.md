#design #definition

> **Draft, 2026-10-20 (Claude).** The boundary between Sophia and Moonkale. It is the first priority (SR-63). It is §7, §11 and §12 of the SPEC-001 of 2026-10-17, which the Moonkale agent reviewed and corrected that day. Here they are made language-agnostic (SR-25), extended with rendering profiles read from the database (SR-28), and given a crate of their own. The Unison source (Moonkale WP-18) checks the same shape from the other side, and Daniel wants both kept (SR-23).
>
> Sources: Moonkale `packages/core/src/source/{mod,query,transaction,descriptor}.rs`, `markdown/architecture/Code Graph Schema.md` (including "WP-18 result"), `Annotations.md`, `Security.md`; the Moonkale agent's review of 2026-10-17 ([[Moonkale Interface Needs]]). code: `crates/sophia-interface` (planned); Moonkale `sources/sophia` (Moonkale's).

## 0. In one paragraph

Moonkale sees a Sophia store as **one `truth: Store` source** of family `Graph`. Namespaces are `Directory` nodes, names are `Name` nodes, and definitions of every language are `Definition` nodes whose id is their content id. Text is printed through the reader's rendering profile, which is stored in the Sophia database. Saving text parses it, stores a new definition and moves the name, so v0 needs **no new `Op` variants**. Queries are read-only `sql` in v0. Changes arrive through `changes_since`.


## 1. Crates and the descriptor

- Sophia provides `sophia-store` (the store and its backends), `sophialang-core` and `sophialang-syntax` (the language, with its printer and parser), and **`sophia-interface`**: a small crate holding everything below that is not Moonkale's own types (node and edge mapping, the operation mapping, the profile-aware printing, the query guard). None of them depends on Moonkale.
- Moonkale's adapter `sources/sophia` depends on `sophia-interface` and implements `moonkale_core::Source` as a thin translation of types. That keeps Sophia usable from its own CLI and other tools, and keeps Moonkale's crate graph the way its `check-deps.py` wants it.
- The descriptor: `truth: Store`, `family: Graph`, `capabilities { read: true, write: true, watch: true (polling), text_query: Some(TextDialect::Sql) }`, `root` = the root namespace node. A separate `Code` family would be a change to `moonkale-core` and is left out of v0.

## 2. Nodes, edges and reads

| Moonkale node | id | kind / properties |
|---|---|---|
| namespace | `derive(source, "ns:" + path)` | Moonkale's existing `Directory` kind, as the Unison source (WP-18) uses, so both store-backed sources look the same in the graph view |
| name | `derive(source, "name:" + path)` | `Name`; `visibility` |
| definition | `from_content(digest)` | `Definition`; `def_kind`, `lang` (`sophialang`, later others), `frontend` (the normaliser), `frontend_version`, `content_id`, `status` |
| generic function, nominal type | `from_content(nominal id)` | `sophialang:GenericFunction`, `type` or `interface`; `nominal = true` |
| lineage | `derive(source, "lineage:" + first digest)` | `sophia:Lineage` |
| annotation | `from_content(annotation digest)` | Moonkale's existing `NodeKind::Annotation`, with `sophia_kind` (`doc`, `equivalence`, …) and `status` as properties, so the annotation layer and badges work unchanged |
| term (on demand) | `from_content(term digest)` | `sophia:Term`; `tag` |

| Edge | From → to |
|---|---|
| `Contains` | namespace → namespace or name (lexical structure only) |
| `Names` | name → definition |
| `References` / `Calls` | definition → definition (`Calls` when the reference is in head position of an application) |
| `Tests` | test → definition |
| `sophia:Supersedes` | definition → definition it replaced (same lineage) |
| `MethodOf` (shared) | method → generic function (current method table) |
| `TypeOf` (shared) | definition → its type's node (a `Definition` or a nominal type). One edge per typing system. The typing fact's `system`, `status`, `by`, `by_version`, `inputs` and `derived_at` are edge properties. A type that is not itself a stored definition (an inferred compound type) is drawn as a `sophia:Type` node |
| `sophia:InLineage` | definition → lineage |
| `Annotates` | annotation → subject (Moonkale's existing `EdgeKind::Annotates`) |
| `sophia:Subterm` | definition or term → child term, with a `position` property. It is **not** `Contains`: shared subterms make a DAG, and `Contains` would put terms in the explorer tree, which the views treat as a tree |

- `Query::Node`, `Children` and `Neighbours` are answered from these.
- `Children(namespace)` gives its names and sub-namespaces.
- `Children(definition)` gives nothing by default. The root term is returned only when terms are explicitly asked for (`Neighbours` over `sophia:Subterm`). Terms are fetched lazily, because the term DAG can be large and Moonkale draws definitions first.
- `All { limit, kinds }` lists definitions and names.
- **Checked against WP-18** (Moonkale's Unison source, merged 2026-10-17):
  - It needed no new shared kinds: `Definition`, `Name`, `Directory`, `Contains`, `Names`, `References` and `TypeOf`.
  - A recursive group shares a component id, and each member has its own index.
  - There are no `Defines` edges.
  - Built-ins keep an intrinsic name identity, matching Sophia's `Prim(name)`.
  - Moonkale's graph panel now draws every node kind for `truth: Store` sources and opens a definition's text on click, so the Sophia adapter gets both for free.
- **Names are primary here.** For file-backed code, Moonkale's graph panel hides `Name` nodes from its normal symbol filter, because there names are metadata. For a Sophia source the name is what you open and navigate, so the adapter or the filter has to show them. This is on Moonkale's side, noted so that the WP-68 acceptance test doesn't trip on it.
- **Who owns which annotations:** proofs, docs and equivalences live in Sophia's store. Review comments made in Moonkale stay in Moonkale (`.moonkale/annotations.jsonl`), anchored to the **lineage** id, so they survive edits and renames.

## 3. Edits through the existing `Op` set

v0 needs **no new `Op` variants**. The file-shaped operations map onto the store:

| Moonkale `Op` | On Sophia |
|---|---|
| `fetch_text(name or definition)` | print the definition with the **reader's rendering profile** (§4). `Version` = the store `seq` at which this name last changed (for a name), or 0 for a bare definition, which is immutable |
| `WriteText { node: name, expected, patch }` | apply the patch to the printed text → parse → hash → store the new definition (or find that it already exists) → move the name → append to the lineage. It is refused if the name's `seq` ≠ `expected` (an optimistic check). It returns the name's new `Version`. The new content id is a property of the name's target |
| `CreateText { parent: namespace, name, text }` | parse and store a new definition under `parent.name`. It starts a new lineage |
| `Rename { node: name, to }` | move the name. The result's `node` is the new name id |
| `Delete { node: name }` | remove the name. The definition stays |
| `CreateDir { parent, name }` | create an empty namespace |
| `WriteText` on a **definition** (not a name) | refused, because a definition is immutable. The error says to open it through a name |

- **Re-ingest is a no-op.** If the new text hashes to the content the name already points at, nothing is written, and the unchanged `Version` is returned. If it hashes to a *different* existing definition, the name moves there and nothing new is stored.
- **Opening a definition for editing resolves it to a name.** When a definition is clicked in the graph, `fetch_text(definition)` shows it read-only. To edit it, the adapter picks a name with this rule:
  1. the public name with the lowest `seq` (the oldest);
  2. if there is no public name, the private name with the lowest `seq`;
  3. ties are broken by the shortest path, then lexicographically.

  A definition with no name opens read-only, with an "add name" action, which is `CreateText` of its printed text under a chosen path. That writes nothing new and only adds the name.
- **Path spelling.** Moonkale passes relative paths with `/`, and Sophia names use `.`. The adapter maps `a/b/c` ↔ `a.b.c`. A path segment must be non-empty, must not start with a digit, and may contain Unicode letters, digits, `_` and `!`, plus operator names in backquotes (`` `+` ``). It may not contain `/`, `.`, whitespace or control characters. A rejected path is a `SourceError` naming the offending segment.
- Parse failures come back as `SourceError` with diagnostics (ranges in the printed text), and the name is left unchanged.
- **Annotations** need one new op later (`Annotate`/`Unannotate`), or a pseudo-text node per annotation. That is Moonkale's choice ([[SP-004 Moonkale operations are file-shaped]]).
- **Typecheck on demand** is a later `refresh(node)` that runs the configured checkers and adds `Typing` nodes.


## 3a. Constraints from Moonkale (2026-10-20)

1. The **reader id** is given once, when the adapter is opened or configured, not on every request.
2. **Shared kinds stay shared:** `MethodOf`, `TypeOf`, `References`, `Calls`, `Tests`, `Contains`, `Names` and `Annotates` are Moonkale's shared edge kinds (Code Graph Schema §6), which the Unison source (WP-18) uses too. Only concepts specific to Sophia are namespaced: the edges `sophia:Subterm`, `sophia:Supersedes` and `sophia:InLineage`, and the nodes `sophia:Lineage`, `sophia:Term`, `sophia:Type` and `sophia:Settings`. (Corrected 2026-10-20 at the Moonkale agent's request; the first version of this item namespaced `MethodOf` and `TypeOf` by mistake.)
3. `sophia-interface`'s public types contain **no async-runtime and no database-driver types**, so the adapter works with every backend Sophia compares. (`sophia-store` already keeps them out: transactions are closures over object-safe traits; see [[backend]].)
4. Moonkale depends on `sophia-interface` by **git revision** at first. Sophia announces the revision when the crate is dependable.

Moonkale holds its own changes (Code Graph Schema §4/§5/§11 and WP-68 as the `sources/sophia` adapter) until these specifications are committed.

## 4. Rendering profiles through the interface

- The adapter is constructed with a **reader id** (Moonkale's user or profile id).
- Sophia looks up the reader's **active rendering profile in its own database** ([[SPEC-005 Rendering Profiles]]). There is no JSON or TOML file anywhere.
- The profile is exposed to Moonkale as one node, `sophia:Settings`, under the root. `fetch_text` on it prints the profile as text, and `WriteText` parses the text back into rows. The text is only a view; the rows are the truth. A parse error is a diagnostic, and the rows are unchanged.
- Changing the profile changes the text of every open definition and no id. The adapter reports it as a change in `changes_since`, so open editors re-fetch.
- Diffs, copies and published pages state the profile they were printed in (North Star §11).

## 5. Queries and their guard

- `Query::Text { dialect: "sql" }` runs on the views of [[SPEC-001 Sophia Store v0]] §11. The result is a `Table`, and id columns are also returned as nodes.
- It is **enforced read-only**:
  - a read-only connection, plus an authorizer that allows only `SELECT` on the documented views;
  - no `ATTACH`, no `PRAGMA` writes, no functions that touch files;
  - a row cap that sets `Table.truncated`;
  - a step budget.
- `Source::classify(dialect, text) → Risk` is implemented, so Moonkale's policy can gate queries written by agents.
- `datalog` later needs a new `TextDialect` variant, on Moonkale's side.
- For a non-SQLite backend, `sql` runs against the backend's equivalent views where it has them (DuckDB). Otherwise the query is refused, and the backend's own dialect (for example Cypher on LadybugDB) is offered as an additional dialect.

## 6. The change feed

`Source::changes_since(seq)` maps to the store's log ([[SPEC-001 Sophia Store v0]] §7):
- a name change → the `Name` node and its `Names` edge changed;
- a new definition → a new `Definition` node;
- a method-table change → `MethodOf` edges changed;
- a profile change → every definition's text changed.

Polling only in v0.

## 7. Elaboration on demand (later)

`Source::refresh(definition)` runs the configured checkers and derivations, and adds derived facts: typings, dispatch resolutions, test results. Each comes with `by`, `by_version`, `inputs` and `derived_at`. Moonkale shows stale facts as stale.

## 8. Many languages, and Sophia as Moonkale's engine (SR-74)

- A Sophia store can hold Rust, Julia or Python definitions normalised by Moonkale's WP-66 normaliser, stored as derived data with their file occurrence. The node mapping above already carries `lang` and `frontend`.
- If that path is taken, Moonkale's index could later use Sophia as its parsing and storage engine: the index would be one more client of `sophia-store`.
- Nothing in v0 depends on this. It is recorded so that v0 keeps it possible.

## 9. Acceptance (Moonkale WP-68)

1. The sample store opens as a source. Its namespaces, names and definitions draw in the graph view.
2. One definition is edited as text in the default profile, and its name moves to the new content id.
3. A parse error during `WriteText` shows as diagnostics in the editor and leaves the name unchanged.
4. A rename shows up through `changes_since`.
5. Switching the reader's profile (for example from `function` to `fn`, SPEC-005) reprints an open definition, and no id changes.
6. `sql` returns a table and a subgraph, and a write attempt is refused.

## Related

[[SPEC-001 Sophia Store v0]] · [[SPEC-005 Rendering Profiles]] · [[Moonkale Interface Needs]] · [[S-ADR-0006 Edits through projections and the existing operations]] · [[SP-004 Moonkale operations are file-shaped]]
