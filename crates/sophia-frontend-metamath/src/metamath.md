# metamath

The Metamath frontend — the [[Roadmap|M0]] target. Source: `metamath.rs` (comments only).

## Why start with a language unlike the target

Metamath has no types to infer, no effects, no memory model, no dispatch and no optimisation. It is nothing like Julia, and choosing it first is not a step toward the real goal in any semantic sense.

It is chosen because it **isolates the plumbing from the semantics**. M0 asks only: does canonicalisation terminate and produce stable digests, does the [[Cycle Hashing|SCC construction]] survive real dependency structure, does the store round-trip, and does re-ingestion write zero new rows. All of those are load-bearing, and none of them requires a single hard semantic decision. Answering them against a language with type inference would confound two unrelated risks.

`set.mm` is a good corpus for it: ~40k theorems, deep dependency structure, and an existing verifier to check against.

## A second reason: Metamath already has the shape

A theorem is a `Decl`, its statement is a `Prop`, its proof is a `Witness`. Metamath is a working, 40k-theorem example of exactly the knowledge-layer structure [[Graph Schema]] proposes, which makes it a useful sanity check on the schema itself and not only on the hashing.

Labels map to `Name` bindings and are deliberately **not** part of the hash — the first concrete exercise of [[Naming and Change Propagation]].

## Shallow embedding, stated as such

Two options: represent Metamath's own objects as SC data (shallow), or interpret its logic in SC's `Prf` fragment (deep). M0 should do the shallow one, and should say so. The trap otherwise is that a "first frontend" built for plumbing quietly becomes the template for the real ones, carrying its shortcuts along.

## Acceptance tests

Round-trip `set.mm` byte-identically; ingest twice and write zero new nodes the second time; match digests across two machines; keep every proof verifiable by an external checker; and report throughput, store size and fetch latency.

**Test two is the one that matters.** If re-ingestion writes new nodes, canonicalisation is nondeterministic and nothing else in the project is worth building until that is fixed. See [[canonical]].

## What M0 deliberately does not test

Types, effects, dispatch, lowering, optimisation, equivalence, or generated-code performance. Those arrive with the Lisp frontend at M1 and with Julia at M3.

## Related

- [[Roadmap]] · [[Graph Schema]] · [[canonical]] · [[Cycle Hashing]]
- [[Proof Assistant]] · [[State of the Art - Proof-Carrying Code and Verified Compilation]]
