//! `sophia-hash` — crate root.
//!
//! Turns a Sophia Core term into the 256-bit identity that everything else in
//! the system is keyed by. Design notes: `sophia_hash.md`, and
//! `markdown/Design/Hashing and Identity.md`.
//!
//! NOTHING IN THIS CRATE IS IMPLEMENTED YET. Every file is comments only.
//! The point of the file is to fix a module boundary and record intent.
//!
//! ## Why this crate is first in the dependency order
//!
//! Identity is the one decision that cannot be revised cheaply later: change
//! the hash and every node in every store changes with it. So this crate is
//! small, has no dependencies on the rest of the workspace, and is meant to be
//! auditable in an afternoon. It is in the trusted computing base
//! (`markdown/Design/Trusted Computing Base.md`); sophia-store is not.
//!
//! ## Intended module layout
//!
//! ```text
//! sophia_hash.rs   this file: public API, Hash type, schema version
//! canonical.rs     term -> canonical form (the part that must be got right)
//! merkle.rs        canonical form -> hash, including the SCC construction
//! ```
//!
//! ## Intended public surface
//!
//! ```ignore
//! pub struct Hash([u8; 32]);                 // BLAKE3-256, never truncated in storage
//! pub struct SchemaVersion(u16);              // first field hashed; see below
//!
//! pub fn hash_term(t: &CanonTerm) -> Hash;    // pure, total, deterministic
//! pub fn hash_type(t: &CanonTerm) -> Hash;    // h_type: interface identity
//! pub fn hash_erased(t: &CanonTerm) -> Hash;  // h_run: codegen cache key
//! pub fn hash_scc(members: &[CanonTerm]) -> Vec<Hash>;   // mutual recursion
//! pub fn hash_edge(kind: EdgeKind, src: Hash, dst: Hash, ord: u32, attrs: &[u8]) -> Hash;
//! ```
//!
//! Three hashes, not one, because they answer three different questions:
//! `h_def` is "is this the same definition", `h_type` is "does this fit the
//! same hole", `h_run` is "can I reuse the compiled code". Collapsing them
//! costs cache hits (too fine) or correctness (too coarse).
//!
//! ## Domain separation
//!
//! Every node kind hashes under its own derived key:
//!
//! ```ignore
//! let key = blake3::derive_key(&format!("sophia.v{ver}.node.{tag}"), b"");
//! ```
//!
//! so a `Term` digest and an `Op` digest cannot collide even in principle,
//! rather than merely with high probability.
//!
//! ## Encoding rules that must not be got wrong
//!
//! - Concatenation is ALWAYS length-prefixed. `("ab","c")` must not collide
//!   with `("a","bc")`. This is the single most common bug in Merkle schemes.
//! - Integers are fixed-width little-endian. No varints, no platform width.
//! - Floating-point attributes are hashed as their IEEE-754 bit patterns, with
//!   NaN payloads preserved; `-0.0` and `+0.0` are DIFFERENT.
//! - The schema version is the first field of every digest, so different
//!   versions inhabit disjoint hash spaces by construction.
//!
//! ## Non-goals
//!
//! - No non-cryptographic hash is ever persisted. `xxhash` is for the in-memory
//!   intern table only (`markdown/Wiki/Hashing/Hash Consing.md`).
//! - No truncation. A 128-bit UUID may be derived for display; it is never the
//!   key (`markdown/Wiki/Hashing/UUID.md`).
//! - No salting, no per-machine state, no time. The function must be global.
//!
//! ## Test obligations (before anything else in the project is worth building)
//!
//! 1. Determinism: hashing the same term twice, in two processes, on two
//!    machines, yields the same digest.
//! 2. Alpha-invariance: renaming bound variables does not change the digest.
//! 3. Sensitivity: changing any attribute that affects semantics DOES change it
//!    — in particular overflow discipline and float flags.
//! 4. Round-trip: canonicalise(canonicalise(t)) == canonicalise(t).
//! 5. Re-ingesting an unchanged corpus writes zero new nodes (the Roadmap M0
//!    acceptance test).

// pub mod canonical;
// pub mod merkle;
