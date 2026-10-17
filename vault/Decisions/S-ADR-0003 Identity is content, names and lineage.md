#design

> **Status: proposed (2026-10-17, Claude).** Answers Moonkale question 1b. Related: [[SPEC-001 Base Sophia v0]] §5, [[Theory of the v0 Store]].

## Context

In Unison a definition *is* its hash, and a name is a label that moves. That is enough for a language whose editing happens in scratch files. Moonkale also needs something that stays put while you edit, so that a comment thread, an open tab or a review stays attached to "this function". For file-backed code, the location id plays that role. Store-backed code has no location.

## Options

1. **Content id only, as in Unison.** Tabs and comments follow names. A rename breaks a tab; an edit breaks a comment, which stays on the old content.
2. **A mutable slot id**: a record whose current content changes. This is simple to use, but it brings back mutable identity, and it is unclear what a slot means after a copy or a merge.
3. **Content id plus name plus lineage.**
   - Content ids are immutable.
   - Names are movable labels.
   - A lineage is the growing set of contents that were produced by editing one another, linked by `Supersedes`.

## Decision

Option 3. The lineage is metadata: it never enters a hash, and it may be many-to-many (forks, and two edits that reach the same content).

## Consequences

- Moonkale attaches comments and history views to the lineage, tabs to the name or the lineage, and proofs to the content id.
- `WriteText` through a name extends the lineage of the definition the name pointed at.
- A merge of two branches (later) has to merge lineages. That is a known later problem.
