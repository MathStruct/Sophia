#implementation #benchmark

> How long Moonkale's Explorer calls take on a large store: `children` of the root and of one namespace, and `all(100000)`; best of RUNS, read-only (run it on a copy). Source: `listing_timing.rs`. Results: [[sophia_interface]] (Listing).

```sh
cargo run --release -p sophia-interface --example listing_timing -- STORE.sophia NS [RUNS]
```
