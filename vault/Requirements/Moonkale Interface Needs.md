#reference

> What Moonkale asked Sophia to specify. It is a summary of the Moonkale coordinator's message of 2026-10-17 (session `moonkale-1e`), with the answer in [[SPEC-001 Sophia Store v0]] for each item. Moonkale cites requirement numbers from [[Requirements]] back on its Code Graph Schema page.
>
> Sources: Moonkale repository `~/Code/Moonkale/Moonkale` (master): `markdown/architecture/Code Graph Schema.md`, `markdown/decisions/ADR-0028 Files or the graph.md`, `markdown/problems/Work Packages.md` (WP-18, 19, 21, 30, 65–68), `packages/core/src/source/{mod,query,transaction,descriptor}.rs`.

## 1. Schema: what is open, and where Unison is the placeholder

| Item | Question | Answered in |
|---|---|---|
| 1a | Content id: surface or core? Types and annotations in the hash? SCC order? Versioning and migration? | SPEC-001 §4 (SR-40) |
| 1b | Is the content id the only identity, or is a stable slot needed? | SPEC-001 §5.3, S-ADR-0003 (SR-41) |
| 1c | Which definition kinds apply; which `sophia:*` kinds | SPEC-003 §1 (SR-42) |
| 1d | Namespaces, aliases, re-exports, rename, name history | SPEC-001 §5 (SR-43) |
| 1e | Several typings: L3 node or annotation? | SPEC-001 §9 (SR-44) |
| 1f | Which layers Sophia owns; IR and lowering | SPEC-001 §2, §10 (SR-45) |
| 1g | E-classes and e-nodes; persisted or derived; equivalences on content ids | SPEC-001 §10 (SR-46) |
| 1h | Proofs as annotations: payload, checker, staleness | SPEC-001 §8 (SR-47) |
| 1i | Holes and unresolved references: stored? hashed? | SPEC-001 §4.4 (SR-48) |

## 2. Interface boundary

Moonkale prefers a Rust crate that implements `moonkale_core::Source`, with a server mode later.
- Reads go through `Query::{Node, Children, Neighbours, All}`, plus `Query::Text` with a named dialect.
- Edits go through `apply(Transaction)`: revision-checked, returning the new content ids.
- Also needed: on-demand elaboration with provenance, a printer and a parser with diagnostics, provenance queries, `changes_since(seq)`, projections that parse back (WP-30), and `truth: Store`.
- Answered in SPEC-004 and SPEC-005 (SR-30 to SR-38).

## 3. Storage

There is no hard requirement. It must be embedded, build on Linux, macOS and Windows, be opened by the server and never by wasm, and be swappable. LadybugDB is Linux-only for now (P-144); Moonkale's own state is in SQLite (ADR-0014). Answered in S-ADR-0005 (SR-39).

## 4. First versus later

- **First** (WP-68 and the deferred question of ADR-0028): the definition node, the content-id rule, names, a crate with read/apply/print, and a handful of definitions stored, named and drawn.
- **Later**: typings and L3, e-graphs, proofs (with WP-19), IR, subscriptions, rendering projections.
- **Moonkale's open questions**:
  - Does a store-backed definition differ from a file-backed one only in lacking `Defines`? SPEC-001 §2 says no: also its `NodeId`, its `status` and its lineage.
  - Does Sophia replace WP-18 (Unison) as the schema test? **No. Daniel, 2026-10-17: keep both** (SR-23).

## 5. Numbering

Sophia uses `SR-nn`, `S-ADR-nnnn`, `SPEC-nnn` and `SP-nnn`. Moonkale's next free numbers (ADR-0029, P-211) are Moonkale's own and are not used here.

## Review of SPEC-001 (Moonkale agent, 2026-10-17)

No fundamental contradiction with the Code Graph Schema. The corrections, all applied to SPEC-001:
- **Descriptor:** `family: Graph` (there is no `Database` family, and no new `Code` family in v0); `read: true`. `datalog` needs a new `TextDialect` variant, on Moonkale's side.
- **Edges:** terms hang off `sophia:Subterm`, not `Contains`. Annotations use Moonkale's existing `NodeKind::Annotation` and `EdgeKind::Annotates`, with `sophia_kind` as a property.
- **Annotation ownership:** proofs, docs and equivalences live in Sophia. Moonkale's review comments stay in Moonkale, anchored to the lineage.
- **Names** are primary for a Sophia source. Moonkale's filter that hides `Name` nodes needs adjusting on Moonkale's side.
- **Operations:**
  - re-ingest is a no-op;
  - opening a definition for editing resolves it to a name by a fixed rule;
  - `/` ↔ `.` path mapping, with rejected characters listed;
  - a parse error leaves the name unchanged.
- **Identity:** the SCC rules are copied exactly from WP-66 v1. The algorithm is shared, but ids are never equal across Moonkale and Sophia. A citation was fixed.
- **Security:** `sql` is enforced read-only, with a row cap and a step budget; `classify` is implemented; the parser is bounded (SR-50).
- **Storage:** `rusqlite` with the bundled feature, built for the NDK; no wasm32 build is needed.

Moonkale's side, once Daniel accepts: update Code Graph Schema §4, §5 and §11; turn WP-68 into the `sources/sophia` adapter work package; cite SR-30 to SR-50.
