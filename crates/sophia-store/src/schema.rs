//! The node/edge schema and its migrations.
//!
//! Design notes: `schema.md`, `markdown/Design/Graph Schema.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! This file is the ONLY place that should know the concrete labels. Every
//! other module talks to the schema through these types, so that the schema
//! really is replaceable — which markdown/Start Here.md explicitly asks for.
//!
//! ## Node kinds
//!
//! ```ignore
//! pub enum NodeKind {
//!     // core layer (sophia-core)
//!     Term, Type, Decl, Sig, Effect, Universe,
//!     // surface layer (one sub-schema per frontend)
//!     SurfaceNode, Span, Frontend,
//!     // IR layers
//!     Op, Region, Block, Value,         // MLIR
//!     Instr, BasicBlock, Func, Module,  // LLVM
//!     Target,                           // triple + datalayout + features
//!     // knowledge layer — NOT compiled
//!     Witness, Prop, Test, Bench, Doc, Example,
//!     // provenance and naming
//!     Name, Author, Signature, Patch,
//!     // Julia-specific
//!     Method, DispatchFact,
//! }
//! ```
//!
//! The knowledge layer is the part markdown/Start Here.md calls "annotations":
//! statements in the database that are not there for compilation.
//!
//! ## Edge kinds
//!
//! ```ignore
//! pub enum EdgeKind {
//!     Child(u32), HasType, DependsOn,
//!     ElaboratesTo, LowersTo, Emits,
//!     Specializes, InstanceOf,
//!     Equiv, Refines, WitnessedBy, Proves,
//!     Tests, Benchmarks, Documents, Annotates,
//!     Named, MigratesTo,
//! }
//! ```
//!
//! Edges are content-addressed too:
//!   h(edge) = H(ver ‖ "EDGE" ‖ kind ‖ h(src) ‖ h(dst) ‖ ord ‖ attrs)
//! because a witness must be able to cite the specific EQUIV edge it justifies
//! and you cannot cite what has no identity.
//!
//! ## The EQUIV edge carries the two attributes that do all the work
//!
//! ```ignore
//! pub struct EquivAttrs { level: EquivLevel, modulo: ModuloSet }
//!
//! pub enum EquivLevel {              // totally ordered, strongest first
//!     Alpha,          // same hash — trivial
//!     DefEq,          // kernel-decided definitional equality
//!     Rewrite,        // a chain of trusted rewrite rules
//!     Observational,  // a proof term in the Prf fragment
//!     // --- everything below is EVIDENCE, NOT PROOF ---
//!     Tested,         // agrees on a test suite
//!     Asserted,       // a human signed for it
//! }
//! ```
//!
//! `Tested` and `Asserted` are NOT substitutable by default. They are not
//! congruences — two sort functions agreeing on every test may differ on
//! stability, and a context observing stability distinguishes them. Silently
//! substituting on those levels is the most likely way this system produces a
//! wrong program. See markdown/Design/Equivalence and Witnesses.md.
//!
//! ## Relational materialisation (first backend)
//!
//! ```sql
//! CREATE TABLE node (h BLOB PRIMARY KEY, kind SMALLINT, ver SMALLINT, payload BLOB);
//! CREATE TABLE edge (h BLOB PRIMARY KEY, src BLOB, dst BLOB, kind SMALLINT,
//!                    ord INTEGER, attrs BLOB);
//! CREATE TABLE name (ns TEXT, sym TEXT, target BLOB,
//!                    valid_from BIGINT, valid_to BIGINT);
//! CREATE INDEX edge_src ON edge(src, kind, ord);
//! CREATE INDEX edge_dst ON edge(dst, kind);
//! ```
//!
//! Two tables. The schema's simplicity is the argument for starting on DuckDB
//! rather than a graph server.
//!
//! ## Migrations
//!
//! A hash schema version change re-hashes the world. Explicitly supported:
//!   - `ver` is the first field hashed, so versions occupy disjoint hash spaces
//!   - versions coexist in one store
//!   - MigratesTo edges relate old and new identities
//!   - migration is RE-ELABORATION from core terms, not a text rewrite —
//!     which is why it is cheap, and why core terms are the source of truth
//!
//! ## Scale note
//!
//! A 100k-line Julia package is ~10^6..10^7 Term nodes; MLIR multiplies by
//! 3..10 and LLVM by another 2..5. So 10^7..10^8 nodes per package if every IR
//! layer is persisted for one target. That number is the main argument for
//! making IR persistence optional and cache-like.
