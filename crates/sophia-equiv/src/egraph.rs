//! E-graph construction, saturation and extraction.
//!
//! Design notes: `egraph.md`, `markdown/Design/Compilation as Query.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! Intended to be a thin layer over `egg` rather than a reimplementation. The
//! e-graph literature is mature (see markdown/State of the Art/State of the
//! Art - Equality Saturation and E-Graphs.md) and Cranelift now ships e-graph
//! based optimisation in production, so this is one of the lower-risk parts of
//! the project.
//!
//! ## Why an e-graph is the natural in-memory form of the EQUIV relation
//!
//!   e-class            <-> equivalence class of hashes
//!   congruence closure <-> the requirement that EQUIV be a congruence
//!   proof extraction   <-> the rewrite-chain witness format
//!
//! The correspondence is close enough that the e-graph is not "a technique we
//! also use" but the operational meaning of the equivalence layer.
//!
//! ## Intended API
//!
//! ```ignore
//! pub struct Seed { roots: Vec<Hash>, depth: u32 }
//!
//! pub fn load(store: &dyn Store, seed: &Seed) -> EGraph;
//! pub fn saturate(g: &mut EGraph, rules: &RuleSet, budget: Budget) -> SatReport;
//! pub fn extract(g: &EGraph, cost: &dyn CostModel) -> Extracted;
//! pub fn writeback(g: &EGraph, store: &mut dyn Store) -> Vec<EquivClaim>;
//! ```
//!
//! `writeback` is what makes this different from an ordinary compiler pass:
//! the equivalences discovered during one build are PERSISTED and available to
//! every later build, on any machine. That accumulation is the main argument
//! for the whole architecture.
//!
//! ## Rules are nodes
//!
//! ```ignore
//! pub struct Rule { lhs: Pattern, rhs: Pattern, guard: Option<Guard>,
//!                   soundness: Hash /* a Witness, or an axiom marker */ }
//! ```
//!
//! A rule lives in the store like everything else, so it is versioned,
//! hashable and citable. A rewrite-chain witness cites rule hashes, which
//! means the soundness of a chain reduces to the soundness of its rules — and
//! those are either themselves witnessed or explicitly marked as axioms that
//! someone signed for.
//!
//! ## Rules must be effect-guarded
//!
//! Almost every interesting rewrite is sound only for effect-free subterms.
//! `let x = e in b` => `b` when x is unused REQUIRES e to be pure. The guard
//! consults the effect row from sophia-core. A rule set without effect guards
//! is a miscompilation generator.
//!
//! ## Extraction
//!
//! Minimise sum over selected nodes of cost(n), subject to: at least one node
//! chosen per e-class, children of chosen nodes chosen, and NO CYCLES in the
//! extracted DAG. The acyclicity constraint is what makes this NP-hard.
//!
//!   - default: greedy bottom-up with a cycle check
//!   - escalate to ILP only for small hot regions
//!   - cost model comes from the Target node; it is target-specific and it
//!     matters more than the search does (see STOKE's results)
//!
//! ## Budgets are mandatory
//!
//! Associativity and commutativity rules blow an e-graph up. Nothing here
//! saturates to a true fixpoint in practice. Hard caps on e-node count,
//! iterations and wall time, with the report recording WHICH budget was hit —
//! a build that silently got a worse result because it ran out of nodes should
//! say so.
//!
//! ## What this will and will not discover
//!
//! Will: peepholes, algebraic simplification, strength reduction, constant
//! folding, loop-invariant motion expressed as rewrites.
//! Will not: that a Julia sort and a C++ sort are equivalent. Superoptimisers
//! run out of room at a handful of operations, and this is the same search.
//! See markdown/State of the Art/State of the Art - Superoptimization and Synthesis.md.
