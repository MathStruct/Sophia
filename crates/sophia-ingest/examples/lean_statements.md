#implementation #measurement

> E-10 (B-029): Mathlib's repeated statements at four levels of equality, the cause of each group (generated, alias, duplicate), the constants a statement and a proof mention, the declarations only proofs reach, and the axioms behind each theorem. A measurement on a lean4export NDJSON file (format 3.1), not a reader for the store. Results: [[Lean Overview]] §E-10.
>
> Sources: lean4export's `format_ndjson.md` (3.1.0) and `Export.lean`; Lean's `Expr.eqv` (`src/lean/Lean/Expr.lean`, v4.36.0-rc1). code: `lean_statements.rs`

```sh
cargo run --release -p sophia-ingest --example lean_statements -- $SOPHIA_CORPORA/mathlib.ndjson $SOPHIA_CORPORA/e10/mathlib
```

Prints a summary and writes `OUTDIR/groups.tsv` (one line per group at the `universes` level: cause, size, members; `[g]` marks a generated name, `[→x]` an alias of member `x`). Mathlib: 55 s, 5.1 GB peak (release build). Reviewed by Codex ([[Review 2026-11-20 Codex B-029 (external)]]).

**Reading.** Names, levels and expression nodes go into flat arrays indexed by the export's own numbers (20 bytes per node). Expression lines are read by key search, as in [[lean_export_stats]]; name, literal and declaration lines through `serde_json`, because names and strings can carry escapes. An inductive line becomes one declaration per type, constructor and recursor, each knowing its group.

**Statement hashes.** Every node a theorem's type reaches is hashed (BLAKE3, 128 bits) children first, in four modes: `exact` (binder names, binder annotations, universe parameter names), `alpha` (binder names erased), `binders` (annotations erased too), and universe names erased. A theorem's key adds its number of universe parameters, and in `exact` their names. The `universes` level renames each universe parameter by its position in the declaration's list. It is computed per theorem, for the members of the universe-erased groups only. `mdata` is transparent (the export removes it anyway).

**What the export can show.** lean4export shares nodes through `HashMap Expr Nat`, whose `BEq` is `Expr.eqv`: α-equivalence with binder annotations ignored. So two subterms that differ only in a bound variable's name or in being implicit rather than explicit are one node, printed with the first occurrence's name and annotation. The tool checks this and prints the count of distinct hashes per mode: on Mathlib the first three modes give the same 19 855 398 hashes, one per node. The `exact`, `alpha` and `binders` levels therefore coincide by construction of the export, and only a different export could separate them. The one difference between `node` (grouping by the type's node) and `exact` is a pair with the same type but different universe parameter lists. A theorem's key includes its list, since a declaration with an extra (even unused) universe parameter is used differently.

**Causes** (at the `universes` level):
- *generated*: at most one member has a name a person wrote. A name is generated if one of its components starts with `_` (`_simp_1_2`, `_proof_3`, `_private`, `_auxLemma`) or is a number, `eq_<n>`, `proof_<n>`, `match_<n>`, `eq_def`, `injEq`, `sizeOf_spec` or `ext_iff`. A rule of thumb from reading the groups, not Lean's own definition: a hand-written `ext_iff`, `_foo` or `eq_17` is counted as generated.
- *alias*: the hand-named members form one class under "m's proof, under its lambdas, is member k applied to bound variables only" (Mathlib's `alias`, `theorem b := @a`).
- *duplicate*: two or more hand-named members whose proofs are not such references. Split into exclusive sub-classes: the same proof node (α-equal proofs); otherwise a proof that mentions another member other than as a bare alias (derived); the rest have independent proofs. Alias chains may pass through generated members.

**Dependencies.** Per theorem, the distinct constants its type mentions and those its value mentions, directly (`const` and `proj` type names). Over the library, two closures through types, definition bodies (not theorem or opaque bodies) and inductive groups: one from all theorem statements, one from all theorem proofs. A declaration is "only proofs" if only the second reaches it. The axioms behind a declaration are propagated through types, values (theorem proofs included) and inductive groups (a type rests on its constructors' types), iterated to a fixpoint. Mathlib needs 13 rounds; it has no forward references outside inductive groups.

**Assumptions, checked or stated.** The hash pass needs every child numbered below its parent; this is asserted. There may be at most 64 axioms, also asserted. Lines are read by key search on lean4export's compact output: whitespace after a colon, or `natVal` written with leading zeros, is not handled (lean4export writes neither). Levels are hashed recursively, so a universe chain of 10⁵ `succ`s would overflow the stack (Mathlib's are short).

**Not done.** Binder names and annotations cannot be compared (above). Generated names are classified by pattern. `to_additive` and `simps` copies have different statements and do not appear as repeats. Transitive constant counts per theorem are not computed.
