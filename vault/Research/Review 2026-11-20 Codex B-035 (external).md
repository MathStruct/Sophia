#review #external

> **Codex's review of B-035 part 1 (`fact_cache.rs` and its notes), 2026-11-20, defects only, high effort.** Eleven findings. All are right.
> - Two change what the results may claim: Julia ids are computed per file while Moonkale resolves module-wide (1), and the small review copies do not follow the fresh-folder headline (2).
> - Three corrected readings in the notes (3 to 5).
> - Six were defects in the program (6 to 11). Each is fixed by reading the code; none was reproduced with a failing input, since this is a measurement program without tests.
>
> **The results have not been re-run yet.** A Julia benchmark of Daniel's was running, so the tables in [[Fact Cache Experiment]] are from the program before the fixes and say so.
>
> Sources: `codex exec -s read-only -c model_reasoning_effort=high`, with a brief naming the requirement and the files and no reasoning sent (`meta/scratchpad/review-brief-B-035.md`); its answer is in `meta/scratchpad/REVIEW-from-codex-B-035.md`. code: `crates/sophia-ingest/examples/fact_cache.rs`, [[fact_cache]].

| # | finding | right? | what was done |
|---|---|---|---|
| 1 | Moonkale resolves Julia references module-wide, so changing one file can change ids in another; the program resolves per file | **right**, checked in Moonkale's `folder_store/producer.rs` (`cid::julia::module`) | stated in both notes. Open: a module-wide Julia mode (B-035 part 1b) |
| 2 | the three review copies are served 4 to 21 % from the cache, not 86 to 89 % | **right**, checked against `summary.tsv` | the headline is limited to full worktrees, and the review copies are reported apart |
| 3 | the frontend's 357 µs is not saved by a hit (the ids come from the frontend); no replay was timed with and without the cache | **right** | reading corrected: a hit saves only the tool's own cost beyond the frontend, which was not measured |
| 4 | the lookups were 16 705 hits and 16 705 misses, not 50 000; the percentiles pooled hits and misses | **right**, `n` was capped by the number of ids | 50 000 each, reported apart; the note states what the old numbers were |
| 5 | "same folder" means "live here before", not "computed here"; the claim about Sophia's 122 hits was not supported | **right** | the column's meaning is stated; the claim is removed |
| 6 | the memo keys differed between modes (git sha against blake3), and ignored the language | **right** | one key `lang:blake3(text)` in both modes |
| 7 | refused definitions were not counted, and a file refused at the frontend's deadline counts as empty under load | **right** | refused definitions and files are reported; the note says a refused file may mean load |
| 8 | quoted git paths (a tab in a name) were dropped | **right**, by reading | `ls-tree -z`, `ls-files -z` |
| 9 | symlinks and non-UTF-8 texts were read differently in the two modes | **right**, by reading | symlinks skipped in both modes; lossy decoding in both |
| 10 | `.` as a folder, a repository with no source files, and a missing directory panic or pass silently | **right**, by reading | paths canonicalised, a missing directory is an error, the benchmark is skipped with no ids |
| 11 | "on disk" was the apparent length; a prefix match could count unrelated files | **right**: redb 1 KiB allocates 46.2 of 67.4 MB | exact file names (`""`, `-wal`, `-shm`); length and allocated bytes both reported |
