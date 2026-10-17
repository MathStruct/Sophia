#open-problem

> **Status: open (2026-10-17).** Related: [[SPEC-001 Base Sophia v0]] §4.2, §6, §9.

**Problem.** Docs are annotations on a definition or a term path, so they are not hashed. But a comment written *between two lines* of a body has no term of its own to attach to. If the parser drops it, the next print loses it. That breaks the expectation, from Moonkale ADR-0028 fact 2, that "comments are part of what people wrote".

**Options.**
1. Attach each comment to the nearest following term path as a `comment` annotation, and have the printer put it back before that term.
2. Allow a `Comment(text, Term)` wrapper in the core that is skipped when hashing. This mixes non-hashed data into the hashed tree.
3. Forbid inline comments in v0, and allow only docs on definitions.

**Leaning.** Option 1. It keeps the hash clean, and the source map (SPEC-001 §6.1) already relates text spans to term paths. An edit that removes the term marks the comment as orphaned. It is never silently dropped.
