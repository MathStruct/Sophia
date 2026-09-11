# witness

Witness formats and their checkers. Source: `witness.rs` (comments only).

Full design: [[Equivalence and Witnesses]].

## What a witness does

It turns a *claim* into a *licence*. The checkers are what make the difference between a database of assertions and a database of knowledge, and they are in the [[Trusted Computing Base]] — which is why each format gets its own small, separate, auditable checker rather than one monolithic verifier.

## Six formats, in descending strength

`Kernel` (a `Prf`-fragment proof term), `RewriteChain`, `Smt`, `TranslationValidation`, `TestEvidence`, `Attestation`. The design notes on each are in the source; three points are worth pulling out here.

**`RewriteChain` is the workhorse.** [[egraph|E-graph]] extraction produces it for free, its soundness reduces to the soundness of the cited rules, and no human is involved. If this project ever has a large number of real witnesses, they will be of this kind.

**`Smt` shifts trust to the encoder and the proof checker, not the solver.** That is the point of taking a certificate rather than a yes/no answer: solvers are large and buggy, proof checkers are small.

**`max_level` is the honest bit.** `TestEvidence` can never justify more than `Tested`, no matter how many tests passed. Encoding that in the trait rather than in a convention means a checker cannot grant a level it is incapable of supporting.

## Answering "nobody will write the proofs"

That objection ([[Open Problems and Risks]]) is correct about *human* proofs. Three sources need no human: rewrite chains from the e-graph, translation-validation reports from Alive2-style checking of each lowering, and SMT certificates from a Souper-style harvester over stored core terms ([[State of the Art - Superoptimization and Synthesis]], [[State of the Art - Program Equivalence Checking]]).

The realistic steady state is: mostly evidence, some machine-generated proofs, very few human ones. Still better than the status quo, in which the claims are not recorded at all and therefore cannot be audited, revoked, or improved later.

## Revocation

An attestation can turn out to be false. Nothing is ever deleted, so revocation is a new node plus a query-time filter — and everything built while trusting the withdrawn witness must be *findable*, which is what the trust-audit query in [[Query Cookbook]] is for.

## Adversarial notes

These checkers parse untrusted input from a shared store. Fuzz them, bound their resource use, and make sure a panicking checker is treated as **rejected** rather than **unverified** — the difference between those two failure modes is the difference between a crash and a wrong binary.

## Related

- [[Equivalence and Witnesses]] · [[sophia_equiv]] · [[egraph]]
- [[Trusted Computing Base]] · [[Query Cookbook]]
- [[State of the Art - Program Equivalence Checking]] · [[State of the Art - Proof-Carrying Code and Verified Compilation]]
