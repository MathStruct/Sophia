# Annotations

Macros for attaching non-compiled statements to code. Source: `Annotations.jl` (comments only).

This is the user-facing surface for what [[Start Here]] calls annotations: "part of the statements in the graph database will not be there for compilation, instead they prove equivalencies between codes […] or correctness or solve memory management. Or just contain comments, markdown."

They have to be macros because they must expand in the user's own session, beside the code they annotate, with access to the surrounding module.

## The default that matters most

`@equiv` with no `level` defaults to **`:asserted`** — attributed, revocable, and *not* substitutable by the compiler without an explicit policy opt-in, tainting the provenance of anything built with it.

That is not excessive caution. Asserted claims are not congruences: two sort functions agreeing on every test may differ on stability, and a context observing stability distinguishes them ([[Contextual Equivalence]], [[Equivalence and Witnesses]]). Making `@equiv` *feel* powerful by defaulting to substitutable is the single easiest way to turn this project into a miscompilation generator, and it is the kind of decision that is very hard to walk back once users depend on it.

## `@equiv` should refuse obviously false claims at macro time

Arity and signature compatibility, effect-row compatibility, and — most usefully — whether the [[Cross-Language Semantic Hazards|hazards]] applying to these two terms have matching `modulo` tags. A claim missing a required tag should be an **error naming the specific hazard**, not a warning.

Most false equivalences will be honest mistakes about integer overflow or floating-point reassociation. Macro expansion is where they are cheapest to catch, and a good error message here does more for correctness than any amount of downstream checking.

## Tests attached to hashes, not files

`@sophia_test` records a fact about a hash: `(test, subject, target, seed) → outcome`, which is pure and cacheable forever. A test never re-runs for an unchanged *definition* — not an unchanged file — so reformatting or editing a neighbouring function costs nothing. Test selection after a change becomes an exact graph query rather than a heuristic, and flakiness is detected automatically when the same inputs give different outcomes. See [[Tests and Documentation as Nodes]].

## Shipping order

`@sophia_test` and `@doc_node` are useful as soon as ingestion works ([[Roadmap|M3]]) and need none of the equivalence machinery. `@equiv` and `@spec` only mean anything at M4–M5. Building them in that order also means the conservative `@equiv` default arrives after users already have a reason to trust the store.

## Related

- [[Tests and Documentation as Nodes]] · [[Equivalence and Witnesses]]
- [[Contextual Equivalence]] · [[Cross-Language Semantic Hazards]]
- [[Frontend]] · [[Store]] · [[sophia_equiv]]
