#implementation

> The worker process of [[bounded]]: runs one query under a hard memory bound (B-017). Source: `sophia-query-worker.rs`.
>
> Sources: [[bounded]]. code: `sophia-query-worker.rs`

`sophia-query-worker <max-memory-bytes>` reads one request on stdin and writes one response on stdout (the protocol is private to `bounded.rs`; the caller and the worker must come from the same build). It exits 0 when it wrote a response, rows or error, and 2 on a bad invocation. Its whole `main` is `bounded::worker_main`. It is built with the backends of `sophia-query`'s features; a request for a backend it lacks answers `Unsupported`.
