#open-problem

> **Status: open, later (2026-10-17).** Related: [[S-ADR-0003 Identity is content, names and lineage]].

**Problem.** A lineage is a growing set of contents linked by `Supersedes`. Once namespaces have branches (SPEC-001 §5.2), two branches can extend the same lineage differently, and a merge has to decide what the lineage's "current" content is. A copy that is then edited (`ForkedFrom`) raises the same question.

**For v0.** One branch, so there is no conflict. A lineage's "current" content is whatever its names point at. If a lineage has no names, it has no current content, only history.
