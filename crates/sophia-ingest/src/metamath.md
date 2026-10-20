#implementation

> The Metamath frontend: `.mm` text to content-addressed statement and proof records. Source: `metamath.rs`.
>
> Sources: Megill and Wheeler, *Metamath: A Computer Language for Mathematical Proofs* (2019), ch. 4 (the language, compressed proofs); [[Real Code Corpora]] §3. code: `metamath.rs`

**A statement's identity** covers:
- its mandatory hypotheses in order: floating ones as (typecode, variable), essential ones as (typecode, math string);
- its mandatory disjoint pairs;
- its typecode and math string.

Variables are numbered by first occurrence and constants kept as text. Labels and variable names are display data. The record kind is `statement` or `syntax`. It **must follow from content**: the kinds axiom, definition and theorem were first-writer-wins, which mislabelled shared statements. They are now the per-library role, recorded as collections.

**A proof** is a separate record of kind `proof`. It names the statement it proves (`of`), then lists what it uses, each once:
- other theorems, through **their proofs' ids**: the justification the author chose;
- axioms, definitions and syntax constructors, through their statements' ids;
- dummy variables, by typecode and number.

The steps are varints into those lists. The design was reached in four corrections, each found by running real queries (see [[Real Code Corpora]] §4):
1. **Proofs are not identity.** Hashing proofs into theorems made 12 059 statements shared by set.mm and iset.mm look different.
2. **"Proves" is not "uses".** Proofs referenced the statement they prove as an ordinary reference.
3. **Roles belong to the library.** Axiom versus theorem had been stored as a record kind.
4. **A step must name its justification.** A step that referenced a statement lost which justification the author used.

`parse` handles normal and compressed proofs, `${ $}` scopes and `$d`, but not file inclusion (`$[ $]`). It does not verify proofs: Sophia stores them; a checker would be a fact producer (SPEC-006 §5).
