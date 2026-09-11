# Unison Abilities

Abilities are Unison's mechanism for tracking and handling side effects — a form of algebraic effects (closely related to effect handlers in languages like Koka or Eff, and to monadic effect systems in Haskell). A function's type declares which abilities it requires (e.g. the ability to throw an exception, do I/O, or access mutable state), and an **ability handler** provides a concrete interpretation of those effects when the function is run, similar to how an exception handler catches a thrown exception but generalized to arbitrary effects.

This matters for a multi-language unification project because effects are exactly the kind of thing that's hard to unify across languages purely syntactically — a Julia mutation and a Rust `&mut` borrow and a C++ pointer write are different mechanisms for what may be the "same" effect. An explicit effect/ability layer is one way to annotate or normalize that across languages, rather than leaving it implicit in each language's own semantics.

## Related

- [[Unison]]
- [[Content-Addressed Code]]
