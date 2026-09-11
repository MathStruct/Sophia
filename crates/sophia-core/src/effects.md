# effects

Effect rows, regions and borrows. Source: `effects.rs` (comments only).

Full design: [[Effects Memory and Resources]].

## Why this is not an optional extra

Effects are what an [[Equivalence and Witnesses|equivalence claim]] is about. [[Contextual Equivalence]] is defined relative to an *observation*, and effects are the observations. Two functions with different effect rows are not equivalent, and a system that cannot express the difference will assert that they are.

Purity is also what licenses the valuable rewrites — CSE, reordering, memoisation, parallelisation. The effect row is the certificate that makes them sound, so [[sophia_equiv]] cannot do useful work without this module.

## Representation notes

`BTreeSet` rather than `HashSet` for the labels: the row is hashed, so its serialisation must be canonical — the same reasoning as step 5 of [[canonical]].

`Unsafe` is a bottom element that poisons anything containing it. That is deliberate: it makes "we do not model this" visible in the type rather than silently absent, which is the honest treatment of `ccall`, inline assembly and C++ undefined behaviour.

## One device for four memory models

Regions plus a borrow discipline subsume the GC heap, reference counting, arenas and Rust lifetimes — see the table in the source. The consequence worth repeating:

**Rust's model forbids programs Julia allows, so Julia → Rust is refinement, not equivalence.** Emitting an `EQUIV` edge where `REFINES` is correct is a concrete, plausible route to an aliasing miscompilation, and it is exactly the kind of error this file exists to make impossible to express. See [[Linear and Affine Types]].

## The bridge to the equivalence layer

`observable(row, modulo)` projects out the effects a claim ignores. An equivalence "modulo `alloc`" *is* a claim about rows with `Alloc` removed. Defining the projection here rather than in the checker means there is one definition of what each modulo tag means — otherwise the tag is just a string and the `modulo` discipline in [[Equivalence and Witnesses]] is decorative.

## Start coarse

A full region-and-borrow system may not earn its complexity in the first cut. Five labels — `Pure`, `Alloc`, `Mut`, `Io`, `Unsafe` — with regions deferred, is the suggested starting point. The structure in the source is the target.

## Related

- [[Effects Memory and Resources]] · [[Effect System]] · [[Unison Abilities]]
- [[Linear and Affine Types]] · [[Contextual Equivalence]]
- [[sophia_core]] · [[sophia_equiv]]
