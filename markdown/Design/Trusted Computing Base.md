# Trusted Computing Base

If the point of storing [[Equivalence and Witnesses|equivalence proofs]] is that claims are checkable rather than assumed, then it matters exactly what still has to be taken on faith. A proof system whose TCB includes the optimiser has not moved any trust.

## What is *not* trusted — the good property

**The database itself is not in the TCB.** Because every node's identity is a hash of its own content ([[Hashing and Identity]]), any consumer can re-verify any node by rehashing it. A malicious or corrupted store can withhold data or return the wrong data, but it cannot substitute a different definition under an existing hash. This is the same property Nix and Git have, and it is worth stating plainly because it removes the largest component from the TCB for free.

Also not trusted:

- **Frontends.** A frontend produces core terms; the kernel re-typechecks them. A buggy frontend produces terms that fail to typecheck, or that typecheck but mean something other than the source — the latter is a real risk, handled by differential testing ([[Naming and Change Propagation]] and the roadmap's conformance suites), not by trust.
- **The e-graph engine.** It proposes equivalences; the witness checker replays the rewrite chain. A bug in saturation produces a bogus chain that fails replay.
- **The extractor.** It chooses representatives; the choice is only sound because the class members are already equivalent.
- **SMT solvers.** Only their *certificates* are consumed, and only by an independent checker.
- **The network / cache / mirrors.** Content-addressing again.

## What *is* trusted

| Component | Size (target) | Why it's irreducible |
| --- | --- | --- |
| The hash function + canonicaliser | ~2–3 kLOC | Defines identity. A canonicalisation bug conflates two different programs. |
| The SC kernel (typechecking + conversion) | ~3–5 kLOC | Decides `defeq` and validates every term |
| The witness checker | ~1–2 kLOC per witness kind | Turns evidence into licence |
| The rewrite rule set | grows | Each rule is an axiom unless itself witnessed |
| The per-language-pair relation `R` | per pair | The content of every cross-language claim |
| The runtime + target model | large | Effects, layout, memory model |

The canonicaliser being in the TCB is the uncomfortable one and is specific to this design. In Lean or Coq, the kernel is trusted but identity is nominal, so a "canonicalisation bug" is not a category that exists. Here, `h` being wrong means two genuinely different definitions become the same node — a silent, global, catastrophic failure. Mitigations:

- Canonicalisation must be *total and deterministic*, with a property test that `h(t) = h(u) ⟹ normalise(t) = normalise(u)` structurally, checked by round-tripping.
- Never remove information during canonicalisation that is not provably semantics-irrelevant. When in doubt, keep it and let it be a distinct node; an over-fine hash costs cache hits, an over-coarse hash costs correctness.
- Version everything so a fix is a migration rather than a silent reinterpretation.

## The LLVM/MLIR problem

The compiler backend is enormous and is not going to be verified. [[Start Here]] explicitly says "the compilation goal should be LLVM/MLIR as I do not want to get into the weeds of writing a compiler backend" — correct decision, with a consequence: **everything below the `Term → Op` boundary is untrusted and unverified.**

The available responses, in increasing order of cost:

1. Accept it. All guarantees are stated about core terms, and lowering is best-effort. This is what every proof assistant that extracts to OCaml already does.
2. **Per-instance translation validation.** Run an equivalence checker on each lowering and store the result as a `Witness` ([[State of the Art - Program Equivalence Checking]]). Alive2 demonstrates this is practical for LLVM at the level of individual optimisations.
3. A verified backend (CompCert-style) for a restricted subset. Out of scope, but worth noting as the endpoint.

(2) fits this architecture unusually well: it is per-instance, produces a citable artifact, and the artifact is exactly the shape of a node in the graph. It is the single highest-leverage thing to adopt from the existing literature.

## Trust levels as first-class data

Rather than a binary, every artifact carries a computed trust label derived from the weakest link in its derivation:

$ "trust"("artifact") = min{ "level"(e) : e ∈ "derivation"("artifact") } $

with the `modulo` sets unioned along the way ([[Equivalence and Witnesses]]). A build can then be *policy-constrained*: "produce this binary using only `defeq` and `rewrite` equivalences, modulo at most `{alloc}`". Builds that required an `asserted` edge are still possible but are labelled, and the label names the asserting author.

This turns the uncomfortable parts of the design into a feature: the system does not pretend everything is proved, it records precisely what is not.

## Adversarial considerations

Not a security product, but worth noting since content-addressed stores get shared:

- **Hash collisions** would let an attacker substitute code. 256-bit [[BLAKE3]] makes this a non-issue; the 128-bit UUID truncation is display-only for exactly this reason.
- **Malicious witnesses** are handled by the checker; the risk is a bug in a checker, which is why each is small and separate.
- **Malicious `asserted` edges** are the real hole, which is why they must be signed and attributable and are non-substitutable by default.
- **Resource exhaustion** during canonicalisation (huge SCCs), saturation (e-graph explosion) or extraction (ILP) needs hard budgets, not heuristics.

## Related

- [[Hashing and Identity]]
- [[Equivalence and Witnesses]]
- [[Cross-Language Semantic Hazards]]
- [[State of the Art - Proof-Carrying Code and Verified Compilation]]
- [[State of the Art - Program Equivalence Checking]]
- [[Open Problems and Risks]]
