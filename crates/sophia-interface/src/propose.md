#implementation

> An outside edit of a generated sophialang file, reported as a proposal without writing (SPEC-010 §5). Source: `propose.rs`.
>
> Sources: [[SPEC-010 Store-Truth Folders and Generated Files]] §4, §5, §6.2, acceptance 3. code: `propose.rs`, `edit.rs` (`normalise_items`, the refusals of `apply`), `sophia_interface.rs` (`export`)

## `propose_text(namespace, text, token) -> TextProposal`

Moonkale generated `text` from `Interface::export(namespace)` and kept the export token as the file's marker. When the file's text no longer hashes to the marker's source id, someone edited it outside Moonkale, and Moonkale asks what the edit would do.

1. A derived store refuses with `StoreError::Derived`, as `propose_files` does.
2. The namespace is exported again, and the target of every name it covers (printed names and aliases) is read at the same log position. A token other than the current one is a conflict: `(namespace, given, current)` in `proposal.conflicts`.
3. The text is parsed in the namespace with Sophia's parser. Errors go to `diagnostics`, with no file in the proposal. Warnings go there too.
4. The refusals of `apply` (`edit.rs`) are checked, with the file's names as the group that may be updated: an invalid path (`InvalidPath`), a laid-out name or a rule on a laid-out relation (`LaidOut`), a name in the namespace that the file does not hold, such as a query (`NameClaimed`, owner empty), and a relation declared over a content name (`NameExists`). The first one found is `proposal.error`.
5. The items are normalised as one batch by `normalise_items`, the function `apply` uses, so the proposed ids are the ids an edit stores. A relation the text introduces gets a provisional id, because the real one is random (`Store::new_nominal`). Records that refer to a new relation will get other ids when the edit is made.
6. One `FileProposal` with `path` = the namespace: `text_changed` (the text differs from the current export), names added, retargeted (old and new target) and removed (exported names the text no longer defines, aliases excepted), and memberships: a relation's signature replaces its old one, and a rule joins its head's relation and replaces its own old version. `report` counts these, plus the records not yet stored.

`TextProposal::changes_nothing()` holds when there is no conflict, no error, no error diagnostic and no name or membership change. It is acceptance 3's check.

## What it does not do

- **There is no accept yet.** SPEC-010 §5 makes accept `replace_files_checked` for foreign files. A sophialang file has no manifest (§6.2), and `write_text` edits one name and its group. An accept for a whole file is specified in SPEC-010 §8 and not built (B-060). Daniel decided on 2026-11-18 (SR-155) that it removes the names the text dropped.
- `spans_moved` stays empty: the parser's items carry no spans.
- `generics_added` and `generics_removed` stay empty: sophialang relations are nominal entities, not generic functions made from files. A new relation appears as an added name.
- A rule whose name the text drops is reported as a removed name, but not as a membership removed from its relation.
- The proposal takes no lock. The store may change between the proposal and an accept; the accept must check the token again.

## Tests

In `crates/sophia-interface/tests/acceptance.rs`, on all four backends:

- `an_exported_file_proposes_back`: acceptance 3 (the export proposed back changes nothing). An edit, a removal and an addition are reported, and nothing is written (the log position is unchanged). The proposed ids equal the ids `write_text` and `create_text` then store. A stale token is a conflict and the current one is not. A parse error gives diagnostics and no file. The round trip failed before `export` listed named relation signatures as aliases ([[sophia_interface]]).
- `a_proposal_refuses_names_outside_the_file`: a query in the namespace is not in the export, and a text defining its name is refused with `NameClaimed`.
