//! Metamath frontend — the Roadmap M0 target.
//!
//! Design notes: `metamath.md`, `markdown/Design/Roadmap.md`.
//! COMMENTS ONLY — nothing here is implemented.
//!
//! ## Why Metamath is first, and why that is not obvious
//!
//! Metamath is nothing like Julia. It has no types to infer, no effects, no
//! memory model, no dispatch and no optimisation. Choosing it first is
//! therefore NOT a step toward the real goal in any semantic sense.
//!
//! It is chosen because it isolates the PLUMBING from the SEMANTICS. At M0 the
//! only questions being answered are: does canonicalisation terminate and
//! produce stable digests, does the SCC construction handle real dependency
//! structure, does the store round-trip, and does re-ingestion write zero new
//! rows. Every one of those is answerable without a single hard semantic
//! decision, and all of them are load-bearing.
//!
//! set.mm is also an excellent corpus for this: ~40k theorems, deeply nested
//! dependency structure, and an existing verifier to check against.
//!
//! ## What Metamath actually is
//!
//! A proof is a sequence of substitutions from axioms, checked by string
//! manipulation over a token stream. The whole grammar:
//!
//! ```text
//! $c ... $.        constant declarations
//! $v ... $.        variable declarations
//! $f typecode var $.        floating hypothesis (a variable's type)
//! $e typecode expr $.       essential hypothesis (an assumption)
//! $a typecode expr $.       axiom
//! $p typecode expr $= proof $.    theorem with proof
//! ${ ... $}        scoping block
//! ```
//!
//! Proofs come in two encodings — a normal one (a list of label references)
//! and a compressed one (a base-20/base-5 scheme with back-references). Both
//! must round-trip.
//!
//! ## Mapping into the graph
//!
//! ```text
//! $a, $p statement   -> Decl  (the statement) + Prop (what it asserts)
//! proof                -> Witness { Kernel }
//! $e hypothesis        -> part of the Decl's context
//! $f hypothesis        -> the variable's typecode
//! label                -> Name binding, NOT part of the hash
//! $c / $v              -> Term nodes for the symbol alphabet
//! scoping block        -> context, flattened during ingestion
//! ```
//!
//! Note that this is already the knowledge layer in miniature: a theorem is a
//! Decl, its statement is a Prop, its proof is a Witness. Metamath is a
//! working example of the shape markdown/Design/Graph Schema.md proposes,
//! which is a second reason to start here.
//!
//! ## Elaboration into SC
//!
//! Two options, and M0 should do the first:
//!
//!   A. SHALLOW: represent Metamath's own objects as SC data (symbols as
//!      constants, expressions as lists, proofs as substitution sequences).
//!      Tests the plumbing. Does not pretend to be a semantic embedding.
//!   B. DEEP: interpret Metamath's logic in SC's Prf fragment. Interesting,
//!      much harder, and not needed to answer any M0 question.
//!
//! Doing A and being honest that it is A avoids the classic trap of a "first
//! frontend" that quietly becomes the template for the real ones.
//!
//! ## M0 acceptance tests
//!
//! ```text
//! 1. Ingest set.mm; re-export; byte-identical output.
//! 2. Ingest set.mm twice; the second ingestion writes ZERO new nodes.
//! 3. Ingest on two machines; digests match exactly.
//! 4. Every stored proof still verifies through a Metamath checker.
//! 5. Report: nodes/s, store size vs source size, latency to fetch one
//!    theorem with its full proof tree.
//! ```
//!
//! Test 2 is the important one. If it fails, canonicalisation is
//! nondeterministic and nothing else in the project is worth building yet.
//!
//! ## What M0 deliberately does NOT test
//!
//! Types, effects, dispatch, lowering, optimisation, equivalence, or anything
//! about performance of generated code. Those arrive with the Lisp frontend at
//! M1 and with Julia at M3.
