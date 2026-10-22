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

## From Daniel's query-language proposal, 2026-10-20

Daniel proposed a native query language and forwarded an external review of the proposal as his direction ([[Review 2026-10-20 Query Language (external)]]). "Q §n" cites the review's sections; "Q msg" its suggested message, which Daniel sent on.

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-90 | Sophia has its **own storage-independent query algebra and query language**. SQL is one frontend or translation target, not the fundamental interface. | Q §1, Q msg | now | done (SPEC-007; [[S-ADR-0011 Sophia has its own query algebra]]) |
| SR-91 | Queries are first-class Sophia statements, stored in the graph. | Q §3, Q msg 1 | now | done (SPEC-007 §8) |
| SR-92 | Queries have a canonical representation and stable content ids; their physical encoding may be compressed. | Q §3, Q msg 2 | now | done, except literal order (SPEC-007 §4.1, SP-009) |
| SR-93 | Proofs, optimisations, compiled implementations and benchmark results can be attached to queries. | Q §3, Q msg 3 | later | possible today through annotations and facts on the query id (SPEC-007 §8); nothing produces them yet |
| SR-94 | Query results are identified by query, snapshot and parameters, not by the query's hash. | Q §3, Q msg 4 | now | done, with a coarse snapshot (SPEC-007 §4.3, SP-010) |
| SR-95 | Several syntax frontends, Julia-like by default. | Q §5, Q msg 5 | now | done: `julia` (parse and print), `datalog` (print) (SPEC-007 §5) |
| SR-96 | Query semantics are independent of storage. Backends expose access paths and capabilities; the planner decides. The algebra depends on neither SQL nor LMDB's key order. | Q §1, §6, Q msg 6 | now | done (SPEC-007 §2, §6; all four backends) |
| SR-97 | The query DSL can eventually be implemented in sophialang itself. | Q msg 7 | M6 | planned (SPEC-007 §9, S-ADR-0012) |
| SR-98 | Query planning, optimisation and incremental view maintenance are extensible components. | Q msg 8 | now (planner, rewrite); later (incremental) | partly done: greedy planner, magic sets as IR → IR (SPEC-007 §6) |
| SR-99 | Ordinary sophialang evaluation and declarative relational query semantics stay distinct; queries are a typed DSL `Query{S, A, E}`. | Q §5, Q msg 9 | now | done (SPEC-007 §3.4; [[S-ADR-0012 The query IR is a separate IR on shared primitives]]) |
| SR-100 | Moonkale and AI agents use the same typed query interface, not backend-specific SQL. | Q msg 10 | now (Moonkale); M3b (agents) | done for Moonkale (the `query` dialect, SPEC-007 §8) |
| SR-101 | Investigate storing Sophia's canonical graph almost directly as LMDB values: binary encoding, but no JSON or SQL layer; zero-copy is an optimisation, never a correctness requirement. | Q §4, Q msg | now (measure) | measured ([[Zero-Copy Storage of Canonical Graphs]]) |
| SR-102 | Datalog (recursive rules) is part of the query algebra from the start, even if an executor supports only a subset. | Q §6 | now | done |
| SR-103 | Study TypeQL, CQL and Datalog; adopt ideas, not systems. | Q §2, Q msg | now | done ([[Query Languages for Sophia]]) |
| SR-104 | No permanent backend before Moonkale's edit-latency benchmarks; LMDB's single writer is measured before it serves a multi-agent server. | Q §6, Q msg | before the choice | accepted ([[S-ADR-0010 Primary backend and an analytics replica]]) |
| SR-105 | The query IR and Moonkale integration come before more storage backends; no new database engine is built. | Q msg | now | followed |
| SR-106 | Long-term: program and query optimisation converge. Queries become compiled functions, functions are partially evaluated against database state, proofs establish query equivalences, materialised views are reusable computations. Only after a small, correct algebra works on at least two backends. | Q msg (end) | later | direction (SPEC-007 §11) |

## From the M3a review, 2026-10-20

From the external review of `d0c08b4` ([[Review 2026-10-20 M3a (external)]]), forwarded by Daniel; "R3 §n" cites its sections. Daniel added: nobody uses Moonkale, Lenticulum or Sophia yet, so big alterations are fine.

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-107 | A cached result is stored under the id of exactly the snapshot it was computed from; result identity is semantic, and execution policy (limits) never changes what a hit returns beyond the caller's own row cap. | R3 §3, §4 | now | done (SPEC-007 §4.3) |
| SR-108 | Queries run under a resource budget: steps, rows, derived tuples, estimated bytes, wall time, cancellation. Untrusted (agent) queries additionally get a hard memory bound from outside the executor. | R3 §5 | now (budget); M3b (hard bound) | budget done (SPEC-007 §6.0); hard bound open |
| SR-109 | Canonicalisation guarantees canon(πQ) = canon(Q) for every transformation it claims to normalise, or refuses; the guarantee is property-tested. | R3 §7 | now | done (SPEC-007 §4.1) |
| SR-110 | Every optimisation is differentially tested against unoptimised evaluation on random programs and data; later, it carries checkable certificates. | R3 §8 | now (tests); later (certificates) | done for magic sets and the SQL translation |
| SR-111 | Queries call typed, pure, cost-annotated functions through one mechanism, not a growing list of built-ins. | R3 §6 | later | design (SPEC-007 §11) |
| SR-112 | One record shape for transformations and their evidence, shared by every optimiser and frontend. | R3 §10 | interface now, build later | specified (SPEC-006 §6a) |
| SR-113 | Real-code benchmarks and Moonkale's edit loop come before the agent server. | R3 §9 | now | adopted ([[Milestones]]) |

## From Daniel, 2026-10-20 (the three projects)

Daniel: "Moonkale was meant as an editor for Lenticulum.jl. Now Sophia is somewhat intermingled with it, so it would be a good idea to think the three things together." Also: "no one really uses Moonkale, Lenticulum or Sophia right now, so you may make really big alterations later."

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-114 | Moonkale, Lenticulum.jl and Sophia are designed together: Moonkale edits, Sophia stores (and later runs), Lenticulum is the program. | Daniel | now | plan ([[Real Code Corpora]] §1) |
| SR-115 | Lenticulum.jl runs in Sophia: a goal, but a later one; easier Julia packages first. | Daniel | later | planned (E-9, after the semantic-boundary experiments, SR-124) |
| SR-116 | Sophia's Julia (and Rust, Python) ingestion uses the same normal form as Moonkale's WP-66 content ids, ideally the same code. | follows from SR-114 | rung 2 | in use (2026-10-20): the shared crate `moonkale-content-id` gives identical ids on Moonkale's golden fixtures; one resolver difference reported |
| SR-117 | Real-code corpora: linear algebra (BLAS-like, in Julia and Rust), Metamath, then Lean. | Daniel | now | Metamath done; ladder in [[Real Code Corpora]] |
| SR-118 | Large breaking changes are acceptable while there are no users: prefer the right design over compatibility. | Daniel | now | adopted |

## From Daniel and the real-code review, 2026-10-20

Daniel's requests with the fourth external review ([[Review 2026-10-20 Real Code (external)]], "R4 §n"). Daniel: "keep extensive documentation in the vault on what statements should actually be saved in the database and what intermediate statements can be cheaply generated. I kinda don't mind if for a short time the memory usage goes high, but once the thing is computed not everything needs to be stored." And: "a folder for each language where the parsing and interoperability with sophialang is documented." On LuxCore as the next step: it "proves relatively little as it avoids possibly the AD and Reactant or other backend libraries that full Lux.jl brings with it."

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-119 | The vault documents, per kind of object, what is persisted and what is computed and discarded: identity and provenance, reconstruction cost, reproducibility, invalidation, materialisation and eviction, garbage collection, compression, benchmarks. | Daniel | now | done ([[Storage and Materialization]]) |
| SR-120 | High transient memory is acceptable if released after the computation; permanent storage holds only what cannot be cheaply had again. | Daniel | now | policy ([[Materialization Policy]]); queries already budgeted |
| SR-121 | An object's logical identity is separate from whether its bytes are materialised; dropping bytes never changes an id. | Daniel, R4 | now | invariant ([[Storage and Materialization]]) |
| SR-122 | One vault folder per language (sophialang, Julia, Rust, Python, Lisp, Metamath, Lean, …) documenting parsing, canonicalisation, name resolution, types, runtime semantics, interoperability with sophialang, compilation and limitations. | Daniel | now | done ([[Languages]]) |
| SR-123 | Support is stated per level (parsed, normalised, resolved, lowered, executable, verified) and per feature, with evidence; never as a percentage of definitions with ids. | Daniel, R4 | now | adopted ([[Languages]]) |
| SR-124 | Experiments target semantic boundaries (dispatch, macros, AD, compiler transformations, Reactant/MLIR) rather than ingesting ever larger packages; full Lux.jl later, LuxCore alone not; Lenticulum after. Each experiment has hypotheses, failure criteria and measurements. | Daniel, R4 §6 | now | plan ([[Semantic Boundary Experiments]]) |
| SR-125 | Moonkale integration and editing performance remain priorities. | Daniel | now | B-15 waits for Moonkale's revision |
| SR-126 | For now: specifications, small experiments and benchmarks, not new major subsystems; no automatic caching system before workloads are measured; document before changing the storage layout. | Daniel | now | adopted |
| SR-127 | A generic function is identified by (module, binding name), never by name alone; methods that extend another module's function belong to it. | R4 §4 | now | done ([[Julia Name Resolution]]; `tests/generics.rs`) |
| SR-128 | Proof-dependency answers are checked against an independent implementation before they are trusted. | R4 §2 | now | done for Metamath (`sophia-mm-check` vs metamath.exe) |
| SR-129 | A semantic conformance suite per language compares Sophia's facts with the language implementation's, classifying each as known, approximated, left open or wrong. | R4 §6 | now | Julia parts 1–2 done ([[Julia Conformance]]) |
| SR-130 | Generic identity, method table at a world, and dispatch result are distinct objects; derived and compiled code that depends on dispatch cites the world. | R4 §5 | now (spec) | specified (SPEC-006 §1a) |
| SR-131 | A record's kind follows from its content; a conflicting kind is refused (SP-012). | R4 §7 | now | done (`StoreError::KindConflict`) |
| SR-132 | Physical storage may use compact local keys for edges; content hashes stay the public identity. | R4 §3 | later (measure first) | proposed ([[Compression]], SP-011) |

## From the second real-code review, 2026-10-20

From the review of `85189737` ([[Review 2026-10-20 Real Code 2 (external)]], "R5 §n"), forwarded by Daniel.

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-133 | Before any canonical or derived bytes are dropped, every record must be rebuildable from immutable stored sources plus a versioned manifest (frontend, normaliser, paths, environment), and verified against its stored identity; tested on all backends. | R5 §1, §3 | now | done for module-mode Julia and Metamath (`source.rs`, `tests/reconstruct.rs`) |
| SR-134 | Authoritative sources, canonical representations, derived indexes and cached artifacts stay separate categories. | R5 §5 | now | adopted ([[Object Categories]]) |
| SR-135 | AD and compiler measurements separate transformation, compilation, execution and restoration time. | R5 §2 | now | done for Reactant (E-AD-1) |
| SR-136 | Persistent artifact reuse is claimed only after an artifact has been restored and run in a fresh process with the same result. | R5 §2 | now | Reactant: shown; Julia pkgimage + Enzyme: fails |
| SR-137 | For Lean, source syntax, elaborated terms, kernel declarations and proof objects are distinct objects; every claim of verification names its checker and where it ran. | R5 §4 | now | specified ([[Lean Overview]]) |
| SR-138 | S-ADR-0012 accepted: the query algebra stays a separate IR sharing identity, types and evidence infrastructure with sophialang. | R5 §1 | now | accepted |
| SR-139 | Next order: finish Mathlib and Reactant; reconstruction before compaction; Moonkale's end-to-end edit benchmark; cross-session artifacts; compact edges; only then other compilers or the agent server. | R5 §4 | now | adopted ([[Semantic Boundary Experiments]]) |
| SR-140 | WebAssembly as a compilation target, "for way later". Two readings, both open: (a) sophialang code compiled to WASM (a backend beside the planned ones; WASM's validation and sandbox make the artifact checkable and portable, an input-addressed artifact in the sense of SPEC-006 §3); (b) Sophia's own crates built for wasm32 (a store or the query engine in a browser). (b) conflicts with SR-39 / S-ADR-0005 (the wasm client never opens the store) and would need its own decision. | D (2026-10-22) | later | recorded; not planned before SR-139's order is done |

## From Moonkale

Each row cites the Moonkale page or section it comes from. Moonkale will cite these numbers back from its Code Graph Schema.

| # | Requirement | Origin | When | Status |
|---|---|---|---|---|
| SR-30 | A Rust crate that a Moonkale source adapter wraps as a `moonkale_core::Source`. A server mode comes later. | M (reply §2; ADR-0005) | v0 | specified (SPEC-004 §1–3) |
| SR-31 | Browsing through `Query::{Node, Children, Neighbours, All}`. | M (reply §2; `core/src/source/query.rs`) | v0 | specified (SPEC-004 §2) |
| SR-32 | A named `Query::Text` dialect for Sophia's own query language, with a stated result shape (table, subgraph, or both). | M (reply §2; North Star §8) | v0: `sql` read-only; now also `query` | done: `sql` (SPEC-001 §11) and `query`, Sophia's own language, on every backend (SPEC-007 §8) |
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

- 2026-10-22: SR-140 added (WASM as a compilation target, for later).
- 2026-10-20: SR-133 to SR-139 added (the second real-code review).
- 2026-10-20: SR-119 to SR-132 added (storage and materialisation, language folders, semantic-boundary experiments; the real-code review).
- 2026-10-20: SR-114 to SR-118 added (the three projects together; real-code corpora).
- 2026-10-20: SR-107 to SR-113 added from the review of M3a.
- 2026-10-20: SR-90 to SR-106 added from Daniel's query-language proposal and its forwarded review; SR-32 updated (the `query` dialect).
- 2026-10-20: SR-75 to SR-89 added from the external review.
- 2026-10-20: SR-25 to SR-29 and SR-51 to SR-74 added from Daniel's message of 2026-10-20 (the SR-25 to SR-29 block was still free).
- 2026-10-17: SR-50 added from the Moonkale agent's review.
- 2026-10-17: page created. Daniel's statements collected from the sources above; Moonkale's needs from its coordinator's list.
