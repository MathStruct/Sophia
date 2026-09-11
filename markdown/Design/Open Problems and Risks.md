# Open Problems and Risks

[[Start Here]] says of its own goals: "we do not know whether they are viable". This note is the attempt to say *which* parts, and how one would find out. Nothing here is an argument against building it; everything here is an argument for the ordering in [[Roadmap]].

## Tier 1 — could invalidate the architecture

### Query cost may exceed recomputation cost

Reconstructing a function's IR from $10^4$–$10^6$ nodes via database queries may simply be slower than re-running a frontend on the source text. Compilers throw IR away not only out of carelessness but because regenerating it is cheap. **Test at [[Roadmap|M2]].** Mitigations are in [[Compilation as Query]]; the fallback (persist core terms, cache IR) preserves most of the value.

### Canonicalisation is in the [[Trusted Computing Base]] and is hard to get right

An over-coarse canonical form silently identifies two different programs. This is a *global, silent* correctness failure with no natural detection point. The discipline — never discard information that is not provably irrelevant — is easy to state and easy to violate under pressure to improve cache hit rates.

### Scale

$10^7$–$10^8$ nodes for a mid-sized package with IR persisted ([[Graph Schema]]). Store size, index build time and query latency at that scale are unknown and none of the candidate databases were designed for it. Chunked subtree storage is the main lever.

### The C++ frontend is enormous

C++ has no realistic parser other than Clang, its semantics contain [[Cross-Language Semantic Hazards|undefined behaviour]] that must be explicitly modelled, and templates mean the elaborated form depends on instantiation context. `Clang_jll` being in `Project.toml` suggests awareness of this. Realistic scope: a subset, via a Clang plugin, at [[Roadmap|M6]], never "C++ support".

## Tier 2 — could shrink the goal to something much smaller

### Nobody will write the proofs

The `observational` and stronger levels of [[Equivalence and Witnesses]] need proof terms. Writing them is expert work and there is no economic incentive for a package author to do it. Most edges in a real deployment would be `tested` or `asserted`, i.e. the non-substitutable ones.

Partial answers: derive `rewrite` witnesses automatically from the [[E-Graph|e-graph]] (free, sound, covers a lot); accept `tested` as first-class and make its limits visible rather than pretending otherwise; harvest proofs from [[State of the Art - Program Equivalence Checking|translation validation]] rather than from humans. The realistic outcome is a database where most claims are evidence and a few are proofs — which is still strictly better than today, where the claims are not recorded at all.

### The modulo union problem makes chained equivalences useless

Formalised in [[Equivalence and Witnesses]]: composing equivalences unions their moduli, so long chains say nothing. If real-world chains are long, the equivalence graph is decorative. Unknown until M4; mitigated by preferring short chains and by recording direct edges aggressively.

### Julia has no formal semantics

There is no document one can point at that says what a Julia program means. Every cross-language claim involving Julia therefore rests on a semantics that this project would have to *invent*, and invent in a way the Julia community would recognise as correct. Compare Rust, where Stacked Borrows / Tree Borrows is still active research, and C, where CompCert needed years. See [[State of the Art - Formal Semantics of Real Languages]].

This is the deepest problem in the whole project and it has no clean answer. The practical stance: state the semantics for the *subset* actually ingested, keep it small, and make the subset boundary explicit and enforced.

### Open-world dispatch undermines static claims

[[Multiple Dispatch]] with a mutable method table means a Julia definition's meaning is relative to a [[World Age|world age]]. Every equivalence claim about a generic Julia function is implicitly conditional on the method table. Making that condition explicit is doable ([[Content-Addressed Precompilation]]'s `DispatchFact` nodes) but it means many claims are narrower than they look.

## Tier 3 — friction, not danger

- **Ergonomics of the hash-first model.** Unison's update-propagation "todo list" is its main usability complaint, and [[Naming and Change Propagation]] inherits it exactly.
- **Tooling ecosystem.** Editors, debuggers, profilers and reviewers all assume files and line numbers. Keeping Git authoritative early ([[Naming and Change Propagation]]) defers this.
- **Frontend drift.** Each frontend version produces different elaborations; the store accumulates near-duplicate subgraphs. Handled by `ELABORATES_TO` carrying the frontend version, and by periodic re-elaboration, but it is real churn.
- **Hash schema migrations** re-hash the world. Cheap in principle, tedious in practice.
- **Database choice lock-in.** Mitigated by the store-trait indirection, but query dialects leak.
- **Two-language build.** Rust + Julia + LLVM + a database is a demanding build for contributors.

## Things that are *not* risks, contrary to first impressions

- **Hash collisions.** 256-bit [[BLAKE3]]; the probability is not a real number anyone needs to think about ([[Hashing and Identity]]).
- **Database corruption.** Content addressing makes the store untrusted and re-verifiable ([[Trusted Computing Base]]).
- **"Someone will disagree with the schema."** [[Start Here]] pre-empts this correctly: migration is a re-elaboration from core terms, not a text rewrite.
- **Reinventing the backend.** Explicitly not being done.

## The strongest argument for proceeding

Strip out everything cross-language and everything proof-related, and what remains is: *a content-addressed, queryable store of code with exact dependency tracking and exact test/compilation invalidation.* That artifact is independently valuable, is reachable by [[Roadmap|M3]], and is what makes the riskier tiers affordable to attempt. The project should be structured so that failure at M6 is a disappointment rather than a write-off.

## Related

- [[Roadmap]]
- [[Trusted Computing Base]]
- [[Cross-Language Semantic Hazards]]
- [[Equivalence and Witnesses]]
- [[Compilation as Query]]
