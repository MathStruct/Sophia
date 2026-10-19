#implementation

> `Store<B>`: the store's rules over any backend. Source: `logic.rs`.
>
> Sources: [[SPEC-001 Sophia Store v0]] §5 to §8; [[S-ADR-0003 Identity is content, names and lineage]]; [[SP-006 Open method tables versus content addressing]]. code: `logic.rs`

## Rules implemented

- **Sequence numbers.** Every change appends one log entry, `seq = last + 1`, in the same transaction. A name's version is the `seq` of its last change.
- **`add`** stores the records that are new (one `add-definition` entry each) and adds names. A new name starts a lineage at its content if the content has none.
- **`update`** (an edit through a name) is optimistic: it fails with `Conflict` unless the name is still at the expected version. It is a **no-op** when the new content is what the name already points at. Otherwise it:
  - moves the name;
  - extends every lineage of the old content with a `supersedes` link;
  - reports the definitions that still reference the old content (SP-002).
- **`rename`**, **`remove`** and **`alias`** move, delete or add names. No id changes.
- **Nominal entities:**
  - `new_nominal` draws a fresh id (`fresh_nonce`: time, process, a counter and an address, hashed);
  - `ensure_nominal` registers built-ins idempotently;
  - `add_method` and `remove_method` change the relation and bump its `version`.
- **Facts are stale** when a relation they recorded has a newer version (`is_stale`). Facts about immutable content are otherwise never wrong.
- **World snapshots** (`snapshot_world`, `world_members`): a content-addressed snapshot of every nominal entity's member set, stored once under `world/<id>` (SPEC-006 §2). Nominal ids are indexed under `nominal-index/` so that every backend can enumerate them.
- **`changes_since`** pages through the log.
- **Path rules** (`validate_path`): dotted segments of letters, digits, `_` and `!`, not starting with a digit, or an operator in backquotes.

## Not yet

Garbage collection, branches, update propagation, the overlap check that would let a new method make only *affected* facts stale (SP-006; today any change to a relation makes all its facts stale), and migration.
