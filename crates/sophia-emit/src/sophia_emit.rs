//! `sophia-emit` — lowering core terms into MLIR and LLVM IR.
//!
//! Design notes: `sophia_emit.md`, `markdown/Design/Compilation as Query.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! markdown/Start Here.md is explicit that "the compilation goal should be
//! LLVM/MLIR as I do not want to get into the weeds of writing a compiler
//! backend". Correct decision. This crate is therefore a TRANSLATOR, not a
//! compiler: it gets core terms into a form that someone else optimises and
//! codegens, and it writes the result back into the store.
//!
//! ## Lowering pipeline
//!
//! ```text
//! SC core term
//!   -> sophia dialect            (a 1:1 MLIR rendering of SC, so the first
//!                                 step is structural and auditable)
//!   -> func / arith / scf / cf   (standard dialects)
//!   -> memref / llvm             (memory and the final dialect)
//!   -> LLVM IR -> object code    (not ours)
//! ```
//!
//! Introducing a `sophia` dialect first is worth the extra step: it makes the
//! SC -> MLIR boundary a mechanical, checkable translation, and pushes the
//! semantic decisions into ordinary MLIR conversion patterns where existing
//! tooling can see them.
//!
//! ## Intended API
//!
//! ```ignore
//! pub fn lower(term: Hash, target: Hash, store: &mut dyn Store)
//!     -> Result<LoweringResult>;
//!
//! pub struct LoweringResult {
//!     ops: Vec<Hash>,          // Op nodes written back
//!     module: Hash,            // the LLVM Module node
//!     witnesses: Vec<Hash>,    // translation-validation reports, if run
//!     pipeline: Hash,          // which passes ran, recorded not assumed
//! }
//! ```
//!
//! Recording the PIPELINE is what makes "why did the optimiser do that?" a
//! query instead of a rerun with -print-after-all. That capability is useful
//! on day one even if nothing else in the project works.
//!
//! ## Writing IR back into the store — the decision to measure
//!
//! markdown/Design/Graph Schema.md estimates 10^7..10^8 nodes per package if
//! every IR layer is persisted. Three options, and M2 picks one with numbers:
//!
//!   A. persist everything            (queryable, huge)
//!   B. persist chunked blobs per function, individual rows only for nodes
//!      that are independently referenced      (probably right)
//!   C. do not persist; regenerate on demand, keep only the artifact
//!
//! Default assumption going in: B for MLIR, C for LLVM IR. LLVM IR is cheap to
//! regenerate from MLIR and is the largest layer.
//!
//! ## Everything below this crate is untrusted
//!
//! MLIR and LLVM are enormous and will not be verified. So:
//!
//!   - all guarantees are stated about CORE TERMS, not about the binary
//!   - per-instance translation validation (Alive2-style) is the realistic
//!     mitigation, and its output is a Witness node — which fits this
//!     architecture unusually well, since it is per-instance and citable
//!   - the artifact's trust label is the minimum over its derivation, so a
//!     lowering with no validation witness LOWERS the label rather than
//!     silently inheriting the core term's
//!
//! See markdown/Design/Trusted Computing Base.md.
//!
//! ## Target nodes
//!
//! Lowering is meaningless without a target: triple, datalayout, CPU features,
//! ABI choices, float environment. All of it hashes into `h_target`, which is
//! part of the codegen cache key (markdown/Design/Content-Addressed
//! Precompilation.md). Two builds differing only in `-mcpu` must not share
//! compiled code, and making the target a node rather than a flag is what
//! prevents that.
//!
//! ## Bindings
//!
//! `melior` for MLIR, `inkwell` for LLVM, or the C APIs directly. Version
//! pinning matters: MLIR's C API is not stable across releases, and the
//! pipeline hash recorded above must include the toolchain version or the
//! cache is unsound across upgrades.
