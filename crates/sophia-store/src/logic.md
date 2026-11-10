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

**Batches** (2026-10-20): `add_members` adds many members to one nominal entity in a single transaction, checking membership once. `add_method` checks it per call, which was quadratic for set.mm's 48 000 proofs. `annotate_many` adds many annotations in one transaction.

**`record_facts(facts)`** (2026-10-21): several facts in one transaction, skipping (and not logging) facts already stored. Ingests use it for `source-span` facts: one transaction for a package instead of one per definition.

**`verify` gives a verdict, and incomplete components are refused** (Gemini's review, 2026-11-07 ([[Review 2026-11-07 Gemini (external)]]); fixed 2026-11-08). `verify` returns a `Verification`:
- `Valid`: the component is complete, and rehashing it gives the id.
- `Invalid`: the component is incomplete, or rehashing it gives another id.
- `Foreign { normaliser }`: the component is complete, but its ids come from another tool's normal form (`moonkale-wp66@…`, or `moonkale-wp66-…@…`, which is how Moonkale's folder store writes `moonkale-wp66-imports-v1@8`; SPEC-009 §1; until the 2026-11-08 Mistral review any `moonkale-` prefix counted, so a record could skip re-hashing by its name alone), which Sophia cannot recompute. It used to say `false` for every Moonkale record, a lone definition included.

`check_components` runs at the end of `put_records`, `add`, `update` and `replace_files`. It refuses with `IncompleteComponent` a component whose stored members are not exactly indices 0 to `component_size` − 1. Every size a record of the write claims is checked, so two records of one component that disagree on its size are refused (Mistral's review, 2026-11-08; `tests/symmetric.rs`). Moonkale's normaliser before `06fd112c` gave symmetric members one id but kept the full size, so a 2-member component was stored with 1 member. Tests: `tests/symmetric.rs` on all four backends fails without the check, and `sophia-ingest`'s `moonkale_records_verify_as_foreign`.

**Fixes from Fable's review (2026-11-10, [[Review 2026-11-10 Fable (external)]]):**
- **`changes_since`** with `limit = usize::MAX` overflowed `limit + 1` (S-2); it now saturates.
- **`set_settings`** accepts only keys under `SETTINGS_PREFIXES` (`profile/`, `reader/`, `ns/`, `settings-version/`), and refuses others with `NotASetting` (S-7). It used to accept any key, so `store/mode` could make the store derived.
- **`set_settings_if`** writes settings only if a version counter still reads the expected value, in one transaction (S-14).

Tests: `settings_are_listed_and_logged` on all four backends. It fails without each of the three.
