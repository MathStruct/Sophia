# Bisimulation

A co-inductive notion of equivalence for state machines and, by extension, for stateful programs. A relation `R` between states is a **bisimulation** if whenever `s R t`:

- every step `s → s'` can be matched by some `t → t'` with `s' R t'`, and
- symmetrically for steps of `t`.

Bisimilarity is the union of all bisimulations — the largest one. Proving two systems bisimilar means *exhibiting* a relation and checking the two matching conditions, which is a finite obligation even for infinite-state systems. That "exhibit a witness, check it locally" shape makes bisimulations well suited to being stored as [[Equivalence and Witnesses|witness nodes]]: the relation is the evidence, and the checker replays the matching conditions.

Variants that matter for real languages:

- **Weak bisimulation** ignores internal (τ) steps, so an implementation may take more steps than a specification. This is what licenses "same behaviour, different performance" — closely related to the `timing` modulo tag.
- **Applicative** and **environmental bisimulation** adapt the idea to higher-order languages, where "a step" must account for functions being passed around.
- **Bisimulation up-to** techniques shrink the relation that must be exhibited, which is what makes the method practical.

Bisimulation and [[Logical Relations]] are the two standard routes to [[Contextual Equivalence]]; the rough division of labour is that logical relations suit types and pure higher-order code, while bisimulation suits state, concurrency and effects. A system covering both Julia and C++ would need both.

## Related

- [[Contextual Equivalence]]
- [[Logical Relations]]
- [[Operational Semantics]]
- [[Equivalence and Witnesses]]
