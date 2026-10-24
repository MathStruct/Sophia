#overview #open-problem

> Problems found while specifying or building Sophia, numbered `SP-nnn` (the `S` keeps them apart from Moonkale's `P-nnn`). Each problem gets a note made from [[Problem Template]] and a line here. Closed problems stay listed, with what closed them.

| # | Problem | Status |
|---|---|---|
| [[SP-001 Comments inside a definition body]] | The core has no place for an inline comment, so printing back loses comments written inside a body | open |
| [[SP-002 Propagating an update to dependants]] | Editing `f` leaves its dependants on the old `f` | open, later |
| [[SP-003 Display names of alpha-equivalent definitions]] | Two definitions that differ only in local names are one definition, with one set of names | accepted wart |
| [[SP-004 Moonkale operations are file-shaped]] | Annotations and structural edits have no `Op` yet | open, later |
| [[SP-005 Lineages under merge and fork]] | Many-to-many lineages need merge rules once branches exist | open, later |
| [[SP-006 Open method tables versus content addressing]] | Multiple dispatch's open method tables against hash identity: generic functions become nominal, and dispatch becomes derived facts | open, proposal |
| [[SP-007 Index base needs an Index type]] | Showing 0- or 1-based indexing needs to know which integers are positions | open |
| [[SP-008 Edits span several transactions]] | A text edit is several store transactions, not one | open |
| [[SP-009 Canonical form of queries]] | Literal order inside a query body is part of its id | open, accepted for v0 |
| [[SP-010 Query snapshots are coarse]] | Any logged change invalidates every cached query result | open |
| [[SP-011 Reference edges dominate storage on real code]] | On set.mm, 32-byte reference edges take 42 % of the store | open |
| [[SP-012 Record kinds must follow from content]] | A kind that can differ between users of the same content is a role, not a kind | enforced (2026-10-20) |
| [[SP-013 Julia bindings of modules Sophia has not ingested]] | Which function a method extends depends on Base's bindings (`Base.Matrix` is `Core.Array`); 86 wrong owners in LinearAlgebra | open, proposal: a binding table from Julia as an environment artifact |
| [[SP-014 The query result cache is never evicted]] | Cached query results accumulate in `t_meta` forever | open, proposal: in-memory by default |
| [[SP-015 Stored definitions deeper than the parse limit cannot be edited]] | 7 % of `sophia gen` definitions print but do not parse back (depth > 128) | open; bound depth on entry and cap the generator: approved 2026-10-24 |
