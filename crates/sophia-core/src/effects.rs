//! Effect rows, regions and borrows for Sophia Core.
//!
//! Design notes: `effects.md`, `markdown/Design/Effects Memory and Resources.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! Effects are not a refinement to be added later. They are what an
//! equivalence claim is ABOUT: two functions cannot be equivalent if they
//! differ observably, and effects are the observations.
//! See markdown/Wiki/Semantics/Contextual Equivalence.md.
//!
//! ## Effect rows
//!
//! ```ignore
//! pub struct EffectRow {
//!     labels: BTreeSet<Effect>,   // ordered => canonical => hashable
//!     tail: Option<RowVar>,       // row polymorphism: `map` is as pure as its f
//! }
//!
//! pub enum Effect {
//!     Read(RegionVar), Write(RegionVar), Alloc(RegionVar), Free(RegionVar),
//!     Throw(Hash),        // the exception type, by identity
//!     Diverge,            // may not terminate — an OBSERVATION, not a detail
//!     Io,
//!     Nondet,             // scheduling, address layout, hash iteration order
//!     Unsafe,             // escapes the model: inline asm, ccall, C++ UB
//! }
//! ```
//!
//! BTreeSet, not HashSet: the row is hashed, so its serialisation must be
//! canonical. Same reasoning as canonical.rs step 5.
//!
//! `Unsafe` is the bottom element and poisons whatever contains it. That is
//! intended — it makes "we do not model this" visible in the type rather than
//! silently absent.
//!
//! ## Regions and borrows — one device for four memory models
//!
//! ```ignore
//! pub enum RegionKind { GcHeap, RefCounted, Arena(ArenaId), Stack, Foreign }
//! pub enum Ptr { Owned(RegionVar), Ref(RegionVar), MutRef(RegionVar) }
//! ```
//!
//! | language | maps to |
//! |----------|---------|
//! | Julia    | one GcHeap region; no Free; aliasing UNKNOWN (view/reshape/unsafe_wrap) |
//! | C++      | explicit regions + Free; aliasing unrestricted unless restrict; starts Unsafe |
//! | Rust     | regions = lifetimes; MutRef implies noalias; nearly mechanical |
//! | Lean     | RefCounted; uniqueness is a perf property, not a semantic one |
//!
//! THE CONSEQUENCE THAT MATTERS: Rust's model forbids programs Julia allows,
//! so Julia -> Rust is REFINEMENT, not equivalence. Emitting an EQUIV edge
//! there instead of REFINES is a concrete route to an aliasing miscompilation.
//! See markdown/Design/Equivalence and Witnesses.md.
//!
//! ## Effect operations the kernel needs
//!
//! ```ignore
//! pub fn union(a: &EffectRow, b: &EffectRow) -> EffectRow;
//! pub fn subsumes(have: &EffectRow, want: &EffectRow) -> bool;   // subtyping
//! pub fn is_pure(e: &EffectRow) -> bool;      // licenses CSE, reordering, memoisation
//! pub fn observable(e: &EffectRow, modulo: &ModuloSet) -> EffectRow;
//! ```
//!
//! `observable` is the bridge to sophia-equiv: an equivalence claim "modulo
//! alloc" is exactly a claim about rows with Alloc projected out. Keeping the
//! projection here rather than in the equivalence checker means there is one
//! definition of what a modulo tag means.
//!
//! ## Start coarse
//!
//! A full region-and-borrow system in the core calculus may not earn its
//! complexity. The suggested first cut is five labels — Pure, Alloc, Mut, Io,
//! Unsafe — with regions deferred until something actually needs them. The
//! structure above is the target, not the starting point.
//!
//! ## Open questions
//!
//! - Can Julia's escape analysis be reused to infer regions, or must they be
//!   inferred from scratch?
//! - Does ccall get a universal bottom effect, or a trusted annotation that
//!   someone has to sign for?
//! - Are effect rows part of h_def, h_type, or both? (Leaning: both — a pure
//!   function and an impure one with the same body are not the same function,
//!   and callers dispatch on the difference.)
