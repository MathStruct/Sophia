#design

> **Status: accepted (2026-10-20, Daniel: "I think metamath and Lisp are easy cases you may try them first, but better do sophialang").** Related: SR-65, [[Milestones]], [[Reading of the Earlier Vault]] item 4.

## Decision

The first language in the store is sophialang's core (SPEC-003), not Metamath or a Lisp. A Lisp or Metamath frontend may come in M7, as a test of the store's language-agnosticism (S-ADR-0007), if it is useful then.

## Consequences

The earlier Roadmap's M0 (Metamath) and M1 (Lisp) are superseded. The store's genericity is tested first by Moonkale's own normal forms (M7), and by the Unison source on Moonkale's side (WP-18).
