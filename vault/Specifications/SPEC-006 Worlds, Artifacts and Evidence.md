#design #definition

> **Draft, 2026-10-20 (Claude), from the external review of that day ([[Review 2026-10-20 (external)]]).** What makes results in Sophia **reproducible**:
> - immutable **world snapshots** of the mutable nominal relations;
> - **artifact identities** that say exactly what a compiled result depends on, including the proof evidence it used;
> - **parameter artifacts** for learned values;
> - **trust records** for proofs;
> - **migration** relations between identities.
>
> It complements [[SPEC-001 Sophia Store v0]] §8 (annotations and facts) and [[SPEC-002 Sophialang Principles]] §3–§8. It meets SR-76, SR-79, SR-80, SR-82, SR-85, SR-87 and SR-88.
>
> Sources: Julia's world age (Belyakova et al., *World Age in Julia*, OOPSLA 2020); Nix and Bazel (input-addressed builds); Necula, *Proof-Carrying Code*, POPL 1997; [[Trusted Computing Base]]. code: `Store::snapshot_world`, `Store::world_members` (implemented); the rest is planned for M4/M5.

## 1. Versioned nominal collections

The store already has one mechanism for every open set:
- a **nominal entity** `N` (fresh id);
- a **member set** of immutable content-addressed definitions;
- a version that moves on every change.

Written `Members(N, v) = {d₁ … dₙ}`, it serves:
- generic functions, whose members are methods (dispatch);
- relations, whose members are a signature plus rules (logic evaluation, S-ADR-0009);
- later, any extension set an evaluator interprets.

The evaluator decides what the members mean. The store only versions them.

## 2. World snapshots (SR-76)

A **world** is an immutable, content-addressed snapshot of every nominal entity's member set:

```
world = BLAKE3( field("sophia","world-v0") ‖ for each nominal N in id order: field("nominal", id(N) ‖ sorted member ids) )
```

- **Implemented** (2026-10-20): `Store::snapshot_world()` returns the id and stores the snapshot. `Store::world_members(world, N)` reads it back.
- The id depends only on the member sets, so two stores with the same tables get the same world, and returning to an earlier table gives the earlier world back. A conformance test checks this on both backends.
- **What cites a world:** every build, test run, proof check and dispatch resolution records the world it ran in. Running "the program `f` in world `w`" is reproducible even after methods are added. This is Julia's world age, made persistent and shareable.
- **What a world does not capture:** names. Programs refer to definitions and nominal entities by id, not by name, so names are not needed to run.
- **Per-environment worlds** (CLOSOS's multiple environments, [[Operating Systems and Sophia]]): a user's extra method lives in their own world. That is later work.
- **Size:** a world lists every member. Snapshots that share structure (a Merkle tree per nominal entity) are a later optimisation, if B-09 shows a need.

## 3. Artifact identity: where proofs may change code (SR-79)

Three identities, never conflated:

```
C = content id of the program            (semantics; never depends on proofs)
P = content id of the evidence           (a proof term, a certificate)
A = BLAKE3( C ‖ world ‖ compiler id ‖ compiler version ‖ target ‖ flags ‖ linked libraries ‖ sorted ids of the evidence the compiler used )
```

- **A proof never changes `C`** (SR-53 is about source identity). It **may change `A`**. A proof that a call resolves statically lets the compiler emit a direct call. A memory-safety proof lets it drop a check. The artifact then depends on that evidence, and records it.
- **The compiler states which evidence it used.** An artifact compiled without evidence is the conservative one, and is always valid.
- **The cache key is `A`.** Changing the world, the toolchain, a flag, the target or the evidence used gives a different artifact (the review's §8.3).
- **Closed-world contract** for standalone binaries (SR-87, [[Minimal Binaries]] §1): a binary is built against one world. It may not use reflection over the store, dynamic loading of definitions outside its closure, or run-time code generation, unless it links the corresponding runtime, which the build reports. Foreign calls name their libraries, which enter `A`.

## 4. Parameter artifacts (SR-80)

Learned values (weights, rule weights after training, fitted constants) are **program data**, not annotations:
- a **parameter artifact** is an immutable, content-addressed blob of values with a declared layout (its type: shapes and element types), stored as a definition of kind `artifact` in a language `blob`;
- a **model instance** is the pair `(program id, parameter artifact id)`, itself content-addressed;
- **training** is a derived fact `trained(instance, from: (program, initial artifact, data artifacts, world, trainer, seed) → artifact)`, so provenance is recorded and replayable;
- a **declared** rule weight in source stays part of the rule's content (SPEC-003 §3). A **learned** weight is a parameter artifact. This replaces SPEC-003's earlier answer to Q8 ("learned weights are annotations").

This matters for Lenticulum and for scientific computing: two runs of the same program with different parameters are different instances, never "the same function with a different annotation".

## 5. Proof trust (SR-82)

A proof annotation records:
- the **checker**: its name, version and content id (when the checker lives in the store);
- the **trusted kernel** the checker reduces to (for example "sophialang-dependent kernel v0", "Lean 4 kernel", "Z3 4.13 with proof logging off", meaning not independently checkable);
- **assumptions**: postulates and axioms used, by content id, and `asserted` claims it depends on;
- the **certificate**, if any: a re-checkable artifact (a proof term, an LRAT certificate, an e-graph rewrite chain);
- the **world** and the **inputs** checked against.

A query or a build policy can then say which checkers and kernels it trusts. "Checked" is never one global flag. A proof that rests on an `asserted` claim inherits that claim's status, and queries can trace this ([[Trusted Computing Base]]).

## 6. Migration of identities (SR-88)

Identity has three layers that change independently:
1. **canonical identity**: the normaliser version and the framing version;
2. **hash function**: BLAKE3-256 today;
3. **physical encoding**: how a backend stores bytes.

Rules:
- A change to (1) or (2) is a **migration**. It produces `migrated_to(old id, new id, from version, to version)` relations, a new world, and stale proofs (SPEC-001 §4.5).
- A change to (3) is **not** a migration. Ids are unchanged, and a backend may re-encode freely (for example a compact storage encoding, B-06), as long as the canonical bytes can be reproduced for verification.
- Facts and artifacts keep their inputs' old ids. A migrated store answers queries about old ids through `migrated_to`.

## 6a. Transformations and their evidence (interface only)

Added 2026-10-20 after the M3a review (§10), so that every optimiser and frontend records its work the same way. **Not built yet.**

Sophia now has two kinds of reusable computation, programs (sophialang and imported languages) and queries (`sophia-query`), and several transformations between them:
- magic sets;
- normalisation;
- specialisation and partial evaluation;
- lowering to IR;
- later, mined abstractions.

Each application of a transformation **T** is one record:

```text
Transformation {
    by:        T's name and version            # e.g. "sophia-query/magic@0"
    input:     content id(s)                   # Q, or a definition
    output:    content id(s)                   # Q′
    relation:  equivalent | refines | lowers-to | specialises(at: …)
    obligation: what must hold                 # e.g. eval(Q, D) = eval(Q′, D) for all D
    evidence:  none | tested(test ids) | checked(certificate id, checker) | trusted(T)
    inputs:    world / snapshot / parameters it depended on
}
```

- It is a **derived fact** (SPEC-001 §8) whose subject is the input, so staleness and provenance work as for any fact.
- **Evidence follows §5 (proof trust).** A transformation run by a trusted optimiser is `trusted(T)`. Its differential tests make it `tested`. An independently checked certificate makes it `checked`. Compiled artifacts may rely only on what their evidence level allows (§3).
- **The first producer will be the magic-set rewrite.** Its outputs are queries, so they can be stored; its obligation is query equivalence; its evidence today is the differential property test.

## 7. Status and order

| Part | Status |
|---|---|
| §1 nominal collections | implemented (methods, relations) |
| §2 world snapshots | implemented in the store; citing worlds in facts comes with M4 |
| §3 artifact identity | M5 (compilation) |
| §4 parameter artifacts | specified; needed when Lenticulum's models enter (with M4 execution) |
| §5 proof trust | specified; fields added to annotations when the first checker exists (M4/M6) |
| §6 migration | specified; the first real migration will be the first normaliser change |

## Related

[[SPEC-001 Sophia Store v0]] · [[SPEC-002 Sophialang Principles]] · [[SPEC-003 Sophialang Core v0]] · [[Minimal Binaries]] · [[SP-006 Open method tables versus content addressing]] · [[Review 2026-10-20 (external)]]
