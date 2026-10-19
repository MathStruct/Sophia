#implementation

> Crate root of `sophialang-core`: the first core calculus of sophialang, and its normaliser. Source: `sophialang_core.rs`.
>
> Sources: [[SPEC-003 Sophialang Core v0]]; [[SPEC-002 Sophialang Principles]]. code: `sophialang_core.rs`, [[term]], [[definition]], [[encode]], [[normalise]], [[samples]]
>
> Theory: [[Type Taxonomy]] · [[Computational Trinitarianism]].

The crate turns sophialang definitions into `sophia_hash::Record`s and back. `definition_of(record)` decodes a stored record and restores its display names.

The crate depends on `sophia-hash` only, and not on the store (S-ADR-0007): the store keeps records of any language.

**Tests** (`tests/identity.rs`, 13), the M0 acceptance of [[Milestones]]:
- determinism;
- names never change ids, also as a property test over random terms;
- the dependency rule (changing `Nat` changes `add` and its test, but not `double`);
- mutual recursion: a shared component, and independence of the input order;
- holes and unresolved names are hashed and incomplete;
- usages, `Unsafe` marks and declared types are hashed;
- decode then rehash gives the same id;
- encode/decode round trips on random terms;
- decoding random bytes never panics.

Two sample relations with the same type and no body (`parent`, `ancestor`) are **one** definition. That is the correct consequence of content addressing, but it surprised the test author. Relations probably need a nominal identity, like structs → to be decided before relations get rules that run.
