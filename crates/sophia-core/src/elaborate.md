# elaborate

Surface syntax → Sophia Core, plus the normaliser. Source: `elaborate.rs` (comments only).

## Where the difficulty is deliberately concentrated

The architecture's central simplification is that there is **one** semantics — [[Core Calculus|SC]]'s — rather than one per language. Cross-language equivalence then reduces to comparing core terms instead of relating two unrelated denotational universes ([[Equivalence and Witnesses]]).

The cost of that simplification is paid entirely here. Each frontend's `elaborate` *is* the specification of what its language subset means, it is unverified, and no one has a formal Julia or C++ semantics to check it against ([[State of the Art - Formal Semantics of Real Languages]]). Differential testing — run the original, run the extracted core term, compare — is the only realistic validation.

## What must be made explicit

The hash is computed on the elaborated term, so anything left implicit is a difference that will silently vanish:

- implicit and instance arguments; trait / type-class resolution
- coercions and numeric promotions (C++'s usual arithmetic conversions, Julia's `promote`) — always an explicit `Convert` node
- overflow discipline and float flags on every arithmetic primitive
- bounds checks, or an explicit assumption node discharging them
- effect rows ([[effects]])
- index-base and layout arithmetic — Julia's 1-based column-major and C's 0-based row-major both become explicit linear-index computation, or comparing the two loops is meaningless

Every item here appears in [[Cross-Language Semantic Hazards]]; this file is where the hazard list turns into code.

## Normalisation by evaluation

`eval`/`quote` implement [[Normalization by Evaluation]], serving two consumers that **must not diverge**: the kernel's `convert`, and step 4 of [[canonical]]. The usual implementation trap — `quote` produces de Bruijn *levels*, the term representation wants *indices* — is noted in the source.

## The property test to write first

Frontends are supposed to be [[Functor|functors]] ([[Multi-AST Layering]]):

```
elaborate(f ∘ g) ≡ elaborate(f) ∘ elaborate(g)
```

Cheap to test with generated inputs, and it catches the most damaging frontend bug class: a special case in the parser for a composite form, which produces hashes that do not compose and quietly breaks every downstream equivalence.

## Bounded subsets, enforced

No frontend will cover its whole language. The subset boundary must be explicit and ingestion outside it must **fail rather than guess** — a term whose meaning was guessed is worse than one never ingested, because it will be hashed, stored, trusted and compiled.

## Related

- [[Core Calculus]] · [[sophia_core]] · [[effects]]
- [[Normalization by Evaluation]] · [[Multi-AST Layering]]
- [[Cross-Language Semantic Hazards]] · [[State of the Art - Formal Semantics of Real Languages]]
