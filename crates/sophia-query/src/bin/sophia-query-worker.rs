//! The worker process of [`sophia_query::bounded`]: runs one query under a
//! hard memory bound (B-017). See `sophia-query-worker.md`.

#[cfg(unix)]
fn main() -> std::process::ExitCode {
    sophia_query::bounded::worker_main()
}

#[cfg(not(unix))]
fn main() {
    eprintln!("sophia-query-worker needs a Unix system");
    std::process::exit(2);
}
