#design

> How Sophia frees space: reachability for persisted objects, eviction for caches, compaction for the log. Specification only; nothing is collected today (the store only grows). Part of [[Storage and Materialization]]. 2026-10-20.
>
> Sources: Nix (`gcroots`, reachability over store-path references); Git (`git gc`, reflog expiry, packfiles); Unison (the codebase keeps everything a namespace reaches); [[Object Categories]]; [[Object Lifecycle]].

## Two collectors, never mixed

| | Persisted objects | Cached objects |
|---|---|---|
| Kept while | reachable from a root | the cache policy wants them |
| Removed by | a reachability sweep | eviction, at any time |
| Removing one can lose information | yes, so only unreachable ones go | no: provenance stays, the bytes are rebuilt on demand |

## Roots

- **names** (every current name's target);
- **lineages** (their current heads; history per retention);
- **nominal collections** (current members: method tables, library roles);
- **cited worlds** (every world a fact, artifact, test run or proof cites);
- **pins**: artifacts a release, a binary or a user pinned;
- **evidence cited by a root**: a proof or certificate an artifact's A includes (SPEC-006 §3);
- **the retained log window** (below).

## Reachability

From the roots, follow:
- a record's `refs` and `nominal_refs`;
- a nominal entity's members;
- an annotation's subjects, and from a subject to its annotations (a proof stays while its statement stays);
- a fact's subject and inputs;
- an artifact's C, world, and evidence;
- a source blob from the canonical forms derived from it.

Anything not reached is garbage. **Content addressing makes sharing free**: two packages that contain the same method keep one record, and it stays while either reaches it.

## The log

The log is the history, so it is authoritative, but it need not be kept at full resolution forever:
- **Retention window:** full entries for a period (or a number of sequences) that Moonkale's undo and `changes_since` need.
- **Compaction:** older entries collapse into a snapshot of the state at that point, as in Git's packfiles or a database checkpoint. Names and nominal states at the snapshot become roots of their own only if something cites that snapshot's world.
- **Ingest entries** collapse to one transformation record: "frontend F version v ingested source blob B" ([[Current Inventory]], point 4).

## Cache eviction

- Caches have a budget (bytes) per store.
- Eviction follows the [[Materialization Policy]]; until there is data, least-recently-used with a size weight.
- Snapshot-keyed entries whose snapshot is no longer current and no longer cited go first: they can never be hit by a query about the present.

## Safety

- A sweep runs against a fixed snapshot (one read transaction) and deletes in a write transaction that re-checks that nothing became reachable in between. The same race the M3a review found in the query cache applies here.
- Deleting is logged. A deleted object's id can reappear later (content addressing), which is not an error.
