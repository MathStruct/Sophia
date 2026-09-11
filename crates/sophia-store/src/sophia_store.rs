//! `sophia-store` — persistence and querying of the code graph.
//!
//! Design notes: `sophia_store.md`, `markdown/Design/Graph Schema.md`,
//! `markdown/Design/Query Cookbook.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! ## The property that shapes this whole crate
//!
//! THE STORE IS NOT TRUSTED. Every node's key is a hash of its own content, so
//! any consumer can re-verify anything by rehashing it. A corrupt or hostile
//! store can withhold data or return the wrong data, but it cannot substitute
//! different content under an existing hash. That removes the largest
//! component from the trusted computing base for free — and it means this
//! crate can be swapped, rewritten or migrated without a security argument.
//! See markdown/Design/Trusted Computing Base.md.
//!
//! It is also what markdown/Start Here.md means by "if anyone disagrees with
//! our implementation, all he needs to do is rewrite the core and migrate the
//! database to his own preferred schema".
//!
//! ## Backend trait
//!
//! ```ignore
//! pub trait Store {
//!     fn get(&self, h: Hash) -> Result<Option<NodeRecord>>;
//!     fn put(&mut self, n: &NodeRecord) -> Result<Hash>;       // idempotent
//!     fn put_edge(&mut self, e: &EdgeRecord) -> Result<Hash>;  // idempotent
//!
//!     fn children(&self, h: Hash) -> Result<Vec<(u32, Hash)>>;
//!     fn parents(&self, h: Hash, kind: EdgeKind) -> Result<Vec<Hash>>;
//!     fn subtree(&self, root: Hash, budget: Budget) -> Result<TermChunk>;
//!
//!     fn dependents(&self, h: Hash, depth: Option<u32>) -> Result<Vec<Hash>>;
//!     fn equiv_paths(&self, h: Hash, policy: &EquivPolicy) -> Result<Vec<EquivPath>>;
//!
//!     fn resolve(&self, ns: &str, sym: &str, at: WorldTime) -> Result<Option<Hash>>;
//!     fn bind(&mut self, ns: &str, sym: &str, h: Hash) -> Result<()>;  // ONLY mutation
//! }
//! ```
//!
//! `put` is idempotent because the key IS the content: writing a node that
//! already exists is a no-op. That is the M0 acceptance test — re-ingesting an
//! unchanged corpus must write zero new rows.
//!
//! `bind` is the only mutating operation in the system. Everything else is
//! append-only. See markdown/Design/Naming and Change Propagation.md.
//!
//! ## Planned backends
//!
//! ```text
//! DuckDbStore    first, and probably the right default. Two wide tables, a
//!                hash primary key, recursive CTEs for traversal, no server.
//! SqliteStore    Turso/libSQL: embeddable, syncable, same schema.
//! FalkorStore    property graph; Cypher makes variable-length traversal
//!                native, at the cost of needing a server.
//! MemStore       in-process, hash-consed; the hot tier and the test double.
//! ```
//!
//! The trait exists so the choice is reversible. It will leak — query dialects
//! always do — so the leak is confined to `query.rs`-shaped code per backend
//! rather than spread through the crate.
//!
//! ## The performance question this crate has to answer
//!
//! Reconstructing one function's IR means walking 10^4..10^6 nodes. If that is
//! an order of magnitude slower than re-running the frontend on source text,
//! the architecture must retreat to "core terms persisted, IR regenerated".
//! MEASURE THIS AT ROADMAP M2, not later. Mitigations, in the order to try:
//!
//!   1. Chunked subtree blobs keyed by root hash (what Git does with packfiles)
//!      with individual rows only for independently-referenced nodes.
//!   2. Materialised DEPENDS_ON* closure.
//!   3. MemStore hot tier, database as cold tier.
//!   4. Do not persist Op/Instr layers by default — treat them as an evictable
//!      cache and keep only core terms as the source of truth.
//!
//! (4) is probably the correct default and is a real retreat from the most
//! ambitious reading of the idea. It keeps everything that matters.
//!
//! ## Garbage collection
//!
//! Reachability from the Name table plus pinned hashes, with a retention
//! window. Deletion is safe precisely because identity is content-derived: if
//! a node is ever needed again it can be regenerated with the same hash.

// pub mod schema;
