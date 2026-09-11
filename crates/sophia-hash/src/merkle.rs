//! Merkle hashing over canonical terms, including the cyclic case.
//!
//! Design notes: `merkle.md`, `markdown/Wiki/Hashing/Cycle Hashing.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! ## The acyclic case
//!
//! ```ignore
//! h(n) = H_key(tag) [ ver ‖ tag ‖ attrs(n) ‖ h(c_1) ‖ … ‖ h(c_k) ]
//! ```
//!
//! with `‖` length-prefixed and `H_key` a BLAKE3 instance keyed by
//! `derive_key("sophia.v{ver}.node.{tag}")`. Bottom-up over the canonical
//! term; memoise on the term's interned pointer (hash-consing) so that a term
//! appearing n times is hashed once.
//!
//! ## The cyclic case — mutual recursion
//!
//! The recursion above is well-founded only on a DAG, and `isEven`/`isOdd` is
//! not a DAG. Construction, following Unison:
//!
//! ```text
//! 1. Tarjan SCC over the DEPENDS_ON graph of the definition group.
//! 2. Condensation is acyclic -> hash SCCs bottom-up in that order.
//! 3. Inside an SCC, order members canonically WITHOUT using their names:
//!
//!      colour_0(m) = H(tag(m) ‖ sorted(hashes of m's EXTERNAL deps))
//!      colour_{i+1}(m) = H(colour_i(m) ‖ sorted multiset of
//!                          { colour_i(n) : n an SCC-neighbour of m })
//!
//!    until the partition stops refining. (1-WL / Paige-Tarjan refinement.)
//! 4. If a colour class still has >1 member they are genuinely symmetric under
//!    refinement; break the tie by lexicographically least serialisation and
//!    RECORD the chosen permutation so the result is reproducible.
//! 5. h_scc = H[ ver ‖ "SCC" ‖ refined_colour_1 ‖ … ‖ refined_colour_m ]
//!    h_i   = H[ h_scc ‖ i ]        for canonical index i
//! ```
//!
//! Step 4 is the theoretically ugly one — canonical labelling of a graph is
//! not known to be polynomial. It does not bite in practice because real SCCs
//! are 2-5 definitions and refinement separates them on the first round. It IS
//! an adversarial input away from being expensive, so:
//!
//!   - cap SCC size (config, default ~64) and return an error above it
//!   - cap refinement rounds at |SCC| and error if not stable
//!
//! Failing loudly on a pathological input is correct. Timing out is not.
//!
//! ## Intended API
//!
//! ```ignore
//! pub fn hash_dag(t: &CanonTerm, cache: &mut HashCache) -> Hash;
//! pub fn hash_group(members: &[CanonTerm]) -> Result<Vec<Hash>, SccError>;
//! pub fn hash_edge(kind: EdgeKind, src: Hash, dst: Hash,
//!                  ord: u32, attrs: &[u8]) -> Hash;
//! ```
//!
//! Edges are hashed too, because a witness has to be able to cite the specific
//! EQUIV edge it justifies. See markdown/Design/Graph Schema.md.
//!
//! ## Erasure, for h_run
//!
//! `hash_erased` runs the same construction over a term with all
//! runtime-irrelevant structure removed:
//!
//!   - types in erasable positions (0-quantity binders, cf. QTT)
//!   - proof terms (the Prf fragment never executes)
//!   - phantom parameters
//!
//! Two definitions differing only in erased content share h_run and therefore
//! share compiled code. That is where the precompilation win comes from; see
//! markdown/Design/Content-Addressed Precompilation.md.
//!
//! The erasure function is itself semantics-critical: erasing something that
//! IS runtime-relevant silently unifies two different programs. Same warning
//! as canonical.rs.
//!
//! ## Streaming / verification
//!
//! BLAKE3's tree structure means a large payload node (a string literal, an
//! embedded blob) can be verified incrementally without holding it in memory.
//! Worth using when blob attributes appear; not needed for ordinary terms.
