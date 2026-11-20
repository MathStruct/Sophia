#open-problem

> **Status: open (2026-11-20).** Area: type theory, content addressing · Prerequisites: a Lean reader (not built), SPEC-006 · Related: [[Lean Overview]], [[Metamath Overview]], [[lean_statements]], SR-137.
>
> Sources: Lean 4 `Expr.eqv` (v4.36.0-rc1, `src/lean/Lean/Expr.lean`: "alpha equivalent. Binder annotations are ignored"); lean4export 3.1.0 `Export.lean` (`visitedExprs : HashMap Expr Nat`). code: `crates/sophia-ingest/examples/lean_statements.rs`. Original to this vault, from E-10 (B-029).

## Question

If a Lean theorem's identity in Sophia is its statement (its kernel type), as a Metamath theorem's is, **which parts of the type belong to it?** The candidates are the binder names, the binder annotations (explicit, implicit, strict-implicit, instance-implicit), the names and order of the universe parameters, and the names of the constants it mentions (or their ids). Two theorems that differ only in binder annotations mean the same proposition to the kernel. To a user they are not interchangeable: `foo (h : a ≤ b)` and `foo {h : a ≤ b}` are applied differently. The question is whether such theorems get one id or two, and if one, what records the difference.

## Setting

- A **statement** is a declaration's type `T`, a closed kernel expression, with universe parameter list `us`.
- `≡_x` is syntactic equality of `(us, T)` erasing `x`: `names` (binder names), `ann` (binder annotations), `uparams` (universe parameter names, replaced by their positions in `us`).
- lean4export shares subterms by `Expr.eqv`, i.e. it identifies `T` and `T'` when `T ≡_{names, ann} T'`, and prints one representative's names and annotations. An export therefore cannot tell apart what `≡_{names,ann}` identifies.

## What is known

- **Measured** (`lean_statements`, Mathlib `98a293f`, 2026-11-20; [[Lean Overview]] §E-10): the export's node identity, exact hashing, α-hashing and hashing without annotations all give 16 422 or 16 423 groups of repeated statements; every statement node has a distinct hash in all three modes. Renaming universe parameters by position gives 17 372 groups: 1 045 more theorems that repeat another's statement.
- **Known**: the kernel ignores binder names and annotations in type checking and definitional equality; they matter to the elaborator and the pretty printer.
- **Not measured**: how many repeat groups would split if annotations counted. That needs an export that keeps them (a modified lean4export, keyed on `Expr.equal`).

## What would count as an answer

A decision rule for the id (which of `names`, `ann`, `uparams` are erased), with the number of Mathlib groups it gives, measured on an export that keeps annotations. If annotations are erased, a place where the per-name annotations are recorded: a rendering fact of the name rather than part of the statement, as names already are for definitions (SPEC-001).

## Why it matters here

Identity is sacred (SPEC-006 §6): choosing wrongly is a migration. If annotations are erased, two Mathlib theorems with different calling conventions share one id, and a user's name must carry the convention. If annotations are kept, a theorem's id changes when someone makes an argument implicit, though the proposition is the same, and the Metamath lesson (one statement, several proofs) applies less.

## Leads

- Metamath has no binders of this kind; its precedent does not decide this.
- Treating annotations as part of the *name's* signature (how it is applied) rather than of the statement's identity fits Sophia's split between names and content.
- Exporting with `Expr.equal` (structural equality including binder names and annotations) instead of `Expr.eqv` is a one-line change to lean4export's cache. It makes the export larger and the measurement possible.
