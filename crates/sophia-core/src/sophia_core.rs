//! `sophia-core` — the Sophia Core calculus (SC), its kernel, and elaboration.
//!
//! Design notes: `sophia_core.md`, `markdown/Design/Core Calculus.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! Every frontend elaborates into this language. Without one common language
//! there is nothing to hash consistently, nothing to lower uniformly, and no
//! vocabulary in which to state that a Julia function and a C++ function are
//! equivalent.
//!
//! ## Intended term representation
//!
//! ```ignore
//! pub enum Term {
//!     Var(DbIndex),                    // locally nameless: bound
//!     Ref(Hash),                       // locally nameless: free, by identity
//!     Univ(Level),
//!     Pi   { dom: Rc<Term>, cod: Rc<Term>, quantity: Quantity },
//!     Lam  { body: Rc<Term> },
//!     App  { fun: Rc<Term>, arg: Rc<Term> },
//!     Sigma{ fst: Rc<Term>, snd: Rc<Term> },
//!     Pair { fst: Rc<Term>, snd: Rc<Term> },
//!     Proj { idx: u8, on: Rc<Term> },
//!     Data { decl: Hash, args: SmallVec<[Rc<Term>; 4]> },
//!     Con  { decl: Hash, idx: u32, args: SmallVec<[Rc<Term>; 4]> },
//!     Elim { decl: Hash, motive: Rc<Term>, branches: Vec<Rc<Term>> },
//!     Let  { val: Rc<Term>, body: Rc<Term> },
//!     Fix  { body: Rc<Term> },         // general recursion — Cmp fragment ONLY
//!     Prim(PrimOp),                    // machine primitives, see below
//! }
//! ```
//!
//! `Rc` plus an intern table gives hash-consing: structural equality becomes
//! pointer equality, which matters because the conversion checker compares
//! terms constantly. See markdown/Wiki/Hashing/Hash Consing.md.
//!
//! ## Primitives are part of the calculus, not a library
//!
//! ```ignore
//! pub enum PrimOp {
//!     IntArith { op: IntOp, width: u8, signed: bool, overflow: Overflow },
//!     FloatArith { op: FloatOp, fmt: FpFormat, rounding: Rounding,
//!                  contract: bool, reassoc: bool },
//!     Cmp { .. }, Convert { .. }, Load { region: RegionVar, .. }, Store { .. },
//! }
//! pub enum Overflow { Wrap, Trap, Poison }   // Julia | Rust-debug | C++-nsw
//! ```
//!
//! This is the whole reason SC cannot have an abstract `Int`. Julia's `+`
//! wraps; C++'s signed `+` is UB and LLVM optimises on that assumption. They
//! are DIFFERENT FUNCTIONS and must get different hashes, or the system will
//! cheerfully prove a false equivalence.
//! See markdown/Design/Cross-Language Semantic Hazards.md.
//!
//! ## Two fragments, one door
//!
//! ```ignore
//! pub enum Fragment { Prf, Cmp }
//! ```
//!
//! `Cmp` has `Fix` and effects; it is where all programs live and it is
//! logically inconsistent (`Fix` inhabits every type). `Prf` is total and
//! effect-free; it is where witnesses live and it is consistent. There is an
//! inclusion Prf -> Cmp and NO map back. Statements about Cmp terms are made
//! in Prf by quoting them (deeply embedded, by hash).
//!
//! Getting this wrong makes every proof in the store meaningless, so the
//! fragment is checked by the kernel, not by convention.
//!
//! ## Judgement
//!
//! ```ignore
//! //  Γ ⊢ t : A ! ε
//! pub fn infer(ctx: &Ctx, t: &Term) -> Result<(Type, EffectRow), TypeError>;
//! pub fn check(ctx: &Ctx, t: &Term, want: &Type) -> Result<EffectRow, TypeError>;
//! pub fn convert(ctx: &Ctx, a: &Term, b: &Term) -> bool;   // definitional eq
//! ```
//!
//! `convert` is decided by normalisation-by-evaluation (see elaborate.rs) and
//! is the SAME normalisation the canonicaliser uses, because definitional
//! equality must be reflected in identity. If the two ever diverge, a term can
//! enter the store under two identities and the invariant is gone.
//!
//! ## Intended module layout
//!
//! ```text
//! sophia_core.rs   this file: Term, Type, kernel API, fragments
//! elaborate.rs     surface -> core, NbE, unification, implicit insertion
//! effects.rs       effect rows, regions, borrows
//! ```
//!
//! ## Deliberate non-goals
//!
//! - SC is not a surface language. Nobody writes it by hand.
//! - SC is not total (see `Fix`) and is not meant to be.
//! - SC is not an optimisation IR; that is MLIR's job (sophia-emit).
//! - SC is not minimal for its own sake. Sigma, Let and inductive families
//!   could be encoded away, and encoding them away would destroy the
//!   structural correspondence with source syntax that keeps hashes stable
//!   and the graph legible.

// pub mod elaborate;
// pub mod effects;
