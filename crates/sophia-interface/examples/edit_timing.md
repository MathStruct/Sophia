#implementation #benchmark

> The four steps of Moonkale's edit loop (fetch text, write text, the name's new id, the old id's dependants) called on `Interface` directly, without Moonkale's adapter or tokio, to separate Sophia's share of B-15. Source: `edit_timing.rs`. Results: [[Benchmark Results]] (B-15).

```sh
cargo run --release -p sophia-interface --example edit_timing -- STORE.sophia NAME EDITED.txt [RUNS]
```
