#implementation #measurement

> B-035, the fact cache experiment of [[SPEC-001 Sophia Store v0]] §8a (SR-152). It replays git histories and fresh checkouts as folder stores, and counts how many per-definition tool runs a per-user cache of content-only facts would save beyond what the folder store already holds. It then builds fact-only cache stores (SQLite and redb) at the size reached and times lookups. It changes no store. Source: `fact_cache.rs`. Results: [[Fact Cache Experiment]].

```sh
cargo run --release -p sophia-ingest --example fact_cache -- OUTDIR \
    --history REPO [--history REPO ...] [--head DIR ...]
```

The arguments are processed in order, and all of them share one cache. Each one is its own folder (at most 31).

- **`--history REPO`:** every commit of `git rev-list --first-parent --reverse HEAD` is one event: the commit's whole tree, read through `git ls-tree` and `git cat-file --batch`. The first commit is a fresh build. It is printed but left out of the history's totals.
- **`--head DIR`:** the checkout's current files as one fresh folder. That is `git ls-files` with uncommitted changes, or, when `DIR` is no git checkout, every file except hidden directories, `target/` and `node_modules/`.

Files are `.jl`, `.rs` and `.py`. Symlinks and submodules are skipped, paths are read with `-z`, and texts are decoded lossily in both modes. Each distinct (language, text) pair goes once through `code::file`, Sophia's per-file frontend. Its definition ids are the cache keys. Both modes share the memo key `lang:blake3(text)`, so a text is parsed and timed once.

**Not Moonkale's ids for Julia.** Moonkale's folder store resolves Julia references module-wide (`cid::julia::module` in its `folder_store/producer.rs`). When one file changes, ids in files that did not change can change too. This program resolves within each file, so its Julia counts are an approximation. Rust and Python are per file in both tools.

**The model.** A tool computes one content-only fact per definition, keyed by the definition's id. In each event, every definition in a file whose text changed would be recomputed without reuse (`defs`). It is counted as:
- **`local`:** the folder store already has it. Its id is live in the folder before the event, or was already computed earlier in the same event. A tool that asks its own folder store first needs no cache for these.
- **`cache_same` / `cache_other`:** not local, but in the cache. For `cache_same`, the id was live in this folder at an earlier event (a revert, a definition that comes back). For `cache_other`, it was never live in this folder. Where it was computed is not tracked.
- **`runs`:** neither, so the tool really runs.

The cache is `all-content-only` without eviction: every id ever computed.

**Check.** After each history, the replayed live multiset must equal a fresh build of HEAD's tree. Otherwise the run aborts. This checks the replay's bookkeeping (replacements and deletions), not the selection of files: both sides share that.

**Output in OUTDIR:**
- `commits.tsv`: one row per commit;
- `summary.tsv`: totals per folder, the number of distinct ids, the refused files and definitions, the frontend's time per definition, and the cache stores.

A definition the normal form refuses gets no id, so it gets no fact and is not counted. A file refused whole, by a parse error or by the frontend's deadline, counts as having no definitions. A count of refused files above zero may mean that the counts depended on load: check it against an idle run.

The cache stores are `cache-{8,72,1024}.{sqlite,redb}`. Each holds one fact per distinct id, with a value of that many bytes. They are written through `Store::record_facts`, so each fact is also logged. The lookups are 50 000 hits (ids drawn with replacement) and 50 000 misses, interleaved, each `facts_of(subject)` in its own read transaction. Hits and misses are reported apart. Sizes are taken after the store is closed, which checkpoints SQLite's WAL. Each is reported twice: the apparent length and the allocated blocks of the file and of its `-wal` and `-shm`.

**What it does not measure:**
- facts with an `Input::Relation`, which the cache refuses;
- several fact kinds per definition. Size is expected to grow roughly linearly with the number of kinds (not measured);
- a replay timed with and without the cache. A hit does not save the frontend, which must run to produce the ids;
- the cost of an expensive tool. None exists in Sophia yet (SPEC-001 §8a), so that half of the experiment is open.

The timings are wall-clock times on a shared machine.
