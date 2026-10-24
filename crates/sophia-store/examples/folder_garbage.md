#implementation #measurement

> How much of a folder store ([[SPEC-009 Moonkale Folder Store]]) no current file reaches: the measurement before any garbage collection ([[Garbage Collection]], SR-126). Read only (run it on a copy: opening applies the schema). Source: `folder_garbage.rs`. Results: [[Folder Store Garbage]].

```sh
cargo run --release -p sophia-store --example folder_garbage -- STORE.sqlite [IDS.tsv | --collect]
```

`IDS.tsv` lists every non-live record with its state. `--collect` first runs `Store::collect_folder`, which writes, and times it.

**Roots:** the current file manifests (`file/<path>`) with the source, named records, generic members and spanned records each lists; every name's target; every generic function's current members. From those it follows records' `refs`.

**Each record is put in one of three states:**
- **live**: reached from a root.
- **lineage-only**: not reached, but an earlier version in the lineage of a live record. It stays if lineage history is a root.
- **unreached**: neither.

Each state is split into manifests, source texts and definitions.

**Payload bytes** = canonical bytes + display + 32 per reference. This is the record's own data, not its size on disk. For disk size, use `sqlite3 STORE "SELECT name, sum(pgsize) FROM dbstat GROUP BY name"`.

**Also counts:**
- facts about live and other records;
- log entries by operation.

**Not counted:** annotations; nominal records of retired generic functions.
