#overview #design

> The requirements collected for Sophia, numbered `SR-nn`. Each one says where it came from: Daniel's own words (with date and place), or Moonkale's needs (tagged with the Moonkale section that raised it). The older design notes in `vault/Design/` are **not** a source of requirements; they are a first reading, possibly a wrong one ([[Reading of the Earlier Vault]]).
>
> Sources: Daniel's statements in [[The Original Idea]], `meta/Prompts/Prompt3.md`, Moonkale's `markdown/architecture/North Star.md` (Daniel, 2026-10-14/15), Moonkale's `ADR-0028` (Daniel, 2026-10-15), Daniel's message of 2026-10-17; the Moonkale coordinator's list of 2026-10-17 (summarised in [[Moonkale Interface Needs]]).
>
> Theory (CT-ML wiki): [Curry-Howard-Lambek Correspondence](https://mathstruct.org/CategoryTheory-ML-Wiki/Curry-Howard-Lambek-Correspondence) · [Attributed C-Set](https://mathstruct.org/CategoryTheory-ML-Wiki/Attributed-C-Set)

## How to read this page

- **Origin**: `D` means Daniel said it, with where and when. `M` means Moonkale needs it, with the Moonkale section in brackets. `A` means an agent proposed it and Daniel has not confirmed it.
- **When**: `v0` is needed for the base Sophia ([[SPEC-001 Sophia Store v0]], Moonkale WP-68). `later` is not needed for v0. `long` is the long-term direction and is not planned.
- **Status**: `open` (not addressed), `specified` (a spec answers it), `confirm` (needs Daniel's yes or no), `done`.
- Daniel adds and changes requirements. An agent only adds `A` rows and marks them `confirm`.

## From Daniel

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-01 | The statements of a program live in a database, not in files. Each one has a reproducible identity computed by hashing it together with the tree of its dependencies. | D: Original Idea; North Star, 2026-10-14 ("something like Unison") | v0 | specified (SPEC-001 §4) |
| SR-02 | Two different identities can be declared equivalent with a proof, instead of being merged. | D: Original Idea | later | open |
| SR-03 | Many languages share one graph: existing languages (Rust, Julia, Python, C/C++, Lean, …) are parsed into it, next to the future language. | D: North Star, amended 2026-10-14 | long | open. For v0, Moonkale's index covers file-backed languages and Sophia covers only its own definitions. |
| SR-04 | Types, functions and dataflow are all tracked, at least to some degree. | D: North Star, 2026-10-14 | v0 (types, functions); later (dataflow) | specified in part (SPEC-003 §1–2) |
| SR-05 | Dependent types: "a dependently typed Julia", or Julia combined with Lean. | D: Original Idea; North Star | v0 can store them, later can check them | specified in part: types are terms in the core (SPEC-003 §1–2) |
| SR-06 | Prolog-like elements: relations and rules are part of the language. | D: North Star, 2026-10-14; Lenticulum's LT-24 to LT-29 (statistical logic programs) | v0 (stored); later (run) | specified in part (SPEC-003 §1–2) |
| SR-07 | Several systems check the same term: dependent, cubical, algebraic (effects), and others. | D: North Star, 2026-10-14 | later | specified as structure only (SPEC-001 §8) |
| SR-08 | Annotations sit beside the code and are not compiled. They can be proofs (equivalence, implication, correctness, memory safety, safe concurrency), comments or Markdown. | D: Original Idea; North Star §7 | v0 (docs); later (proofs) | specified in part (SPEC-001 §9) |
| SR-09 | Tests are nodes attached to the code they test. | D: Original Idea | v0 | specified (`test` kind, SPEC-003 §1–2) |
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
| SR-19 | Syntax is the reader's setting: keywords, index base, blocks and operators are projections chosen in settings, and editable projections must parse back to the same graph. | D: North Star §11, 2026-10-14 (Moonkale WP-30) | later; v0 must not prevent it | specified as a constraint (SPEC-005) |
| SR-20 | The code graph meets a knowledge graph: notes, papers and concepts link to code nodes and back. | D: North Star §9 | later | open |
| SR-21 | Sophia exists first to give Moonkale something real to design against. Unison can stand in until then. | D: message of 2026-10-17 | v0 | specified ([[S-ADR-0001 Sophia's first job is Moonkale's code store]]) |
| SR-23 | Keep both store-backed sources: Moonkale builds the Unison source (WP-18) **and** the Sophia source (WP-68). Neither replaces the other. | D: answer to SPEC-001 Q1, 2026-10-17 ("keep both, it should be little problem to do both") | v0 | done (decided) |
| SR-22 | Document extensively in the vault: decisions, specifications, problems, implementation plans, and the theory behind them. | D: message of 2026-10-17 | always | in progress |

## From Daniel, 2026-10-20

From Daniel's message of 2026-10-20, which answers SPEC-001 Q2 and adds the language's principles. Rows SR-25 and onwards. In the Origin column, "D 10-20" means that message.

### Store and language

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-25 | **Sophia is the store; sophialang is the store's own language.** Keep them separate: every programming language should eventually live in the store, and sophialang is one of them. | D 10-20 | v0 | specified ([[SPEC-001 Sophia Store v0]], [[SPEC-002 Sophialang Principles]]) |
| SR-26 | Answer whether the canonical form should be compressed beyond normalisation, so that it is no longer human-readable, and whether that helps higher-order code, JIT compilation and macros. If it has serious benefits, plan for it. | D 10-20 | v0 (answer); later (build) | answered ([[Compression and Canonical Form]]) |
| SR-27 | The surface syntax is Julia-flavoured by default. | D 10-20 (answer to Q2) | v0 | specified ([[SPEC-005 Rendering Profiles]]) |
| SR-28 | How code is displayed depends **entirely** on the user's declared settings (for example `fn`, `def` or `function`; 0- or 1-based indexing). The settings are stored **in the database**, not in JSON. | D 10-20 | v0 (stored, default profile); later (other profiles) | specified ([[SPEC-005 Rendering Profiles]]) |
| SR-29 | When in doubt about which language's behaviour to follow, follow Julia's. | D 10-20 | always | specified ([[SPEC-002 Sophialang Principles]] §1) |

### Sophialang principles

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-51 | **Autoresolve, unsafe, safe.** What is left unspecified is resolved automatically. Explicit control goes as far as letting you shoot yourself in the foot (unsafe). An unsafe section is then made safe, or mostly safe, by a proof written in a DSL. | D 10-20 | design now | specified (SPEC-002 §2) |
| SR-52 | No element of sophialang makes compiled code inherently slower than C++, Rust or Julia. | D 10-20 | design now | specified (SPEC-002 §3) |
| SR-53 | Dependent types can be swapped for another type system when compiling. Correctness and other results are stored in the graph and do not affect the performance of the compiled program. | D 10-20 | design now | specified (SPEC-002 §3, §4) |
| SR-54 | A test is not re-run when the changes since its last run verifiably do not influence it. | D 10-20 | v0 (by content id); later (finer) | specified (SPEC-001 §8; SPEC-002 §8) |
| SR-55 | Sophialang is written in sophialang as soon as possible (self-hosting). | D 10-20 | after the core runs | planned ([[Milestones]]) |
| SR-56 | Memory: a garbage collector when nothing is specified, preferably running on another core without disturbing the main thread; explicit management like C++ when asked; and a proof DSL showing the explicit management is correct (like Rust's borrow checker, but accepting more cases). | D 10-20 | design now; build later | specified (SPEC-002 §5) |
| SR-57 | Concurrency: resolved automatically when unspecified; an explicit way; concurrency proofs. | D 10-20 | design now; build later | specified (SPEC-002 §6) |
| SR-58 | JIT: compile when necessary, plus a DSL to prove that a section will not trigger a sudden JIT compilation. | D 10-20 | design now; build later | specified (SPEC-002 §7) |
| SR-59 | Polymorphism is multiple dispatch, unless there is a better reason for something else. | D 10-20 | design now | specified (SPEC-002 §4) |
| SR-60 | A compiled sophialang program can be obtained as a minimal binary (or other target) that does not need the Sophia machinery, as in the embedded case. | D 10-20 | later | specified ([[Minimal Binaries]]) |
| SR-61 | A wiki article that systematises types: machine types (`Int64`, `Float64`, …), abstract types (`Union{}`, `Nothing`, `Any`), types that are only a UUID or hash, and the type systems that build further types (algebraic, dependent, cubical, linear, …), with each one's place in Sophia. | D 10-20 | v0 (doc) | done ([[Type Taxonomy]]) |

### Tooling, store and research

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-62 | AI agents reach sophialang through a frontend, because no model is trained on its canonical form, and they have to cope with mixed-language content. Once existing languages are parsed in, models could be trained on sophialang directly. | D 10-20 | v0 (projection); later (training) | specified ([[LLMs and Mixed-Language Code]]) |
| SR-63 | **Priority: the interface to Moonkale first**, then each individual language parsed into the store. | D 10-20 | v0 | specified ([[SPEC-004 Moonkale Interface]]) |
| SR-64 | Try several databases and compare them. | D 10-20 | v0 | planned ([[Benchmark Plan]], [[Database Comparison]]) |
| SR-65 | Metamath and a Lisp are easy cases that could come first, but sophialang is preferred. | D 10-20 | v0 | decided: sophialang first; a Lisp as an optional second frontend ([[Milestones]]) |
| SR-66 | Precompilation needs analytics on top of the store (which statements are compiled most often). It is a goal, but not the first priority. | D 10-20 | later | open |
| SR-67 | A wiki article on computational trinitarianism, extensively, with its benefit to this project, the DSLs it yields, and how their statements are embedded in the code graph. | D 10-20 | v0 (doc) | done ([[Computational Trinitarianism]]) |
| SR-68 | Document how the operating system could hinder or help Sophia: LispOS (Strandh), database operating systems. | D 10-20 | v0 (doc) | done ([[Operating Systems and Sophia]]) |
| SR-69 | Would dedicated Lisp hardware help? What would it require, and what would it look like? | D 10-20 | v0 (doc) | done ([[Hardware for Sophia]]) |
| SR-70 | Look at Unison's specifications and adopt what applies. | D 10-20 | v0 | done ([[Adopting from Unison]]) |
| SR-71 | Given the processor's cache behaviour, what block size suits the language, and what hash-to-data ratio is efficient? | D 10-20 | v0 (analysis); benchmarks decide | analysed ([[Block Size and Hash Overhead]]) |
| SR-72 | Keep an extensive benchmarking suite and benchmark documents. | D 10-20 | v0 onwards | in progress ([[Benchmark Plan]], `crates/sophia-bench`) |
| SR-73 | This is an experiment. Record **why things fail** as carefully as what works. The dream is to abolish programming languages, or at least to reuse code seriously, with few downsides. | D 10-20 | always | ongoing ([[Problem Log]], [[Open Problems and Risks]]) |
| SR-74 | Sophia may later become Moonkale's main engine for parsing and storing code. | D 10-20 | long | open (SPEC-004 §8) |

## From the external review, 2026-10-20

An external review (ChatGPT) of commit `27cb3a7`, passed on by Daniel, who asked for more requirements and specifications to be drawn from it. Triage: [[Review 2026-10-20 (external)]]. In the Origin column, "Rev" means that review.

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-75 | A recursive group with structurally symmetric members is stored, reloaded and verified without loss, stable under renaming and member permutation, on every backend. | Rev §2, demonstration 1 | v0 | done (quotient; `tests/symmetric.rs`) |
| SR-76 | Builds, tests, proofs and dispatch resolutions are evaluated against an **immutable, content-addressed world snapshot**, not the current mutable world. | Rev §3, demonstration 3 | v0 (store); M4 (use) | done in the store; use specified (SPEC-006 §2) |
| SR-77 | An editable text carries a token covering everything its save would change. Two views cannot silently overwrite each other's work or an unintended alias. When a bare definition is opened for editing, the editor keeps the navigation context or asks which name to edit. | Rev §1, demonstration 2 | v0 | done on Sophia's side (SPEC-004 §3b); Moonkale's side open |
| SR-78 | SophiaLang has a **mandatory** well-formedness and execution semantics independent of optional checkers, and a representation contract no optional type system may contradict. | Rev §4, §5.1 | v0 (spec) | specified (SPEC-002 §3a, SPEC-003 §7) |
| SR-79 | Proofs never change a program's semantic identity, but may change **artifact identity**. The artifact records the evidence the compiler used. | Rev §4 | M5 | specified (SPEC-006 §3) |
| SR-80 | Learned parameters are content-addressed **parameter artifacts**. A model instance is (program id, parameter artifact id). Training is a replayable derived fact. | Rev §5.2 | with execution (M4) | specified (SPEC-006 §4) |
| SR-81 | The core distinguishes pure from effectful computation, with a reserved path for I/O, exceptions, state, nondeterminism, concurrency and staging. Call-by-push-value is evaluated. | Rev §5.3 | design now | specified as a boundary (SPEC-003 §8) |
| SR-82 | Proof trust is recorded per proof: checker, trusted kernel, assumptions, certificate, versions, world. Policies say which checkers they trust. | Rev §8.4 | M4/M6 | specified (SPEC-006 §5) |
| SR-83 | Benchmarks include real repositories and Moonkale's **edit-to-updated-graph latency**, with cold and warm caches, allocations, resident memory and latency distributions. | Rev §7 | M2/M3 | planned (B-15, B-16) |
| SR-84 | Cross-system and cross-language claims go through explicit interpretations that state which properties they preserve, with proof obligations. | Rev §6 | later | specified ([[Computational Trinitarianism]] §0) |
| SR-85 | Invalidation after a method-table change is minimal, by signature overlap, not "any change invalidates all". | Rev §3 | M4 | open (SP-006, B-09) |
| SR-86 | Macro hygiene, phases and quotation are specified, not assumed from de Bruijn indices. | Rev §8.1 | with macros | open |
| SR-87 | A closed-world contract governs standalone binaries: reflection, dynamic loading, foreign calls and generated code. | Rev §8.2 | M5 | specified (SPEC-006 §3) |
| SR-88 | Migrations separate canonical identity (normaliser, hash) from physical encoding. | Rev §8.7 | later | specified (SPEC-006 §6) |
| SR-89 | **P0 gate:** foundational invariants count as settled only after the three acceptance demonstrations pass: symmetric recursion, two-view editing, reproducible dispatch. | Rev §9, §10 | now | Sophia side: demonstrations 1 and 3 (store level) pass; 2 passes on Sophia's side, Moonkale's client side open |

## From Moonkale

Each row cites the Moonkale page or section it comes from. Moonkale will cite these numbers back from its Code Graph Schema.

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-30 | A Rust crate that a Moonkale source adapter wraps as a `moonkale_core::Source`. A server mode comes later. | M (reply §2; ADR-0005) | v0 | specified (SPEC-004 §1–3) |
| SR-31 | Browsing through `Query::{Node, Children, Neighbours, All}`. | M (reply §2; `core/src/source/query.rs`) | v0 | specified (SPEC-004 §2) |
| SR-32 | A named `Query::Text` dialect for Sophia's own query language, with a stated result shape (table, subgraph, or both). | M (reply §2; North Star §8) | v0: `sql` read-only; later: `datalog` | specified (SPEC-001 §11) |
| SR-33 | Structural edits through `Source::apply`: add, replace or delete a definition, rename (move a name), add or remove an annotation. Edits are revision-checked and return the new content ids. | M (reply §2; Code Graph Schema §5) | v0 (without annotations) | specified (SPEC-004 §1–3.3) |
| SR-34 | A canonical printer that turns a definition into text, plus a parser from text back into the graph that reports errors as diagnostics. | M (reply §2; ADR-0028) | v0 | specified (SPEC-005) |
| SR-35 | Typechecking and elaboration on demand. The results are L3 nodes with provenance (`by`, `by_version`, `input`, `derived_at`), so stale results show as stale. | M (Code Graph Schema §7; North Star §5) | later (one simple checker in v0.x) | specified (SPEC-001 §8) |
| SR-36 | Provenance queries: what a node was lowered to, and what it was derived from. | M (Code Graph Schema §2, §7) | later | specified as structure only (SPEC-001 §10) |
| SR-37 | A change feed, `changes_since(seq)`. A push subscription is optional. | M (reply §2; `Source::changes_since`) | v0 | specified (SPEC-001 §7) |
| SR-38 | The descriptor declares `truth: Store`. | M (ADR-0028; `descriptor.rs`) | v0 | specified (SPEC-004 §1) |
| SR-39 | An embedded store that builds on Linux, macOS and Windows. The server opens it; the wasm client never does. The store can be swapped behind Sophia's own interface. | M (reply §3; P-144; ADR-0014) | v0 | specified (S-ADR-0005) |
| SR-40 | The content-id rule: what is hashed, whether types and annotations count, how members of a recursive group are ordered, and how the hash is versioned and migrated. | M (Code Graph Schema §4 and the WP-66 section) | v0 | specified (SPEC-001 §4) |
| SR-41 | Identity: is the content id enough, or does an edited definition need a stable handle, the way a file location gives one? | M (Code Graph Schema §4, §5) | v0 | specified (SPEC-001 §5; [[S-ADR-0003 Identity is content, names and lineage]]) |
| SR-42 | The definition kinds that apply to Sophia, and the namespaced `sophia:*` kinds it needs. | M (Code Graph Schema §3, §6) | v0 | specified (SPEC-003 §1–2) |
| SR-43 | The names model: namespaces, aliases, re-exports with visibility, what a rename does, and whether names have history or branches. | M (Code Graph Schema §5) | v0 (namespaces, aliases, rename); later (branches) | specified (SPEC-001 §5) |
| SR-44 | How one definition carries several typings: an L3 node or an annotation? | M (Code Graph Schema §6; North Star §5) | later | specified (SPEC-001 §8) |
| SR-45 | Which layers (L0 to L4+) Sophia owns, and what its IR layers and lowering edges are. | M (Code Graph Schema §2) | v0 (which layers); later (IR) | specified (SPEC-001 §2) |
| SR-46 | How e-classes and e-nodes map to nodes and edges; whether they are persisted or derived; whether an equivalence can point at a content id. | M (North Star §4) | later | sketched (SPEC-001 §10) |
| SR-47 | Proofs as annotations: the payload, the checker's identity and version, and how they go stale. | M (Code Graph Schema §6, §7; WP-19) | later | specified (SPEC-001 §8) |
| SR-48 | Holes and unresolved references: are they allowed in the store, and are they hashed? | M (Code Graph Schema §6 `Unresolved`; North Star §6) | v0 | specified (SPEC-001 §4.4) |
| SR-50 | Security: the `sql` dialect is read-only by enforcement (an authorizer, no `ATTACH`, a row cap, a step budget); the adapter classifies query risk (`Source::classify`); the parser is bounded in size, depth and count. | M (Security.md, the rules at its end; review of 2026-10-17) | v0 | specified (SPEC-005.2, §11) |
| SR-49 | Acceptance for WP-68: a handful of Sophia definitions stored, named and drawn in Moonkale. | M (Work Packages, WP-68) | v0 | specified (SPEC-004 §9) |

## Changes

- 2026-10-20: SR-75 to SR-89 added from the external review.
- 2026-10-20: SR-25 to SR-29 and SR-51 to SR-74 added from Daniel's message of 2026-10-20 (the SR-25 to SR-29 block was still free).
- 2026-10-17: SR-50 added from the Moonkale agent's review.
- 2026-10-17: page created. Daniel's statements collected from the sources above; Moonkale's needs from its coordinator's list.
