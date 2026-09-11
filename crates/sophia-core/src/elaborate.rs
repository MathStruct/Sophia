//! Elaboration and normalisation: surface syntax -> Sophia Core.
//!
//! Design notes: `elaborate.md`, `markdown/Design/Core Calculus.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! Elaboration is where every frontend's real work lands, and where all the
//! per-language difficulty is concentrated on purpose: there is ONE semantics
//! (SC's) rather than n, and each frontend's obligation is to explain its
//! language in terms of it. That obligation is unverified and is the de facto
//! specification of the ingested subset.
//! See markdown/State of the Art/State of the Art - Formal Semantics of Real Languages.md.
//!
//! ## Intended API
//!
//! ```ignore
//! pub trait Frontend {
//!     fn lang(&self) -> LangId;
//!     fn version(&self) -> FrontendVersion;   // recorded on ELABORATES_TO
//!     fn elaborate(&self, node: &SurfaceNode, ctx: &mut ElabCtx)
//!         -> Result<(Term, Type, EffectRow), ElabError>;
//! }
//! ```
//!
//! The version is not cosmetic: two frontend versions produce different
//! elaborations of the same source, and the store must be able to tell them
//! apart or it accumulates unattributable near-duplicates.
//!
//! ## Normalisation by evaluation
//!
//! ```ignore
//! pub enum Value { Lam(Closure), Neutral(Neutral), Univ(Level), .. }
//! pub fn eval(env: &Env, t: &Term) -> Value;     // host closures do beta
//! pub fn quote(lvl: Level, v: &Value) -> Term;   // reify, fresh vars under binders
//! pub fn nf(t: &Term) -> Term { quote(0, &eval(&Env::empty(), t)) }
//! ```
//!
//! Two consumers, and they must agree:
//!   - the kernel's `convert` (definitional equality)
//!   - sophia-hash's canonicaliser (step 4)
//! If they ever diverge, one term gets two identities. See sophia_core.md.
//!
//! Quote naturally produces de Bruijn LEVELS; convert to INDICES at the end.
//! Mixing the two is the classic NbE bug.
//! See markdown/Wiki/Type Theory/Normalization by Evaluation.md.
//!
//! ## What elaboration must make explicit
//!
//! Everything the surface language left implicit, because the hash is computed
//! on the result and implicit things are exactly the things that differ
//! silently between languages:
//!
//!   - implicit/instance arguments, type class and trait resolution
//!   - coercions and numeric promotions (C++'s usual arithmetic conversions,
//!     Julia's promote/convert) — NEVER silent, always an explicit Convert node
//!   - overflow discipline and float flags on every arithmetic primitive
//!   - bounds checks, or an explicit assumption node discharging them
//!   - effect rows (effects.rs)
//!   - index-base and layout arithmetic: Julia's 1-based column-major and C's
//!     0-based row-major both become explicit linear-index computation, or no
//!     comparison between them is meaningful
//!
//! ## Functoriality — the property test to write first
//!
//! A frontend is meant to be a functor (markdown/Design/Multi-AST Layering.md):
//!
//! ```ignore
//! elaborate(compose(f, g)) ≡ compose(elaborate(f), elaborate(g))
//! elaborate(identity)      ≡ identity
//! ```
//!
//! Cheap to test with generated inputs, and it catches the most damaging class
//! of frontend bug — a peephole in the parser that special-cases a composite
//! form and therefore produces hashes that do not compose.
//!
//! ## Bounded subsets, enforced
//!
//! No frontend will cover its whole language. The subset boundary must be
//! EXPLICIT and ingestion outside it must FAIL, not guess. A term whose
//! meaning was guessed is worse than a term that was never ingested, because
//! it will be hashed, stored, trusted and compiled.
//!
//! ## Error reporting
//!
//! Elaboration errors are the ones a user actually sees, so they need the
//! surface span (kept on SurfaceNode, deliberately NOT part of the core hash —
//! see markdown/Design/Hashing and Identity.md) and the elaboration context.
