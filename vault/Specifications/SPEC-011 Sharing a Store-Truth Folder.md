#design #definition

> **Draft, a proposal by Claude (Sophia), 2026-11-18 (B-004). Not accepted, not built.** It specifies the text export that SR-149 asks for. When a folder's truth is its Sophia store (SPEC-010), the folder is shared through git like this: Sophia writes a text form of its store into the folder, and that text is committed beside the generated files. A fresh clone, a second machine or a collaborator rebuilds the store from it. It also covers what a git merge of two such exports means. The pointer-file idea of SR-149 is kept as a later option (§9). Sophia owns the format and the two calls. Moonkale owns the directory, when to call them, and what the user sees. Questions for Daniel are at the end. Mistral reviewed the first version against the code on 2026-11-18. Of its 12 findings, 11 were right and 1 was partly right, and all are folded in. The triage is in [[Review 2026-11-18 Mistral SPEC-011 (external)]].
>
> Sources: [[SPEC-010 Store-Truth Folders and Generated Files]] (question 5 and its answer, §3 to §5); [[SPEC-009 Moonkale Folder Store]]; [[SPEC-001 Sophia Store v0]] (records, names, nominal entities, lineages, annotations, facts, the log); [[Current Inventory]] (which tables are authoritative and which derivable); [[Storage and Materialization]]. git: `gitattributes(5)` (the built-in `union` merge driver). code: `crates/sophia-store/src/types.rs` (the stored types), `backend.rs` (what a backend can enumerate), `folder.rs` (meta rows, `source_record`, modes), `logic.rs` (`fresh_nonce`, `verify`), `crates/sophia-interface/src/sophia_interface.rs` (`export`).

## 0. Scope

**In scope:**
- what the text export holds and what it leaves out;
- its layout and line formats;
- how it merges under git;
- `Store::export_text` and `Store::import_text`;
- the checks an import makes;
- what a rebuild loses.

**Out of scope:**
- **Folders whose files are the truth.** There the store is a cache, built from the files (SPEC-009), so there is nothing to share. `export_text` refuses a derived store.
- **The generated files themselves.** Those are SPEC-010 §4. They stay what people read and review. The export is what a store is rebuilt from.
- **A Sophia server** (SPEC-010 question 5, option c).
- **Moonkale's side:** the directory's location, the trigger, the status bar, how conflicts are shown.

## 1. Requirements met

- **SR-149:** sharing through a text export, committed beside the generated files, and a rebuild from it. The pointer-file idea is §9.
- **SR-148:** store-truth folders work on more than one machine.
- **SR-77:** no silent overwrite. An import never discards a local change that was not exported (§6).
- **S-ADR-0004:** Rust only. Both calls are in `sophia-store` and build for wasm (§5).

## 2. What is exported

The rule: export what is authoritative in a store-truth folder; leave out what is local, what can be rebuilt, and what is a cache. The tables are classified in [[Current Inventory]]. The meta rows are listed in [[folder]] and `logic.rs`.

| part | exported | why |
|---|---|---|
| records (definitions, sources, file manifests), by component | **yes**, every record, current and old | Store truth keeps every version (SR-146). Old versions are the history that names and lineages point into. |
| names: path, target, visibility | **yes**, without `seq` | the authoritative map |
| nominal entities: id, language, kind, label | **yes**, without `created_seq` and `version` | Ids come from a random nonce (`fresh_nonce`), so two branches never create the same id. |
| relations: the **live** members of each nominal entity | **yes**, members only | needed to dispatch and to print |
| lineages: (lineage, id, supersedes) | **yes**, without `seq` | the version history of each name, as edges |
| annotations | **yes** | authoritative in store-truth folders (ADR-0031, SPEC-010 §7) |
| facts listed by a current file manifest, and facts marked `fact-external/` | **yes**, with the external marks as a set | needed to rebuild the manifests' fact rows; the external mark records that a fact was stored before any file contributed it, so the folder writer must never remove it ([[folder]]). Nothing else records that |
| `file/<path>`: path → current manifest | **yes** | layout of the foreign files |
| settings: `profile/`, `reader/`, `ns/`, `folder/normaliser` | **yes** | rendering profiles, readers' choices, empty namespaces, the folder's normaliser |
| `owner/`, `member/`, `defined-in/`, `fact/`, `nominal-index/` | no | derived from the manifests and nominal entities; recomputed on import (§5) |
| the log, and every `seq` (names, relations, lineages, nominal versions) | no (question 1) | local positions: two clones number differently, and a shared log would conflict on every concurrent edit |
| removed memberships (`removed_seq`) | no (question 1) | part of the log's history |
| other facts | no (question 3) | computed; a cache under SPEC-001 §8a |
| `query-result/`, `settings-version/`, `world/`, `store/truth-since` | no | caches and local counters; `truth-since` is set again on import |

**What git keeps instead of the log.** Every committed export is a full state, and records are never removed from it. So `git log` of the export is the shared history at commit granularity. Every old version stays readable by its id, and its lineage edges stay. What is lost is the order of edits *between* two commits, and who made each one. Moonkale already shows history from git in legacy folders.

## 3. Layout

The export is a directory tree. Moonkale chooses where it lives; the proposal is `.moonkale/store/` (question 5). All files are UTF-8 text with `\n` line ends. Within a file, lines are sorted bytewise and unique.

```text
FORMAT                       sophia-text-export 1
settings                     key \t value
files                        path \t manifest-id
nominals                     id \t lang \t kind \t label
lineages                     lineage-id \t id \t supersedes-id-or-dash
facts-external               fact-id
names/<namespace>            name \t c:<id> | n:<id> \t public|private
relations/<nominal-id>       member-id
records/<2 hex>/<component-id>
annotations/<2 hex>/<annotation-id>
facts/<2 hex>/<fact-id>
```

Fields are separated by tabs. Inside a field, `\`, tab and newline are escaped as `\\`, `\t` and `\n`. Ids are lowercase hex. Names are grouped by namespace, one file per namespace, as `export` groups the generated files (SPEC-010 Q3).

**File names.** A namespace path may hold any character a name may (`validate_path` in `logic.rs` allows `/` inside a backquoted segment). The file system may also ignore case (macOS, Windows). So a namespace's file name is its path with every byte outside `[a-z0-9._-]` written as `%XX`: upper case, `/`, `%` and backquotes are all encoded. Two distinct namespaces then never share a file name, on any file system. A name at the root goes in `names/@root`. `@` is always encoded, so no namespace can produce that file name.

### 3.1 Records

There is one file per **component**, holding its members in canonical index order. Content ids are verified per component (`Record::verify`), and a recursive group is written and read as a whole. A record has fields that are not text: its canonical bytes and its display. So the file is an envelope:

```text
record 0
id <member id>
lang sophialang
normaliser sophialang@3
kind def
status …
refs <id> <id> …
nominal-refs <id> …
bytes base64 <length>
<base64, wrapped at 76 columns>
display utf8 <length>
<the text, exactly <length> bytes>
end
```

A byte field is written as `utf8` when it is valid UTF-8, and as `base64` otherwise. A source record's bytes are its file's text (`source_record` in `folder.rs`), so a source file in the export reads as itself, and git's delta compression sees successive versions of it as text. The length makes the envelope unambiguous whatever the text contains. Component files are written once and never change, except for one case in §4. So they almost never merge, and `.gitattributes` can mark them `-diff` (question 2).

The envelope holds every field of `Record`:
- `index` is the number after `record`;
- the component id is the file name;
- `component_size` is the number of records in the file;
- the member `id` is written explicitly.

A member's id is `member_digest(component, index)`. The import recomputes that for every record. It also recomputes the component id itself wherever Sophia can (§5, step 2).

### 3.2 Annotations and facts

There is one file per id, in the same envelope:
- **annotations:** kind, subjects, payload, checker, checker version, status, inputs;
- **facts:** kind, subject, value, by, version, inputs, `derived_at`.

The import recomputes both ids from the fields, with `Annotation::new` and `Fact::new` (§5, step 2). A fact's `inputs` can name a relation at a `version`, which is a log position (`Input::Relation`). The fact is written as stored, so a round trip keeps its bytes. But after an import, the relation's version is a new local position. So `Store::is_stale` reports such a fact as stale until a tool computes it again. That is right for a computed fact, and it is part of what a rebuild loses (§6).

An annotation's id hashes its claim, but its `status` and `inputs` can change (SPEC-010 §7). So an annotation file can change, and two branches that change one annotation's status conflict in git, as they should.

### 3.3 The root digest

The root is a digest over the sorted list of (path, digest of the file's bytes), for every file of the tree. It is **computed, never stored in the tree**. `export_text` and `import_text` both return it, and Moonkale keeps it locally. A stored copy would conflict in every merge, and an import would have to trust it. Two stores in the same state give byte-identical exports and the same root. That makes two checks cheap:
- whether the store has changes that were not exported (§6);
- whether the directory changed under the store (after a pull).

It is also what a pointer file would name (§9). After a union merge, files can hold unsorted or repeated lines. Their root then differs from the root of the store rebuilt from them. That is expected: the rebuilt store's next export writes them canonical again.

## 4. Merging under git

The parts fall into two classes.

**Grow-only sets.** These are `records/` and `facts/` (as file sets), `nominals` and `lineages`. `annotations/` is a file set too, but an annotation file's status can change (§3.2). Nothing in them is ever removed:
- a record or fact file is immutable;
- a nominal entity is never deleted;
- a lineage only gains edges.

Two branches adding to these never conflict. New files in a directory merge cleanly. For `nominals` and `lineages`, Sophia supplies `.gitattributes` lines with git's built-in `union` driver, so that two branches appending near each other do not conflict either. A union merge can leave lines unsorted or repeated. The import accepts that, and the next export writes the file sorted again.

A union of lineage edges is a union of two valid edge sets, but it is not always valid itself. Say both branches give one id in a lineage different predecessors, or the edges close a cycle. That cannot come from one store, and git cannot see it. The import refuses both (§5, step 4).

**One exception: `display`.** A record's id does not cover its `display`. Two files with identical text share one source record, and its display keeps the path that was written first. [[Folder Store Garbage]] counts 79 such records on Moonkale's replay and 14 on Lenticulum.jl's. So two branches can write one component file with different displays, and git reports a conflict on a file that should never change. Both versions are valid, so either may be kept. Proposal: the import refuses only conflict markers, as everywhere else, and Moonkale resolves such a file by taking either side without asking the user. Sophia's merge driver (§8) would do the same. The alternative is to leave `display` out of the envelope for source records and to rebuild it from the manifest that names the source. That is cleaner, but it changes what a source record holds after a round trip.

**Maps.** These are `names/`, `relations/`, `files` and `settings`. A line can be removed or changed, so these use git's normal three-way merge:
- edits to different names merge;
- two edits of one name conflict, and so do two edits that touch adjacent lines.

A union driver would be wrong here: it would bring a removed name back.

**A textual merge can be inconsistent even without a conflict.** Say one branch edits file `a.jl` and the other edits `b.jl`, and both move one method between those two files. Each side's `files` line merges, but the merged manifests may both claim the same name. The import checks this (§5) and refuses with the list. It never repairs silently.

**After a merge, the generated files are compared, not trusted.** git also merged the generated files, as text. Once the store is rebuilt from the merged export, Moonkale prints each generated file again. A file whose merged text differs from the printed one is an outside edit, and it becomes a proposal (`propose_text` for sophialang, `propose_files` for foreign code, SPEC-010 §5). The export is the truth, and the generated files are checked against it.

## 5. The two calls

```rust
/// A store-truth store's state as a text tree (path → bytes), §2 and §3.
/// Refuses a derived store (`StoreError::Derived`). One read transaction.
Store::export_text(&self) -> Result<ExportTree>

/// Rebuilds a store from a text tree into this store, which must be empty.
/// One write transaction: on any error nothing is written.
Store::import_text(&self, tree: &ExportTree, limits: ImportLimits) -> Result<ImportReport>
```

`ExportTree` is a `BTreeMap<String, Vec<u8>>`. The store crate does no file I/O, so both calls work on wasm. `sophia-cli` gets `sophia export-text DIR` and `sophia import-text DIR` for tests and by hand. Moonkale writes the tree into its directory itself:
- it writes the files whose bytes changed;
- it deletes the files that are gone. Only `names/` and `relations/` files can go: a namespace or a relation that became empty.

**`import_text`, in order:**
1. **Parse.** Read `FORMAT` (only version 1). Parse every file. A malformed line is refused with `ImportError { path, line, what }`. Git conflict markers are reported as such, but only where the parser expects a line. Inside a length-delimited block of a record envelope, the bytes are data: a source file may well contain `<<<<<<<`. A conflict inside an envelope breaks its structure, and that is refused like any malformed envelope. All errors are collected, up to a limit.
2. **Verify records.**
   - Every record: its `id` must equal `member_digest(component, index)`, and its file must hold indices 0 to n−1.
   - Sophialang components: recompute their ids (`Record::verify`).
   - Source records and file manifests: recompute their ids from their bytes.
   - Foreign records (Moonkale's normalisers): they cannot be re-hashed in Sophia, as with `Store::verify` (`Foreign`). They are accepted under their file name's id. They are checked by being reachable: from a manifest whose source text is in the export, or from a lineage. The import report counts them.
   - A mismatch is refused, and so is a component whose size disagrees with its members (SPEC-010, SR-153).
   - Annotations and facts: recompute their ids from their fields. A mismatch is refused.
3. **Check references.** Every id referenced exists in the tree:
   - a name's target;
   - a relation member;
   - a lineage edge's ids;
   - a record's `refs`;
   - a manifest's source, records and facts;
   - an annotation's subjects and inputs;
   - a fact's subject, and its content inputs;
   - every id in `facts-external`.

   Nominal references must be in `nominals`, both a record's `nominal_refs` and a fact's relation inputs.
4. **Check consistency.** This is what a textual merge can break:
   - every definition name a current manifest lists has that manifest's target (as `give_truth`'s `ExportMismatch`, SPEC-010 §6.5);
   - no definition name is owned by two manifests. Generic functions are not definition names: several files may contribute methods to one, as `replace_files` allows;
   - every generic membership a manifest lists is a live member of that relation.

     The converse is not checked: in a store-truth folder, the interface adds members that no file lays out (sophialang relations);
   - a lineage holds each id at most once (the store keys lineages by (lineage, id), with one `supersedes`);
   - the lineage edges contain no cycle.
5. **Write**, in one transaction:
   - records, then nominal entities;
   - names, relations and lineage edges, with `seq`s assigned in the order of the sorted files (lineages in topological order of `supersedes`);
   - annotations, facts and settings;
   - the `file/` rows, then the derived rows: `owner/`, `member/` counts, `defined-in/`, `fact/` counts, `nominal-index/`. These are computed from the manifests by the code `replace_files` uses, not by a second copy of it;
   - one log entry of a new `LogOp::Import`, whose subject is the tree's root digest. `LogOp` gains a variant, and every backend's decoder learns it. A store written by a newer Sophia is then not readable by an older one. That is acceptable while Sophia has no users (SR-118);
   - `store/truth-since` set to that entry's seq. It stays a log position, as SPEC-010 §3 defines it.

   The store stays normal (store truth).

`ImportReport` gives the root digest, and the counts per part, with foreign records counted separately. `ImportLimits` caps the total bytes, the files, the lines per file, and the errors collected. A collaborator's export is untrusted input.

**Round trip.** `import_text(export_text(s))` followed by `export_text` gives the same tree, byte for byte. The rebuilt store equals `s` on everything §2 exports, and on the derived rows.

## 6. When Moonkale calls them

A proposal for Moonkale; the decision is Moonkale's.

- **Export** whenever it writes the generated files (SPEC-010 §4). Then the tree on disk always matches the generated files beside it. It records the root digest `export_text` returned, locally, for instance in `.moonkale/generated.json`.
- **Import**, into a fresh store file that replaces the old one only on success:
  - when the store file is missing (a fresh clone);
  - when the directory's root digest differs from the one it last wrote (after a checkout, pull or merge). Moonkale computes that digest over the files as §3.3 says, or reads it from `import_text`'s report.
- **Unexported local changes.** Before an import replaces the store, Moonkale compares the store's own `export_text` root with the last one it wrote. If they differ, the store holds changes the directory does not, and an import would drop them. Moonkale refuses, as git refuses a checkout over local changes. It offers to export first. Exporting over a directory that changed underneath means merging, so the user goes through git: commit the export, then merge.

**What a rebuild loses** (all local, §2):
- the log and the change feed: Moonkale's subscriber starts again from the import's entry;
- every edit, file and export token: Moonkale must re-read before its next checked write, as after any conflict;
- the query cache, other facts, and world snapshots;
- the freshness of facts that depend on a relation's version (§3.2).

## 7. Size

Not measured yet, so these are expectations, not claims:
- Moonkale's own repository in store-truth mode held 7 840 records at the end of its replay (SPEC-010 acceptance 2), so the export would hold up to as many component files;
- [[Folder Store Garbage]] measured the replayed stores as SQLite files: 91.2 MB for Moonkale's, holding 11 541 records, all versions kept; 36.2 MB for a fresh build, holding 5 940. Source texts are 73 % of the old versions' payload;
- the export should be smaller than the SQLite file, since it has no indexes, no log and no `refs` tables;
- a git pack of it should be smaller still, since source texts are stored as text.

Acceptance 5 measures this.

## 8. Not in this version

- **A Sophia git merge driver** for `names/` and `relations/`. It would merge per name rather than per line, so adjacent edits would not conflict. That is worth building if false conflicts are frequent in practice.
- **An incremental export.** Records are immutable, so only new components need writing. The first version computes the whole tree, and Moonkale writes the files that changed. Measure first.
- **An incremental import** after a pull, applying only the difference. The first version rebuilds.
- **Sharing the log.**

## 9. Later: a pointer file (SR-149)

Daniel's later option is a file that only says which information to request from the store, like a git-lfs pointer. The layout above prepares for that:
- the root digest names a whole state;
- every record, annotation and fact file is named by its content id.

A pointer file would hold the `FORMAT`, the root digest, and a location. The small mutable files (`names/`, `relations/`, `files`, `settings`, `nominals`, `lineages`) could stay in git, where they diff and merge. The `records/`, `annotations/` and `facts/` files would be fetched by id from a shared content store, as git-lfs fetches objects. Nothing here needs to change for that.

## Acceptance

1. **Round trip, on four backends.** Use a store-truth store with history: two files sharing a generic function, an edit that moves a name, a recursive group, a removed name, an annotation with a changed status, a manifest fact, an external fact also contributed by a file, a namespace whose name needs `%` encoding, a sophialang name written through the interface, an empty namespace, and a reader profile. `export_text` → `import_text` into an empty store → `export_text` is byte-identical. The rebuilt store equals the original on every part §2 exports, and on `owner/`, `member/`, `defined-in/` and `fact/`.
2. **A merge.** Make two stores from one export, and on each side add a different name and a different method to the same generic function. Merge the two trees with `git merge-file`, using `union` where §4 says so. The import gives the store both edits would give. Two edits of one name leave conflict markers, and the import refuses them with path and line.
3. **Untrusted input.** Each of these is refused with path and line, and nothing is written:
   - a changed byte in a sophialang record or a source;
   - a member id that is not `member_digest(component, index)`;
   - an annotation or a fact whose id does not hash its fields;
   - a missing referenced id;
   - two predecessors for one id in a lineage, or a lineage cycle;
   - an inconsistent manifest (two manifests claiming a name);
   - a malformed line;
   - conflict markers in `names/`. A source record whose text contains `<<<<<<<` is accepted;
   - a tree over the limits.
4. **The derived store is refused** by `export_text`.
5. **Measured on Moonkale's replay store** (switched to store truth at half): file count, bytes, and `git count-objects -vH` after a commit of the export, against the SQLite file. Also export and import times. Recorded in [[Benchmark Results]] with the command.
6. `sophia-store` builds for `wasm32-unknown-unknown` with both calls.

## Questions for Daniel

1. **History.** The proposal shares states and lineage edges, not the log (§2). The order of edits between commits, and who made them, stays on the machine that made them. The alternative exports the log too, at the cost of a git conflict on every pair of concurrent edits. Is git's history of the export enough?
2. **Records in a text envelope** (proposal, §3.1), base64 where the bytes are not UTF-8. The alternatives: the store's binary codec (smaller, not text), or sophialang records printed as sophialang (readable, but every import then depends on the parser and printer staying stable).
3. **Facts:** export only those a current manifest lists (proposal), or every fact?
4. **After a conflicting merge**, the import refuses until the conflict markers are resolved (proposal); Moonkale may offer "ours" or "theirs" per name. Or should Sophia ship a merge driver from the start (§8)?
5. **Location** (Moonkale's to answer): `.moonkale/store/`, committed, while the SQLite file stays gitignored?

## Related

[[SPEC-010 Store-Truth Folders and Generated Files]] · [[SPEC-009 Moonkale Folder Store]] · [[SPEC-001 Sophia Store v0]] · [[Current Inventory]] · [[Folder Store Garbage]] · [[Requirements]] (SR-148, SR-149)
