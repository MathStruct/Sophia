//! Witness formats and their checkers.
//!
//! Design notes: `witness.md`, `markdown/Design/Equivalence and Witnesses.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! A witness turns a CLAIM into a LICENCE. The checkers are the part of the
//! system that has to be right; keeping each one small and separate is the
//! whole design, because a single monolithic "verifier" would be both
//! unauditable and a single point of failure.
//! See markdown/Design/Trusted Computing Base.md.
//!
//! ## The formats
//!
//! ```ignore
//! pub enum Witness {
//!     /// A Prf-fragment proof term of `lhs =_A rhs`, checked by the SC kernel.
//!     /// Strongest. Needs a human, which is why there will be few of these.
//!     Kernel { proof: Hash },
//!
//!     /// An ordered list of (rule, position, direction). The checker replays
//!     /// it. This is the WORKHORSE: e-graph extraction produces these for
//!     /// free, soundness reduces to soundness of the rules, and no human is
//!     /// involved.
//!     RewriteChain { steps: Vec<(Hash /*rule*/, Path, Dir)> },
//!
//!     /// An SMT proof object plus the encoding used. Trust shifts to the
//!     /// ENCODER and the proof checker, not to the solver — which is the
//!     /// point: solvers are large and buggy, proof checkers are small.
//!     Smt { logic: SmtLogic, encoding: Hash, certificate: Blob },
//!
//!     /// Per-instance translation validation of one lowering, Alive2-style.
//!     /// The realistic mechanism for Term -> Op -> Instr edges, since nobody
//!     /// is going to verify MLIR and LLVM in general.
//!     TranslationValidation { tool: Hash, target: Hash, report: Blob },
//!
//!     /// EVIDENCE, NOT PROOF. Records suite, seed, inputs, environment.
//!     TestEvidence { suite: Hash, seed: u64, env: Hash, results: Blob },
//!
//!     /// The weakest. Must be attributable so it can be revoked.
//!     Attestation { author: Hash, signature: Blob, statement: Hash },
//! }
//! ```
//!
//! ## Each checker is independent
//!
//! ```ignore
//! pub trait WitnessChecker {
//!     fn kind(&self) -> WitnessKind;
//!     fn max_level(&self) -> EquivLevel;   // the strongest claim it can justify
//!     fn check(&self, w: &Witness, claim: &EquivClaim, store: &dyn Store)
//!         -> Result<Verdict, CheckError>;
//! }
//! ```
//!
//! `max_level` is the honest bit: TestEvidence can never justify more than
//! `Tested`, however many tests passed. The type system should make it
//! impossible for a checker to grant a level it cannot support.
//!
//! ## Where the witnesses will actually come from
//!
//! The worry in markdown/Design/Open Problems and Risks.md is that nobody
//! writes proofs. Three answers that need no human:
//!
//!   1. RewriteChain, free, from the e-graph, for every rewrite it applies.
//!   2. TranslationValidation, harvested from Alive2-style checking of each
//!      lowering instance.
//!   3. Smt, from a Souper-style harvester over stored core terms, for
//!      decidable fragments (bitvectors, linear arithmetic).
//!
//! Realistic outcome: most claims in a deployed store are evidence, a few are
//! machine-generated proofs, very few are human proofs. That is still better
//! than today, where the claims are not recorded at all.
//!
//! ## Revocation
//!
//! An Attestation can turn out to be false. Since nothing is ever deleted, a
//! revocation is a new node asserting that a witness is withdrawn, plus a
//! query-time filter. Anything built while trusting it must be findable —
//! which is what the trust audit query is for (markdown/Design/Query Cookbook.md).
//!
//! ## Adversarial notes
//!
//! Checkers parse untrusted input. Fuzz them. Bound their resource use.
//! An unbounded SMT certificate check is a denial-of-service vector, and a
//! panicking checker that gets treated as "unverified" rather than "rejected"
//! is a correctness one.
