#experiment #benchmark

> B-15: Moonkale's edit loop through Sophia, timed end to end on Moonkale's side (`moonkale-sources-sophia` example `edit_loop`, written by the Moonkale agent), on synthetic stores of 10³, 10⁴ and 10⁵ definitions. Source: `run.sh`. Results: [[Benchmark Results]] (B-15).
>
> Sources: [[Benchmark Plan]] B-15; Moonkale `packages/sources/sophia/examples/edit_loop.rs` (Moonkale `26752448`); `crates/sophia-interface/examples/edit_timing.rs` ([[edit_timing]]), the same steps without Moonkale.

For each store, the edited definition is the most-referenced one whose text has an integer literal (bumped by one) or a hole `?(Int64)` (filled with `1`), so the id changes. Each run alternates the original and the edited text; 200 runs. The store is copied first; a copy on btrfs starts with a cold page cache, so pre-read it (`cat file > /dev/null`) for numbers that are not about the disk.
