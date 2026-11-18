#reference

> Mistral Large 4 reviewed the first draft of [[SPEC-011 Sharing a Store-Truth Folder]] against the code on 2026-11-18. It ran read-only through the `vibe` CLI (`--agent ask`) in the main checkout, and was asked for defects only. It reported 12 findings. Claude checked each against the code: 11 are right, and 1 is partly right. All are folded into the draft. No code exists yet, so none of this could be checked by a test. Each finding was checked by reading the code it cites.
>
> Sources: the answer and the brief, kept outside the repository in `/home/daniel/.cache/agent-usage/vibe/2026-11-18-0042-spec011/` (`ANSWER.md`, `brief.md`; cost $0, Mistral Large 4). code: `crates/sophia-hash/src/merkle.rs`, `crates/sophia-store/src/types.rs`, `logic.rs`, `folder.rs`, `sqlite.rs`, `crates/sophia-interface/src/edit.rs`.

| # | finding | right? | what was done |
|---|---|---|---|
| 1 | Record files are named by component, but a member's id is `member_digest(component, index)`. The envelope wrote no id and no index, so foreign records could not be placed. | right (`merkle.rs` `member_digest`) | The envelope writes `id`; the index is the `record n` number; the import checks `id = member_digest(component, index)` for every record (§3.1, §5 step 2). |
| 2 | A log entry `import` is no `LogOp`. Every decoder would fail on it. | right (`types.rs` `LogOp`, `LogOp::parse` in the decoders) | A new `LogOp::Import`, stated as a format change (§5 step 5). |
| 3 | A fact's relation inputs carry a log position. After an import, the round trip changes bytes and `is_stale` misreports. | partly | The bytes do not change, because a fact is written as stored and its id covers the version. But such a fact reads as stale after an import. That is now stated as a loss of the rebuild (§3.2, §6). |
| 4 | `ROOT` in the tree is never verified, and it conflicts in every merge. | right | The root digest is computed, never stored in the tree (§3.3). |
| 5 | `fact-external/` was called derivable. It is not: only it records that a fact predates every file. | right (`folder.rs`, `fact_external_key` and its use in `replace_in`) | Exported as the set `facts-external` (§2, §3). |
| 6 | Annotation and fact ids were not recomputed on import. | right | Recomputed with `Annotation::new` and `Fact::new`; a mismatch is refused (§5 step 2). |
| 7 | Fact and annotation `inputs` were not checked for existence. | right | Added to §5 step 3. |
| 8 | A union merge of lineages can give one id two predecessors, or a cycle, with no conflict. | right (`put_lineage` keys on (lineage, id)) | The import refuses both (§4, §5 step 4); added to acceptance 3. |
| 9 | Scanning for `<<<<<<<` refuses a source file that contains that text. | right | Markers are detected only where a line is expected, never inside an envelope's length-delimited block (§5 step 1); acceptance 3 tests a source that contains them. |
| 10 | A namespace may contain `/` (inside backquotes), and `_root` can collide with a real namespace. | right (`validate_path`) | File names are percent-encoded, with upper case encoded too, which also makes them safe on case-insensitive file systems. The root file is `@root` (§3). |
| 11 | The check "every member that comes from files has a manifest" cannot be decided, and "no name claimed by two manifests" would refuse shared generic functions. | right (`edit.rs` adds members in store-truth stores; `replace_files` allows shared generic functions) | Only the direction manifest → live member is checked, and only definition names must have one owner (§5 step 4). |
| 12 | `store/truth-since` was set to a digest, where SPEC-010 §3 defines it as a log position. | right (`folder.rs` `take_truth`) | It is set to the import entry's seq (§5 step 5). |

**Not reported, as Mistral checked:** `owner/`, `member/`, `defined-in/`, `fact/` and `nominal-index/` can be computed from manifests and nominal entities, and `removed_seq` is invisible to every read path (`removed_seq IS NULL`).
