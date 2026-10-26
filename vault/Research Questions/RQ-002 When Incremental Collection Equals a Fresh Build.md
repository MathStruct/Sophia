#open-problem

> **Status: open (2026-10-25); the one known counterexample is refused since 2026-10-26.** Area: garbage collection, incremental computation · Prerequisites: reference counting, reachability, strongly connected components · Related: [[folder]], [[Folder Store Garbage]], [[Garbage Collection]], [[SPEC-009 Moonkale Folder Store]], SR-146.
>
> Sources: Bacon, Cheng & Rajan, *A Unified Theory of Garbage Collection*, OOPSLA 2004 (tracing and reference counting as duals); the question is original to this vault. code: `crates/sophia-store/src/folder.rs` (`replace_files`, `collect`, `collect_folder`), `crates/sophia-store/tests/folder.rs`.

## Question

A folder store in derived mode is a cache of a folder's files. Each batch replaces some files, then
removes what the batch's old file manifests reached and nothing reaches any more. The intended
invariant is that **after every batch, the store holds exactly the records a store built fresh from
the current files would hold.** Under which conditions on the files does this hold for every history,
and can it be proved rather than tested?

## Setting

- A **file** $F$ provides records $R(F)$, with names, generic-function members and spans, each pointing
  at records in $R(F)$. Its **manifest** is a record that references the named records, the members,
  the spanned records and the source text.
- A store's **roots** are the current manifests, the targets of names, and the current members of
  generic functions. A record is **reachable** if a root reaches it along references.
- A **batch** replaces a set of files. The collector starts from the records the replaced manifests
  referenced, together with the manifests themselves. It removes a strongly connected component when
  none of its members is a root and no record outside it references one of them. Removed records'
  references are considered next.
- $\mathrm{fresh}(T)$ is the record set after one batch writes the tree $T$ into an empty store.

## What is known

- **Tested:** `files_truth_collects_what_files_no_longer_reach`, a six-commit history with edits, a
  move, a replaced method, a two-member component and deletions, on all four backends. After every
  commit the record set equals the fresh build's. Further tests cover membership counting and names
  that change kind (P-231). `cargo test -p sophia-store --test folder`.
- **Measured:** Moonkale's replay of its own history (471 commits) and of Lenticulum.jl's (130),
  with collection: 0 differences from a fresh build at every checkpoint ([[Folder Store Garbage]]).
- **Counterexample (2026-10-25, a scratch test, not kept):** a file that provides a record with no
  name, no span, no generic membership and no record referencing it. The fresh build stores the record.
  When the file is replaced without it, the record is never a seed, because the manifest did not
  reference it. So it stays, and the store differs from the fresh build. The test appended to
  `tests/folder.rs`:
  `a.records.push(rec("hidden", …)); replay_records(&s, &fresh, vec![vec![a], vec![file("a.jl", &[("a", "1")])]])`
  fails after commit 1. A fresh build is not stable under `collect_folder` either: the sweep would
  remove the record.
- **An earlier version was wrong too:** Codex's review found that letting facts keep records broke
  the invariant ([[folder]], review triage). Facts no longer keep records.
- **Expected:** with the hypothesis "every record a file provides is reachable from its manifest",
  the invariant holds. Cross-component references form a DAG, so reference counting by components
  misses no garbage cycle.

## What would count as an answer

A proof of the invariant under stated hypotheses, by induction on batches, with the root sets
(names, memberships) included and not only the records. Or a further counterexample that satisfies
the hypotheses. The proof should name what each index (`dependants`, member counts, `defined-in`)
must satisfy for the induction step.

- **The counterexample is refused (2026-10-26, Daniel's choice):** `replace_files` rejects a file with
  a record its names, spans and generic memberships do not reach, directly or through references
  (`StoreError::UnreachedRecord`, test `unreached_records_are_refused`). So the hypothesis above is
  now enforced, not assumed. A manifest that references every record is the alternative, kept for
  later; it would need a new manifest version.

## Why it matters here

SR-146 makes the files the truth in derived mode. If the store can diverge from its files, Moonkale
sees definitions that no file has.

## Leads

- An invariant of the form "store = fresh(T) ∪ G, where G is unreachable and contained in the
  closure of the seeds", proved per batch.
- Model checking small histories (two files, three records, all operations) against the fresh build,
  as a property test.
