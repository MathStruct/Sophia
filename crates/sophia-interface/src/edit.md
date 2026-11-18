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

Steps 3 and 4 (renumbering the parsed items and normalising them as one batch) are `normalise_items`, shared with `propose_text` ([[propose]]) so that a proposal computes the ids an edit stores.

## The rest

- **Store-truth folders (2026-10-28, SPEC-010 §2):** `write_text`, `rename` and `delete` on a name that a file lays out (`Store::laid_out`) return `EditError::Store(StoreError::LaidOut { path, token })`. Moonkale edits it through the file. In a store without files no name is laid out, so nothing changes there. **Every name an edit would update is checked** (Codex review, 2026-10-28): the text of a recursive group may change the group's other names too, and one of those may be laid out though the edited name is not. `apply` refuses before writing anything. A generic function's name is laid out by its contributing files ([[folder]]). Tested by `generic_and_group_names_route_through_files`, which fails without the check. **A rule whose head is a laid-out generic function is refused too** (second Codex review, 2026-10-29): it would add a method to that generic function. The same test creates such a rule and fails without the check.
- **A moved reference warns on save (SR-150, Daniel's option C, 2026-10-29).** The edit token covers the name, its target and its group's names, not the names the text refers to. If `sample.helper` moved to other content after `sample.user` was last saved, saving `user`'s unchanged text takes the new `helper`. That is what the text says now, so it is not refused. Instead `Written.warnings` names `helper`. `moved_references` reads the log since `user`'s version, finds names that pointed at one of the old definition's references and were moved or removed, and warns for each that now points at one of the new definition's references. A reference the user typed newly does not warn. The cost is one log read since the name's last save, per updated name whose references changed; unmeasured. Tested by `a_moved_reference_warns_on_save`, which fails without the warning.
- **`create_text`, `create_dir`, `rename`, `delete`:** as SPEC-004 §3. An empty namespace is a meta row `ns/<path>`. Deleting a namespace removes its own row and every `ns/<path>.…` row below it, so no empty sub-namespace outlives its parent; it is refused while a name lies strictly inside it, and a name whose path equals the namespace's does not count (Fable S-9, 2026-11-18; tested by `deleting_a_namespace_removes_sub_namespaces`, which fails without either half). The root namespace is never deleted (Codex's review of the fix: in a store without names it "succeeded" and kept every marker; `the_root_namespace_is_not_deleted` fails without the check). The name check, the marker scan and the write are separate transactions, so a name or sub-namespace created concurrently can escape the deletion (Codex's review; the SP-008 class, B-010).
- **The settings node:** writing it replaces the reader's personal profile `reader-<id>` (`based_on` plus overrides), points the reader's default at it, and bumps the reader's settings version. It is one logged transaction, so Moonkale sees `reprint_all`.
- **`load_sample`:** the 16 sample definitions, the two relations as nominal entities with their members, and the built-ins.

**Queries as text** (2026-10-20): `create_text` with text whose first line starts with `query` or `select` stores a query under the new name (`sophia_query::save`). `write_text` on a name that points at a query re-parses it, checks the edit token, and moves the name with `Store::update`. Query parse errors become diagnostics with byte spans.

**`write_settings`** checks the reader's settings version in the same transaction as the write, through `Store::set_settings_if` (Fable's review, 2026-11-10, [[Review 2026-11-10 Fable (external)]], S-14). Before, two writers could both pass the check and write the same version.
