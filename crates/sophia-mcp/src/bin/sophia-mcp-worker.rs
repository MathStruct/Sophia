//! The query worker of `sophia-mcp`: [`sophia_query::bounded::worker_main`]
//! built with this crate's backends. See `sophia-mcp-worker.md`.

#[cfg(unix)]
fn main() -> std::process::ExitCode {
    sophia_query::bounded::worker_main()
}

#[cfg(not(unix))]
fn main() {
    eprintln!("sophia-mcp-worker needs a Unix system");
    std::process::exit(2);
}
