#implementation #evidence

> Proof trust records (SPEC-006 §5; E-0, B-021): what an external checker said about each proof, stored as one annotation per proof, and two queries that split proofs into those resting only on checked proofs and those resting on at least one unchecked one. Used by `sophia-mm-trust` with metamath.exe; results in [[Benchmark Results]] and [[Metamath Overview]]. Source: `trust.rs`.
>
> Sources: [[SPEC-006 Worlds, Artifacts and Evidence]] §5; [[Semantic Boundary Experiments]] E-0; [[Trusted Computing Base]]. code: `trust.rs`, `tests/trust.rs`, `sophia-bench/src/bin/sophia-mm-trust.rs`

- **One annotation per distinct proof** (`proof_checks`): kind `checked` (status `checked`) or `check-failed` (status `failed`); subject the proof's id; `checker` and `checker_version` the program (`metamath.exe`, its version); payload the trusted kernel in canonical encoding (`proof-check-v0`); `inputs` the ids the check ran against (the library's source manifest). Two labels with one proof (a repeated theorem) are one record and fail if either failed. A failed label with no proof comes back as `unknown`.
- **Not recorded:** the certificate (a Metamath proof is its own certificate and is in the store) and the assumptions (the axioms and definitions a proof's references reach: a query, as `sophia-mm-check` does). The world is the source manifest in `inputs`.
- **`metamath_verify_failures`**: the labels on metamath.exe's `?Error … label "X"` lines from `verify proof *`; run with `set width 100000` so the label is on the first line.
- **The queries** `RESTS_ON_CHECKED` / `RESTS_ON_UNCHECKED`: a proof is *tainted* when it has no `checked` annotation or references a tainted proof. Taint flows up the references once, so the cost is linear in the references, not one transitive closure per theorem. Stratified negation (`!checked`, `!tainted`).

## Limits (what this does not do)

- **Checker policy is not expressible in a query.** The base relation `annotation(a, kind, subject)` shows neither the checker, nor its version, nor the status, so "trust only metamath.exe ≥ 0.198" cannot be written; the queries trust any `checked` annotation. The kind carries the verdict for that reason. Exposing the checker (a base relation `checked_by(a, checker, version)` or the status) changes the query schema (`store-base-v0`): a proposal, B-081.
- **Pass and fail together:** the kind is part of the annotation's id, so a later `checked` does not replace an earlier `check-failed` (nor the reverse); a proof with any `checked` counts as checked. For a deterministic checker on content-addressed proofs both can only coexist after a checker bug or a version change, which the version would show if the query could see it.
- Metamath only. Lean (E-10) would record the kernel `Lean 4 kernel` and the axioms a theorem uses (`#print axioms`).
