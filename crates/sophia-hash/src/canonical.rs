//! Canonicalisation: Sophia Core term -> canonical form.
//!
//! Design notes: `canonical.md`, `markdown/Design/Hashing and Identity.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! This is the most dangerous file in the repository. Everything else fails
//! loudly; this fails silently. An over-aggressive canonicaliser identifies two
//! programs that are not the same, and the result is a wrong binary with no
//! error anywhere. Read `markdown/Design/Trusted Computing Base.md` before
//! touching it.
//!
//! ## The pipeline
//!
//! ```ignore
//! pub fn canonicalise(t: &Term, env: &Env) -> Result<CanonTerm, CanonError>;
//! ```
//!
//! Applied in this order, and the order is part of the specification:
//!
//! 1. LOCALLY NAMELESS CONVERSION
//!    Bound variables -> de Bruijn indices; free variables -> the hash of the
//!    definition they refer to. This is what makes hashing alpha-invariant.
//!    See markdown/Wiki/Hashing/De Bruijn Index.md.
//!
//! 2. LET-FLOATING AND DEAD-BINDING ELIMINATION
//!    `let x = e in b` with x unused in b  ==>  b, IF e is effect-free.
//!    The effect check is mandatory: dropping an effectful binding changes the
//!    program. sophia-core supplies the effect row.
//!
//! 3. ETA-EXPANSION TO A FIXED ARITY CONVENTION
//!    So that `f` and `\x. f x` hash identically. Requires the type, which is
//!    why canonicalisation runs on ELABORATED terms, never on surface syntax.
//!
//! 4. NORMALISATION OF TYPE-LEVEL SUBTERMS  (via sophia-core's NbE)
//!    Definitional equality must be reflected in identity, or the kernel's
//!    conversion rule and the hash disagree about what "the same" means — and
//!    then the same term can enter the store under two identities.
//!    See markdown/Wiki/Type Theory/Definitional vs Propositional Equality.md.
//!
//! 5. DETERMINISTIC ORDERING OF UNORDERED STRUCTURES
//!    Record fields, module members, effect rows, independent let-groups.
//!    Order by the hash of the contents — which is a fixpoint, because the key
//!    depends on hashes that depend on the order. Iterate to stability; cap the
//!    iteration count and fail loudly rather than looping.
//!
//! 6. ATTRIBUTE NORMALISATION FOR PRIMITIVES
//!    Overflow discipline, rounding mode, contraction/reassociation permission,
//!    alignment. These are made EXPLICIT, never defaulted away. A Julia `+`
//!    (wrapping) and a C++ `+` (nsw) must not converge here.
//!    See markdown/Design/Cross-Language Semantic Hazards.md.
//!
//! ## What canonicalisation must NOT do
//!
//! - No commutativity. No associativity. No distributivity. No algebraic
//!   identities of any kind. Those are EQUIV edges discovered by the e-graph
//!   (sophia-equiv), not identity. Baking them in here requires a decision
//!   procedure for program equality, which does not exist.
//! - No constant folding. Same reason: it is a rewrite, and rewrites are
//!   recorded as equivalences so they stay inspectable and reversible.
//! - No dropping of attributes "that probably don't matter".
//!
//! The governing rule: WHEN IN DOUBT, KEEP IT. An over-fine canonical form
//! costs cache hits. An over-coarse one costs correctness.
//!
//! ## Properties that must hold (and be property-tested)
//!
//! ```ignore
//! canonicalise(canonicalise(t)) == canonicalise(t)          // idempotent
//! t =alpha= u                    ==> canon(t) == canon(u)    // alpha-invariant
//! canon(t) == canon(u)           ==> t and u are semantically equal  // SOUNDNESS
//! ```
//!
//! The third is the one that cannot be tested directly, only argued. Every
//! step above needs a one-paragraph justification of why it preserves meaning,
//! written down next to it.
//!
//! ## Failure modes worth an explicit error rather than a silent result
//!
//! - Ordering fixpoint does not converge within N iterations
//! - SCC larger than a configured cap (see merkle.rs)
//! - Normalisation budget exceeded (a term whose type-level part diverges)
//! - An attribute the current schema version does not know how to encode
//!
//! Each of these is a case where the honest answer is "this term has no
//! identity under schema version V", not a best-effort digest.
