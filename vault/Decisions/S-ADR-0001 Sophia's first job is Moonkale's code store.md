#design

> **Status: proposed (2026-10-17, Claude), from Daniel's message of 2026-10-17.** Related: [[SPEC-001 Base Sophia v0]], [[Requirements]] SR-18, SR-21; Moonkale ADR-0028 §3, WP-68.

## Context

The earlier vault presents Sophia as a multi-language compiler whose headline is cross-language interoperability proved by equivalence edges. Its roadmap starts with Metamath (M0) and a Lisp (M1), and puts Julia precompilation (M3) at the point "where the project justifies itself".

Daniel's statement of 2026-10-17 gives a different reason to continue. Moonkale needs a specification, and designing Moonkale against a Unison stand-in is hard. So Sophia should make a real first attempt at the database compiler. Moonkale's ADR-0028 deferred "where Sophia's definitions live" until real definitions exist, and named a base Sophia (WP-68) as the way to get some.

## Decision

Sophia's first goal is to be **the store-backed code source that Moonkale edits**:
- definitions in a database;
- content identity;
- names;
- printing and parsing;
- a change feed;
- queries.

The compiler, the checkers and interoperability are layers that come later, hung off the same identities. Their structure is fixed in v0 only far enough not to block them.

## Consequences

- The earlier roadmap's M0 to M7 are no longer the plan. They remain a list of ideas ([[Reading of the Earlier Vault]]).
- Sophia's v0 language is chosen for what Moonkale has to show (functions, data types, relations, rules, holes, tests), not for proving the plumbing on Metamath.
- Moonkale WP-18 (a Unison source) **stays**. Daniel, 2026-10-17: keep both Unison and Sophia (SR-23). Two independent store-backed sources are a stronger test of Moonkale's schema than either one alone.
