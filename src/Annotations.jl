"""
    Sophia.Annotations

Macros for attaching non-compiled statements to code: the "annotations" of
`markdown/Start Here.md`.

Design notes: `Annotations.md`, `markdown/Design/Tests and Documentation as
Nodes.md`, `markdown/Design/Equivalence and Witnesses.md`.
COMMENTS ONLY — nothing here is implemented.

markdown/Start Here.md: "part of the statements in the graph database will not
be there for compilation, instead they prove equivalencies between codes […] or
correctness or solve memory management. Or just contain comments, markdown."
This module is the user-facing surface for all of that.

These have to be macros in Julia because they must expand in the user's own
session, next to the code they annotate, with access to the surrounding module.

# Intended macros

    @equiv f g                         # claim f ≈ g
    @equiv f g modulo=(:alloc, :fp_assoc)
    @equiv f g level=:tested witness=:(my_property_suite)

    @spec f  "sorted(result) && multiset(result) == multiset(input)"
    @spec f  begin ... end             # a Prf-fragment proposition

    @modulo (:timing,)                 # scoped default for a block

    @sophia_test f begin ... end       # a Test node attached to f, not a file

    @assume f  "indices are in bounds" # discharges an @inbounds promise,
                                       # attributed to whoever wrote it

    @doc_node f "markdown …"           # a Doc node, queryable, hash-attached

# Defaults that must be conservative

`@equiv` with no `level` defaults to `:asserted`, NOT to something stronger.
An asserted equivalence is:

  * signed and attributed (so it can be revoked)
  * NOT substitutable by the compiler without an explicit policy opt-in
  * a taint on the provenance of anything built using it

That is not pessimism. Asserted claims are not congruences: two sort functions
agreeing on every test may differ on stability, and a context that observes
stability distinguishes them. Making `@equiv` feel powerful by defaulting to
substitutable would be the single easiest way to turn this project into a
miscompilation generator.
See `markdown/Wiki/Semantics/Contextual Equivalence.md`.

# `@equiv` should refuse obviously false claims at macro time

Before recording anything, check what is cheaply checkable:

  * arity and type signatures compatible?
  * effect rows compatible, or is the difference covered by `modulo`?
  * do the hazards in `markdown/Design/Cross-Language Semantic Hazards.md` that
    apply to these two terms have corresponding `modulo` tags?

A claim missing a required `modulo` tag should be an ERROR with the specific
hazard named, not a warning. Most false equivalences will be honest mistakes
about overflow or floating point, and this is where they are cheapest to catch.

# Tests attached to hashes, not files

    @sophia_test f begin @test f([3,1,2]) == [1,2,3] end

The result is a fact about a hash: `(test, subject, target, seed) -> outcome`,
which is pure and therefore cacheable forever. Consequences:

  * a test never re-runs for an unchanged definition — not "unchanged file",
    unchanged DEFINITION, so reformatting or editing a neighbour costs nothing
  * test selection after a change is exact (a graph query), not heuristic
  * same inputs with different outcomes ⇒ the test is flaky, detected
    automatically rather than by folklore

See `markdown/Design/Tests and Documentation as Nodes.md`.

# Where this fits in the roadmap

`@sophia_test` and `@doc_node` are useful as soon as ingestion works (M3) and
need none of the equivalence machinery. `@equiv` and `@spec` only become
meaningful at M4–M5. Ship them in that order.
"""
module Annotations
end
