//! `sophia-equiv` — equivalence claims, their strength, and their checking.
//!
//! Design notes: `sophia_equiv.md`, `markdown/Design/Equivalence and Witnesses.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! This crate is where markdown/Start Here.md's central claim lives: that code
//! from different languages can run together "with no use of FFI, just by
//! means of parsing it into a graph database and inserting equivalency
//! proofs". It is also the crate most likely to produce a wrong program if it
//! is built carelessly, so most of what follows is about refusing things.
//!
//! ## First, the part that is free
//!
//! If two definitions elaborate to the same core term they get the same hash
//! and ARE THE SAME NODE. No edge, no witness, no checking. A surprising
//! amount of cross-language sameness is of this kind — arithmetic on
//! fixed-width integers, straight-line data manipulation, pure functions over
//! primitives. This crate is only needed where the core terms genuinely
//! differ.
//!
//! ## Intended API
//!
//! ```ignore
//! pub struct EquivClaim {
//!     lhs: Hash, rhs: Hash,
//!     level: EquivLevel, modulo: ModuloSet,
//!     witness: Option<Hash>,
//! }
//!
//! pub fn check(claim: &EquivClaim, store: &dyn Store) -> Result<Verdict, CheckError>;
//!
//! /// Find a substitute for `h` that a caller's policy actually permits.
//! pub fn find_substitute(h: Hash, policy: &EquivPolicy, store: &dyn Store)
//!     -> Result<Option<(Hash, EquivPath)>>;
//!
//! pub struct EquivPolicy {
//!     min_level: EquivLevel,       // default: Rewrite. NOT Tested.
//!     allowed_modulo: ModuloSet,   // default: empty
//!     max_chain: u8,               // default: 3
//! }
//! ```
//!
//! Defaults matter more than features here. The default policy must exclude
//! Tested and Asserted, because those are not congruences and substituting on
//! them is unsound. Opting in must be explicit and must taint the artifact's
//! provenance record.
//!
//! ## Composition degrades, and that is correct
//!
//! ```ignore
//! // chaining two claims:
//! level  = min(a.level, b.level)
//! modulo = a.modulo ∪ b.modulo
//! ```
//!
//! Union, not intersection. Chain a few claims and the modulo set grows until
//! the result says nothing. Consequences for the implementation:
//!
//!   - NEVER materialise the transitive EQUIV closure. It would be enormous
//!     and useless. Search lazily, per query, against the caller's budget.
//!   - Keep SHORTEST witness chains, not just reachability.
//!   - Bound chain length (policy default 3).
//!
//! ## The congruence rule
//!
//! Substitution is licensed only if the relation is a congruence:
//!   t ≈ u  =>  C[t] ≈ C[u]   for every context C
//!
//! DefEq and sound Rewrite chains are congruences by construction. Tested and
//! Asserted are not. This is not a detail to revisit later — it is the
//! difference between an optimisation and a miscompilation.
//! See markdown/Wiki/Semantics/Contextual Equivalence.md.
//!
//! ## Cross-language claims need a relation R, written down
//!
//! "t1 does the same as t2" is not well-formed until someone supplies
//! R ⊆ D1 × D2 relating the two observation universes. Sophia makes this
//! tractable by elaborating both sides into SC so there is only one semantics
//! — but the residual R (Julia Int64 ↔ C++ int64_t only where no overflow,
//! Julia String ↔ std::string only under stated invariants, Array ↔ vector
//! only with an ownership story) is a reviewed artifact and part of the TCB.
//! See markdown/Wiki/Semantics/Logical Relations.md.
//!
//! ## Refinement is the more useful primitive
//!
//! ```ignore
//! pub fn refines(a: Hash, b: Hash) -> ...;   //  a ⊑ b : a may replace b
//! ```
//!
//! Most real cases are asymmetric: fewer behaviours, total where the other was
//! partial, checked where the other was unchecked. Alive2 checks REFINEMENT
//! rather than equality for exactly this reason, and Julia -> Rust is
//! refinement rather than equivalence (see sophia-core/effects.rs).
//! Conflating the two directions is a classic unsoundness.
//!
//! ## Intended module layout
//!
//! ```text
//! sophia_equiv.rs  this file: claims, levels, policy, substitution search
//! egraph.rs        saturation and extraction (via `egg`)
//! witness.rs       the witness formats and their checkers
//! ```

// pub mod egraph;
// pub mod witness;
