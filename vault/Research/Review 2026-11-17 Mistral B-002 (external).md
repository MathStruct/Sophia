#reference

> A read-only correctness review of B-002 (annotations move with the truth, [[SPEC-010 Store-Truth Folders and Generated Files]] §7) by Mistral Large 4 through the `vibe` CLI on 2026-11-17, in the helper worktree `../Sophia-mistral`. Codex was asked first, but its quota was used up. Mistral found no defect and one missing test, now added.
>
> Sources: the review's answer file, kept outside the repository at `~/.cache/sophia-corpora/vibe/review-b002.md`. code: `folder.rs`, `logic.rs`, `backend.rs`, `kv.rs`, `sqlite.rs`, `duckdb_store.rs`, `tests/folder.rs`.

## How it was run

```sh
vibe -p --prompt-file review-brief.md --agent review --trust --workdir ../Sophia-mistral --max-price 1 --max-turns 100
```

The brief gave the §7 requirement and the diff, asked for defects only, and forbade cargo (a gate was running).

## Points

| point | right? | what was done |
|---|---|---|
| No test shows that a `take_truth` whose files fail mid-transaction stores no annotation; it holds only because every backend rolls back | right (a missing test, not a defect) | added to `annotations_move_with_the_truth`: a batch with an unreached record and an annotation is refused with `UnreachedRecord`, the annotation is absent and the store stays derived; passes on all four backends |
| The key-value codec decodes `checker: Some("")` as `None`, so such an annotation would never compare equal and would be logged on every sync | not checked by a test; informational | left as is: `Annotation::new` takes a checker name, and no caller passes an empty one. Recorded here in case one does |
| `give_truth` scans every annotation id without a limit | right, by design | the check needs every id; bounded by the annotation count |

It confirmed, by reading, that the refusal precedes every write in one transaction on all four backends; that collection never removes annotations; and that no other `ReadTx` implementor lacks `annotation_ids`.
