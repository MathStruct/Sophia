#open-problem

> **Status: rule adopted (2026-10-20); to be enforced.** Related: [[Real Code Corpora]] §4, `crates/sophia-ingest/src/metamath.md`, SPEC-001 §4.

**Problem.** A record's `kind` is stored with it but is not part of its id. When two records with the same id arrive with different kinds, the first one wins. Metamath showed this is a real error, not a corner case. A statement is an axiom in one library and a theorem in another, and with identity by statement only one label survives.

**Rule.** A record's kind must be a **function of its canonical content** (for Metamath: `statement` or `syntax`, from the typecode). Anything that can differ between users of the same content is a **role**, recorded where it belongs:
- in a nominal collection (a library's axioms);
- in an annotation (a proof);
- in a name (a label).

**To do.**
- `Store::add` should reject a record whose id exists with a different kind, rather than keep the first silently. Today the second is skipped.
- The sophialang normaliser already derives kinds from content.
