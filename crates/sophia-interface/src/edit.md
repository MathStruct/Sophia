#implementation

> Edits through text. Source: `edit.rs`.
>
> Sources: [[SPEC-004 Moonkale Interface]] §3; [[S-ADR-0006 Edits through projections and the existing operations]]; [[S-ADR-0009 Relations are tracked, not content-addressed]]. code: `edit.rs`

## `write_text(name, expected, text)`

1. Parse the text in the name's namespace. An error stores nothing.
2. The text must define the edited name. It may also redefine other names of the same recursive group, and add new names. Redefining any other existing name is refused.
3. Check the version, then create nominal entities for new relations.
4. Normalise all items as one batch, so that recursive groups re-hash together.
5. Add new names and update existing ones. A save that changes nothing is a no-op.
6. Attach relation signatures (replacing the old one) and rules (by their head's relation; an edited rule replaces its old version) to their relations.
7. Report warnings and the definitions still using replaced content.

## The rest

- **`create_text`, `create_dir`, `rename`, `delete`:** as SPEC-004 §3. An empty namespace is a meta row `ns/<path>`.
- **The settings node:** writing it replaces the reader's personal profile `reader-<id>` (`based_on` plus overrides), points the reader's default at it, and bumps the reader's settings version. It is one logged transaction, so Moonkale sees `reprint_all`.
- **`load_sample`:** the 16 sample definitions, the two relations as nominal entities with their members, and the built-ins.
