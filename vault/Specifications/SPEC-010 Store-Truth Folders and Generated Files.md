#design #definition

> **Draft, a proposal by Claude (Sophia), 2026-10-26. Accepted by Daniel on 2026-10-26, with questions 1 to 4 answered as the Moonkale agent recommended and question 5 answered with (b).** **Implemented 2026-10-28 (§2, §3, §4 export, §5 `propose_files`)** on all four backends ([[folder]], [[edit]], [[sophia_interface]]). Not built yet: `propose_text` for sophialang files, and the text export of SR-149. Three amendments from implementing it are listed in §6. It specifies what Sophia provides for Moonkale's ADR-0030 (SR-148): moving a folder's source of truth from its files into the store and back, and files generated from a store whose graph is the truth, with outside edits to them taken back as proposals. The split of work follows the Moonkale agent's proposal of 2026-10-26: Moonkale normalises foreign code and hands Sophia finished records, as in SPEC-009.
>
> Sources: Moonkale `markdown/decisions/ADR-0030 Where the truth lives.md` (master `874d0c2b`), ADR-0028 (the `truth` field), ADR-0029; [[SPEC-009 Moonkale Folder Store]] (the folder store, `replace_files`, the derived mode, §9 collection); [[SPEC-004 Moonkale Interface]] §3b (edit tokens, SR-77); [[SPEC-005 Rendering Profiles]]. code: `crates/sophia-store/src/folder.rs`, `crates/sophia-interface/src/edit.rs` (`refuse_if_derived`), `sophia_interface.rs` (`text_token`).

## 0. Scope

ADR-0030 (Daniel, 2026-10-26): a legacy folder keeps its files as the truth unless the user explicitly moves the truth into Sophia; everything else has the Sophia store as the truth, even when files are generated from it.

| | legacy folder (`truth: files`) | store-truth folder (`truth: store`) |
|---|---|---|
| store mode | derived (SPEC-009 §3) | normal |
| history | git; the store collects (SR-146) | the graph; nothing is removed |
| files | the truth | generated layout, written from the store |
| foreign code (Julia, Rust, Python) | Moonkale normalises, `replace_files` | Moonkale normalises, `replace_files` with tokens (§2) |
| sophialang | not in legacy folders | printed by Sophia in a profile (§4) |

Out of scope: mixing both truths in one folder (ADR-0030 point 5), and Moonkale's side (the command, the confirmation, the status bar, the marker format).

## 1. Requirements met

SR-148 (this), SR-146 (history follows the source of truth), SR-143 (per-file replacement in one transaction), SR-77 (no silent overwrite), SR-116 (one normaliser for both tools).

## 2. Foreign code in a store-truth folder: `replace_files` with file tokens

**Moonkale keeps the write path it has.** It parses the edit, normalises with `moonkale-content-id`, splices the edited definition into the file's text with its lossless syntax layer, and hands Sophia the new file text, records, spans, names and generic memberships as a `FileIngest`. Sophia stays language-agnostic for foreign code, and the ingest crate stays out of the store (it does not build for wasm). `NormaliserChanged` works as before.

**What changes in normal mode:**
- `replace_files` stays allowed in a normal store, as it is today. Collection does not run (SR-146, already implemented).
- **A file token** guards each file, as edit tokens guard names (SR-77). `Store::file_token(path) -> u64` is a hash over the file's current manifest id and, for each name the manifest lists, the name's path, `seq` and target, plus the version of each generic function it contributes to. A file that does not exist has token 0.
- **`Store::replace_files_checked(&[(FileIngest, u64)])`**: like `replace_files`, but each file carries the token Moonkale read when it last printed or read that file. If any file's current token differs, nothing is written, and the error lists every conflicting path with the expected and the found token (`StoreError::FileConflict`). A derived store refuses the call, because there the files are the truth and nothing can conflict with them.
- **Names belong to the store** (Moonkale's proposal): the manifest records where a definition sits (file, span, order) as **layout**, used to print the file again. `owner/` rows keep their meaning for consistency inside one batch: a name sits in one file, and moving it is one batch with both files (SPEC-009 §2).

**Interface edits to laid-out names.** In a normal store the interface allows edits. A sophialang name has no file, and `write_text`, `rename` and `delete` work on it as today. For a name that a file manifest lists, an interface `write_text`, `rename` or `delete` would leave the manifest describing a file the store no longer matches. **Proposal:** refuse these with `StoreError::LaidOut { path, token }`, naming the file and its current file token, so that Moonkale can route the edit through its splice and `replace_files_checked` without a further read. This is open question 1.

## 3. Moving the truth: an explicit switch, in place

**Into the store** (`truth: files` → `truth: store`):
- `Store::take_truth(&[FileIngest])`: one transaction. It replaces the given files, which must be the folder's current files, normally the stale ones. Then it clears `store/mode`, so the store is normal from then on. It records the switch: meta row `store/truth-since` = the log sequence number, and a log entry.
- **In place, not a rebuild**: the derived store already holds the files' records, source texts and manifests. A fresh normal build would hold the same records. Keeping the store keeps the layout that §4 needs, and costs no rebuild.
- From the switch on, nothing is collected. The store keeps every later version. History before the switch stays in git.

**Back to the files** (`truth: store` → `truth: files`):
1. Moonkale writes every file from the store (§4) and checks that no file token changed while it did.
2. `Store::give_truth()`: sets `store/mode = derived` and runs `collect_folder`, in one transaction.

`give_truth` **discards the store's version history**: everything that no current file reaches is collected. Moonkale's confirmation must say so, and should offer to commit the exported files to git first. Proposal: refuse `give_truth` while any file's text in the store differs from what step 1 wrote. Moonkale passes the source ids it wrote, and Sophia compares them with the manifests.

## 4. Generated files

**Printing.** For foreign code, the file text is the manifest's source text, and Moonkale splices edits into it (§2). Sophia prints only sophialang:
- `Interface::export(namespace, profile) -> Vec<ExportedFile>`, one file per namespace, definitions in name order. Each `ExportedFile` carries the path, text, spans and file token.
- After writing an exported file, Moonkale stores it back with `replace_files_checked`, so that the file has a manifest and a token like any other.

**Markers.** A generated file must say which store state it came from. Sophia provides the data; the format is Moonkale's (a header comment or a manifest under `.moonkale/`, ADR-0030):
- the file token at export time;
- the source id of the text written (`source_record(path, text).id`, as in SPEC-009 §7).

A file whose current text hashes to a different source id was edited outside Moonkale. Proposal: no marker inside the file text. A header would make the stored source differ from what compilers read, and changes to it would look like edits.

## 5. Outside edits as proposals: `propose_files`

`Store::propose_files(&[(FileIngest, u64)]) -> Proposal`, a read transaction that writes nothing:
- **per file:** names added, removed and retargeted (old and new target), memberships added and removed, and spans moved;
- **conflicts:** files whose token differs from the one given. The store changed since that file was generated, so the edit was made against an old state;
- **errors** `replace_files` would raise (`NameClaimed`, `UnreachedRecord`, `NormaliserChanged`), reported instead of thrown.

Moonkale shows the proposal. **Accept** is `replace_files_checked` with the same tokens. If the store has changed in between, it fails with `FileConflict` and the user sees a new proposal. **Reject** rewrites the file from the store. A conflict is shown as one: never merged silently, never overwritten silently (ADR-0030 point 4, SR-77).

For sophialang files, Moonkale does not normalise. `Interface::propose_text(path, text, token)` parses with Sophia's parser and returns the same `Proposal`. Proposal: this comes after §2 to §4.

## 6. Amendments from the implementation (2026-10-28)

1. **`propose_files` takes the write lock.** It runs the replacement in a write transaction that is always rolled back, rather than in a read transaction. That makes its report and errors exactly what an accept would give at that moment, without a second implementation of the diff. It writes nothing; the test checks the records, the state and the log position.
2. **Exported sophialang files are not stored back as file manifests.** §4 said Moonkale would store an exported file with `replace_files_checked`. That would lay out the sophialang names it contains, and question 1's answer would then refuse their interface edits. Instead, `Interface::export(namespace)` returns its own token, a hash over the exported names' text tokens, which Moonkale keeps as the marker. An outside edit of such a file goes through `propose_text`, which is not built yet.
3. **`export` prints in the reader's profile** for the namespace, the one `fetch_text` uses. It takes no profile argument: the profile is the reader's setting.

4. **Codex's review (2026-10-28) changed four things.** (a) A generic function's name is laid out by its contributing files, so the interface refuses to edit it too. Every name an interface edit would update is checked, not only the edited one. (b) `give_truth` refuses while a name is in no file (`NotInFiles`): a sophialang definition written while the store was the truth must be removed first, because the files cannot hold it (§0). (c) `Proposal` lists the generic functions an accept would create or remove. (d) `ExportedFile` has `aliases`, for a second name of a recursive group's member, which the text cannot define again. The export token is read in the same transaction as the names it covers. See [[folder]], [[edit]], [[sophia_interface]].

A file's token also covers the version of each generic function the file contributes to. So another file adding a method to a shared generic function makes this file's token stale: a conservative conflict.

## Acceptance

1. `replace_files_checked`, `file_token`, `take_truth`, `give_truth` and `propose_files` on all four backends, with tests:
   - a conflict writes nothing;
   - `propose_files` followed by an accept equals a plain `replace_files`;
   - `take_truth` then `give_truth` leaves a fresh build's records (as SR-146's tests do);
   - an interface edit of a laid-out name is refused (if open question 1 is accepted).
2. Moonkale's history replay switched into store-truth mode halfway: no differences against a store built fresh and switched, and the replay's later commits appended as history. **Passed 2026-10-28 (Moonkale's report, on `08e6ffb`):** Lenticulum.jl 130 commits and Moonkale 471, each switched at half. Every checkpoint after the switch showed 0 differences against the twin and against a fresh build. There were 0 errors and 0 token conflicts. After `give_truth`, 2 473 → 1 770 and 7 840 → 5 940 records, equal to a fresh build. Command: on Moonkale's branch `replay-truth` (`08e23152`), `MOONKALE_REPLAY_TRUTH_AT=half target/agent/debug/examples/history_replay <repo> <range> ~/.cache/moonkale-replay`. Rerun on `v0.1.6` (2026-10-29): the same counts, 0 differences.
3. `Interface::export` round-trips: exporting, then storing the export back, changes no name. (Amended by §6.2: the export is not stored back as a manifest; the round trip goes through `propose_text`, not built yet.)
4. A tag after 1 to 3, as for `v0.1.x`.

## Questions for Daniel, answered 2026-10-26

**Accepted as recommended (Daniel):** 1 refuse, with `LaidOut { path, token }`; 2 a confirmation, after Moonkale copies the store file; 3 one file per namespace in name order; 4 markers outside the text, in Moonkale's `.moonkale/generated.json`. 5 below.


1. **Interface edits of file-laid-out names in store-truth folders:** refuse them so all foreign-code edits go through Moonkale's splice (proposal), or let Sophia update the manifest and leave the file to be regenerated?
2. **`give_truth` loses the store's history.** Is a confirmation enough (proposal), or should Sophia keep the history (switch to derived without collecting until the user asks)?
3. **Layout of sophialang code in files:** one file per namespace in name order (proposal), or a layout the user keeps, stored like a foreign file's manifest?
4. **Markers outside the file text** (proposal), or a header line in the file (ADR-0030 allows either)?
5. **Sharing a store-truth folder** (raised by the Moonkale agent). In a legacy folder `.moonkale/` is gitignored, because the store is a cache. In a store-truth folder the store is the truth: a fresh clone, a second machine or a collaborator needs it, and needs `generated.json` to know which files are generated. Without an answer, store-truth folders work on one machine only. Options:
   - (a) commit the SQLite file: simple, but binary diffs and merge conflicts;
   - (b) commit a text form of the store beside the generated files, and rebuild the SQLite file from it;
   - (c) sync through a Sophia server.

   **Proposal: (b), in a form that suits content addressing.** Records go one file per id: they are immutable, so two branches never conflict on a record, and a merge is a union. Names, memberships and manifests go in text files sorted by path: these are the only parts that can conflict, and git shows them line by line. Recording the version history (the log and lineages) in that form needs its own measurement. This is large enough for its own spec.

   **Answered by Daniel (2026-10-26, relayed by the Moonkale agent): (b), a text export, is fine for now.** Later perhaps a file type that only says which information to request from the store, a pointer like a git-lfs pointer file, instead of the full text. The export format gets its own spec (SR-149).

## Moonkale's review (2026-10-26)

The Moonkale agent's comments, Moonkale's side; Daniel decides:
- **Q1:** yes, refuse. One write path for foreign code. The error carries the path and the current file token (`LaidOut { path, token }`, now in §2). Moonkale's `Source` for a store-truth folder routes graph edits, renames and deletes through its splice.
- **Q2:** a confirmation is enough. Moonkale copies the store file to `.moonkale/code.sophia.before-give-truth-<date>` first, which needs nothing from Sophia. The refusal while any text differs from the export stays.
- **Q3:** one file per namespace to start with; a user-kept layout later, stored like a foreign manifest.
- **Q4:** markers outside the text, in `.moonkale/generated.json`, per path: token and source id.
- **New, Q5 below:** where a store-truth folder's store lives and how it is shared.
- Moonkale shows the mode in the status bar and explorer. Its replay tool can run acceptance 2 once the calls exist.

## Related

[[SPEC-009 Moonkale Folder Store]] · [[SPEC-004 Moonkale Interface]] · [[SPEC-005 Rendering Profiles]] · [[Folder Store Garbage]] · [[Requirements]] (SR-146, SR-148)
