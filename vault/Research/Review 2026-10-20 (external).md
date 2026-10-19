#reference #open-problem

> An external review of the push of 2026-10-20, written by ChatGPT and passed on by Daniel. It reviewed commit `27cb3a7` (M0/M1 and the vault), not the M2 work that followed. It worked by static reading only: it built and ran nothing. This note records each point, **whether it was right**, and what was done about it. The requirements it led to are SR-75 to SR-89 in [[Requirements]]. Its three proposed acceptance demonstrations are now the **P0 gate** in [[Milestones]].
>
> Sources: the review text (in the conversation of 2026-10-20); the code and tests named below.

## Verdicts

| § | Point | Verdict | What was done |
|---|---|---|---|
| 2 | **Symmetric recursive components lose information** (`f() = g(); g() = f()`): members share an index, the store keeps one record, verification fails | **Correct, reproduced.** `tests/symmetric.rs` failed on both backends before the fix | Fixed by an explicit **quotient**. Automorphic members are one class, and the component is serialised over its classes, so `f`, `g` and `h() = h()` are one definition (identity up to bisimilarity). Regression tests cover both backends, permutation, renaming, and asymmetric and partially symmetric groups ([[component]], SPEC-001 §4.1) |
| 1 | Edits need a **projection token**, not just the name's version, because the profile may change between fetch and save | **Partly right.** The profile itself is not a hazard here: the parser reads every editable profile, so fetched text means the same after a switch. But checking it turned up a **real hole the review did not name**: saving a recursive group also updated the group's *other* names without checking their versions | The version Moonkale receives is now an **edit token** over every name the text covers (path, version, target). A concurrent edit of `odd` makes a stale save of `even` a conflict. Test: `group_members_cannot_be_overwritten_silently` (SPEC-004 §3b) |
| 1 | Picking "the oldest public name" when a bare definition is opened for editing is unsafe with aliases | **Correct** | SPEC-004 §3 now requires Moonkale to keep the navigation context, or to ask which name to edit. The interface already refuses edits through a bare definition |
| 3 | **Reproducible worlds:** builds, proofs and tests must be evaluated against an immutable snapshot of method tables, not the mutable current state | **Correct**; SPEC-002 §4 had only per-fact relation versions | `Store::snapshot_world` / `world_members`: content-addressed world snapshots, with a store-level demonstration on both backends. SPEC-006 §2 specifies their role in builds, tests and proofs |
| 3 | `is_stale` invalidates conservatively, not by signature overlap | **Correct, and already known** (SP-006, "today any change to a relation makes all its facts stale") | Stays open in SP-006, measured by B-09. Recorded as SR-85 |
| 3 | A common abstraction: versioned nominal collections of immutable definitions | **Agreed.** It is what the store already implements, for both methods and rules | Named in SPEC-006 §1 |
| 4 | **Contradiction:** SPEC-002 says proofs never change compiled code, yet proofs enable optimisations (devirtualisation, removed checks) | **Correct** | SPEC-002 §3 is corrected: proofs never change a definition's **semantic identity**, but they may change **artifact identity** = H(semantic id, compiler, target, flags, world, *selected evidence*). SPEC-006 §3 |
| 4 | Optional type systems need a **mandatory representation and execution contract** | **Correct** | SPEC-002 §3a: representation is fixed by representation types, erasure (usage 0) and nominal layouts; no optional checker may contradict it |
| 5.1 | What does unchecked code mean, e.g. a linear value used twice with no checker? | **Correct, unspecified** | SPEC-003 §7: mandatory well-formedness, call-by-value semantics, and "stronger annotations without evidence are not exploited": a linear binder used twice executes as ω, and only proved properties license optimisations |
| 5.2 | Learned parameters are program data, not annotations | **Correct.** SPEC-003 Q8 had treated learned weights as "annotations with provenance" | SPEC-006 §4: parameter artifacts are content-addressed, and a model instance is (program id, parameter artifact id) |
| 5.3 | The core needs an explicit effects boundary | **Correct** | SPEC-003 §8 reserves the pure/effectful split and a call-by-push-value evaluation |
| 6 | Computational trinitarianism overclaims universal equivalence | **Correct about the tone.** The correspondences hold for specific structures | [[Computational Trinitarianism]] §0 is a new scope section. Cross-system use goes through explicit interpretations with **preservation obligations** (SR-84) |
| 7 | Benchmarks are synthetic; add real repositories and edit-to-graph latency, cold and warm caches, memory | **Correct** | [[Benchmark Plan]] B-15 (edit-to-graph latency) and B-16 (real corpora); memory and latency distributions added to the principles |
| 8.1 | De Bruijn indices do not solve hygienic macros | **Correct.** [[Compression and Canonical Form]] §3.2 overclaimed | Corrected there, and recorded as SR-86 |
| 8.2 | The "exact closure" overclaims: reflection, dynamic loading, foreign calls, dispatch and generated code need a closed-world contract | **Correct** | [[Minimal Binaries]] §1 now states the contract (SR-87) |
| 8.3 | Cache keys must include world, toolchain, flags, target, libraries and evidence | **Correct** | SPEC-006 §3 |
| 8.4 | Proof trust: checker, kernel, assumptions, certificates, provenance; not one `checked` flag | **Correct** | SPEC-006 §5 (SR-82) |
| 8.5 | The language plug-in interface is proposed but not implemented | **Correct.** SPEC-001 §3 describes a `Language` trait that does not exist; sophialang is wired in directly | SPEC-001 §3 now says so. The trait comes with the second language |
| 8.6 | Rendering profiles are specified but not implemented | **Outdated.** True at `27cb3a7`; implemented in M2 (`85d6bbb`) | none needed |
| 8.7 | Identity migration must separate canonical identity from physical encoding | **Correct** | SPEC-006 §6 (SR-88) |
| — | "Avoid treating M0/M1 as settled before identity, worlds and projections are validated" | **Agreed.** The symmetric bug proves the point | The P0 gate in [[Milestones]] |

## What the review got wrong or missed

- **Profile switches** are not, by themselves, a silent-overwrite risk, because the parser accepts every editable profile. The real risk was elsewhere (row 2).
- **The quotient is the better of its two options here,** not the harder one. It is canonical, it makes the store's dedup and verification agree, and it matches the semantics: automorphic members are bisimilar.
- **It reviewed an older commit,** so rendering profiles and the interface (M2) were outside it.
