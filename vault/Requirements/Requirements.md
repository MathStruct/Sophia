#overview #design

> The requirements collected for Sophia, numbered `SR-nn`. Each one says where it came from: Daniel's own words (with date and place), or Moonkale's needs (tagged with the Moonkale section that raised it). The older design notes in `vault/Design/` are **not** a source of requirements; they are a first reading, possibly a wrong one ([[Reading of the Earlier Vault]]).
>
> Sources: Daniel's statements in [[The Original Idea]], `meta/Prompts/Prompt3.md`, Moonkale's `markdown/architecture/North Star.md` (Daniel, 2026-10-14/15), Moonkale's `ADR-0028` (Daniel, 2026-10-15), Daniel's message of 2026-10-17; the Moonkale coordinator's list of 2026-10-17 (summarised in [[Moonkale Interface Needs]]).
>
> Theory (CT-ML wiki): [Curry-Howard-Lambek Correspondence](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence) · [Attributed C-Set](https://mathstruct.org/CategoryTheory-ML-Wiki/Attributed-C-Set)

## How to read this page

- **Origin**: `D` means Daniel said it, with where and when. `M` means Moonkale needs it, with the Moonkale section in brackets. `A` means an agent proposed it and Daniel has not confirmed it.
- **When**: `v0` is needed for the base Sophia ([[SPEC-001 Base Sophia v0]], Moonkale WP-68). `later` is not needed for v0. `long` is the long-term direction and is not planned.
- **Status**: `open` (not addressed), `specified` (a spec answers it), `confirm` (needs Daniel's yes or no), `done`.
- Daniel adds and changes requirements. An agent only adds `A` rows and marks them `confirm`.

## From Daniel

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-01 | The statements of a program live in a database, not in files. Each one has a reproducible identity computed by hashing it together with the tree of its dependencies. | D: Original Idea; North Star, 2026-10-14 ("something like Unison") | v0 | specified (SPEC-001 §4) |
| SR-02 | Two different identities can be declared equivalent with a proof, instead of being merged. | D: Original Idea | later | open |
| SR-03 | Many languages share one graph: existing languages (Rust, Julia, Python, C/C++, Lean, …) are parsed into it, next to the future language. | D: North Star, amended 2026-10-14 | long | open. For v0, Moonkale's index covers file-backed languages and Sophia covers only its own definitions. |
| SR-04 | Types, functions and dataflow are all tracked, at least to some degree. | D: North Star, 2026-10-14 | v0 (types, functions); later (dataflow) | specified in part (SPEC-001 §3) |
| SR-05 | Dependent types: "a dependently typed Julia", or Julia combined with Lean. | D: Original Idea; North Star | v0 can store them, later can check them | specified in part: types are terms in the core (SPEC-001 §3) |
| SR-06 | Prolog-like elements: relations and rules are part of the language. | D: North Star, 2026-10-14; Lenticulum's LT-24 to LT-29 (statistical logic programs) | v0 (stored); later (run) | specified in part (SPEC-001 §3) |
| SR-07 | Several systems check the same term: dependent, cubical, algebraic (effects), and others. | D: North Star, 2026-10-14 | later | specified as structure only (SPEC-001 §8) |
| SR-08 | Annotations sit beside the code and are not compiled. They can be proofs (equivalence, implication, correctness, memory safety, safe concurrency), comments or Markdown. | D: Original Idea; North Star §7 | v0 (docs); later (proofs) | specified in part (SPEC-001 §9) |
| SR-09 | Tests are nodes attached to the code they test. | D: Original Idea | v0 | specified (`test` kind, SPEC-001 §3) |
| SR-10 | E-graphs, intermediate representations and DSLs all live inside the graph. | D: North Star, 2026-10-14 | later | open (SPEC-001 §10 sketches it) |
| SR-11 | Code compiles down to assembly, LLVM and MLIR. | D: Original Idea; North Star | later | open |
| SR-12 | You can query the exact context of a piece of code. | D: Original Idea | v0 | specified (SPEC-001 §11) |
| SR-13 | Anyone who disagrees can replace the core and migrate the database to their own schema. | D: Original Idea | later | specified in part (hash versioning and migration, SPEC-001 §4.5) |
| SR-14 | Improve Julia's precompilation. | D: Original Idea | long | open |
| SR-15 | Make old software usable instead of starting a new ecosystem. | D: Original Idea | long | open |
| SR-16 | **Implement in Rust only.** Julia is reserved for possible later experiments, such as machine learning on the code graph (graph kernels, GNNs, analytics), "maybe way later, but now not". | D: Prompt3; superseded by the answer to SPEC-001 Q3, 2026-10-17 | v0 | done ([[S-ADR-0004 Rust for the store, Julia for experiments]], accepted) |
| SR-24 | Later: machine learning and analytics on the code graph (graph kernels, GNNs), possibly in Julia, possibly with Sophia introspecting itself. | D: 2026-10-17 | long | open |
| SR-17 | The candidate databases are LadybugDB, HelixDB, TypeDB, FalkorDB, DuckDB and TursoDB. | D: Original Idea | v0 | confirm ([[S-ADR-0005 SQLite first behind a store trait]]) |
| SR-18 | Build a **base Sophia**, even a partly non-functional one, to find out what its definitions need from the store. | D: Moonkale ADR-0028 §3 and WP-68, 2026-10-15; repeated 2026-10-17 ("have a run at the database compiler") | v0 | specified (SPEC-001) |
| SR-19 | Syntax is the reader's setting: keywords, index base, blocks and operators are projections chosen in settings, and editable projections must parse back to the same graph. | D: North Star §11, 2026-10-14 (Moonkale WP-30) | later; v0 must not prevent it | specified as a constraint (SPEC-001 §6) |
| SR-20 | The code graph meets a knowledge graph: notes, papers and concepts link to code nodes and back. | D: North Star §9 | later | open |
| SR-21 | Sophia exists first to give Moonkale something real to design against. Unison can stand in until then. | D: message of 2026-10-17 | v0 | specified ([[S-ADR-0001 Sophia's first job is Moonkale's code store]]) |
| SR-23 | Keep both store-backed sources: Moonkale builds the Unison source (WP-18) **and** the Sophia source (WP-68). Neither replaces the other. | D: answer to SPEC-001 Q1, 2026-10-17 ("keep both, it should be little problem to do both") | v0 | done (decided) |
| SR-22 | Document extensively in the vault: decisions, specifications, problems, implementation plans, and the theory behind them. | D: message of 2026-10-17 | always | in progress |

## From Moonkale

Each row cites the Moonkale page or section it comes from. Moonkale will cite these numbers back from its Code Graph Schema.

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-30 | A Rust crate that a Moonkale source adapter wraps as a `moonkale_core::Source`. A server mode comes later. | M (reply §2; ADR-0005) | v0 | specified (SPEC-001 §12) |
| SR-31 | Browsing through `Query::{Node, Children, Neighbours, All}`. | M (reply §2; `core/src/source/query.rs`) | v0 | specified (SPEC-001 §12.2) |
| SR-32 | A named `Query::Text` dialect for Sophia's own query language, with a stated result shape (table, subgraph, or both). | M (reply §2; North Star §8) | v0: `sql` read-only; later: `datalog` | specified (SPEC-001 §11) |
| SR-33 | Structural edits through `Source::apply`: add, replace or delete a definition, rename (move a name), add or remove an annotation. Edits are revision-checked and return the new content ids. | M (reply §2; Code Graph Schema §5) | v0 (without annotations) | specified (SPEC-001 §12.3) |
| SR-34 | A canonical printer that turns a definition into text, plus a parser from text back into the graph that reports errors as diagnostics. | M (reply §2; ADR-0028) | v0 | specified (SPEC-001 §6) |
| SR-35 | Typechecking and elaboration on demand. The results are L3 nodes with provenance (`by`, `by_version`, `input`, `derived_at`), so stale results show as stale. | M (Code Graph Schema §7; North Star §5) | later (one simple checker in v0.x) | specified (SPEC-001 §8) |
| SR-36 | Provenance queries: what a node was lowered to, and what it was derived from. | M (Code Graph Schema §2, §7) | later | specified as structure only (SPEC-001 §10) |
| SR-37 | A change feed, `changes_since(seq)`. A push subscription is optional. | M (reply §2; `Source::changes_since`) | v0 | specified (SPEC-001 §7) |
| SR-38 | The descriptor declares `truth: Store`. | M (ADR-0028; `descriptor.rs`) | v0 | specified (SPEC-001 §12.1) |
| SR-39 | An embedded store that builds on Linux, macOS and Windows. The server opens it; the wasm client never does. The store can be swapped behind Sophia's own interface. | M (reply §3; P-144; ADR-0014) | v0 | specified (S-ADR-0005) |
| SR-40 | The content-id rule: what is hashed, whether types and annotations count, how members of a recursive group are ordered, and how the hash is versioned and migrated. | M (Code Graph Schema §4 and the WP-66 section) | v0 | specified (SPEC-001 §4) |
| SR-41 | Identity: is the content id enough, or does an edited definition need a stable handle, the way a file location gives one? | M (Code Graph Schema §4, §5) | v0 | specified (SPEC-001 §5; [[S-ADR-0003 Identity is content, names and lineage]]) |
| SR-42 | The definition kinds that apply to Sophia, and the namespaced `sophia:*` kinds it needs. | M (Code Graph Schema §3, §6) | v0 | specified (SPEC-001 §3) |
| SR-43 | The names model: namespaces, aliases, re-exports with visibility, what a rename does, and whether names have history or branches. | M (Code Graph Schema §5) | v0 (namespaces, aliases, rename); later (branches) | specified (SPEC-001 §5) |
| SR-44 | How one definition carries several typings: an L3 node or an annotation? | M (Code Graph Schema §6; North Star §5) | later | specified (SPEC-001 §8) |
| SR-45 | Which layers (L0 to L4+) Sophia owns, and what its IR layers and lowering edges are. | M (Code Graph Schema §2) | v0 (which layers); later (IR) | specified (SPEC-001 §2) |
| SR-46 | How e-classes and e-nodes map to nodes and edges; whether they are persisted or derived; whether an equivalence can point at a content id. | M (North Star §4) | later | sketched (SPEC-001 §10) |
| SR-47 | Proofs as annotations: the payload, the checker's identity and version, and how they go stale. | M (Code Graph Schema §6, §7; WP-19) | later | specified (SPEC-001 §9) |
| SR-48 | Holes and unresolved references: are they allowed in the store, and are they hashed? | M (Code Graph Schema §6 `Unresolved`; North Star §6) | v0 | specified (SPEC-001 §4.4) |
| SR-50 | Security: the `sql` dialect is read-only by enforcement (an authorizer, no `ATTACH`, a row cap, a step budget); the adapter classifies query risk (`Source::classify`); the parser is bounded in size, depth and count. | M (Security.md, the rules at its end; review of 2026-10-17) | v0 | specified (SPEC-001 §6.2, §11) |
| SR-49 | Acceptance for WP-68: a handful of Sophia definitions stored, named and drawn in Moonkale. | M (Work Packages, WP-68) | v0 | specified (SPEC-001 §14) |

## Changes

- 2026-10-17: SR-50 added from the Moonkale agent's review.
- 2026-10-17: page created. Daniel's statements collected from the sources above; Moonkale's needs from its coordinator's list.
