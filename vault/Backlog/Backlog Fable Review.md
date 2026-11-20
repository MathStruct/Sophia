#planning

> Open backlog tasks: the open findings of Fable's review. Part of [[Backlog]], whose rules (rows, picking, closing) apply.

> Sources: [[Backlog]], [[Status]].

## Open findings of Fable's review

| id | task | status | needs | size | source | done when |
|---|---|---|---|---|---|---|
| B-007 | S-1: verify sophialang records on write (one hash per component); Moonkale records stay `Foreign` | proposed | Daniel | S | Fable S-1 / B-1 | a record whose bytes do not match its id is refused, tested on four backends; cost of the check measured on bulk ingest (B-03) |
| B-008 | S-4: a name bound twice in one file: refuse, or keep the last as Julia does | proposed | Daniel | S | Fable S-4 | Daniel's choice implemented and tested in `folder.rs` |
| B-009 | S-5: `path_from_moonkale` maps `a.b/c` and `a/b/c` to one path: escape `.` in segments, or forbid it | proposed | Daniel, Moonkale | S | Fable S-5 | the mapping is injective and round-trips, with a property test |
| B-010 | SP-008: an edit in one transaction, with the review's acceptance test (a backend that fails the n-th write) | proposed | Daniel | M | [[SP-008 Edits span several transactions]]; Fable | the failing-backend test passes for every n on four backends |
| B-013 | The cost points I-1 to I-9: measure the ones a benchmark can show, change nothing unmeasured | proposed | — | M | Fable 03; SR-126 | each point measured or marked "not measurable", in [[Benchmark Results]] |
