//! `sophia-mcp --store <path> [--backend sqlite|redb|lmdb|duckdb] [--worker <path>]
//! [--max-memory <MiB>] [--reader <name>] [--allow-edits] [--allow-removals]`:
//! the agent server on stdio. See `sophia-mcp.md`.

use std::path::PathBuf;
use std::process::ExitCode;

use sophia_mcp::{Config, Kind, SophiaServer};

const USAGE: &str = "usage: sophia-mcp --store <path> [--backend sqlite|redb|lmdb|duckdb] [--worker <path>] [--max-memory <MiB>] [--reader <name>] [--allow-edits] [--allow-removals]";

fn main() -> ExitCode {
    let config = match parse(std::env::args().skip(1)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("sophia-mcp: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let server = match SophiaServer::open(config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("sophia-mcp: {e}");
            return ExitCode::from(1);
        }
    };
    let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("sophia-mcp: {e}");
            return ExitCode::from(1);
        }
    };
    match rt.block_on(server.serve_stdio()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sophia-mcp: {e}");
            ExitCode::from(1)
        }
    }
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Config, String> {
    let (mut store, mut worker, mut mib, mut reader) = (None, None, None, None);
    let mut backend = Kind::Sqlite;
    let (mut edits, mut removals) = (false, false);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--store" => store = Some(PathBuf::from(value()?)),
            "--backend" => {
                let b = value()?;
                backend = Kind::parse(&b).ok_or_else(|| format!("--backend must be sqlite, redb, lmdb or duckdb, not {b:?}"))?;
            }
            "--worker" => worker = Some(PathBuf::from(value()?)),
            "--max-memory" => mib = Some(value()?.parse::<u64>().map_err(|e| format!("--max-memory: {e}"))?),
            "--reader" => reader = Some(value()?),
            "--allow-edits" => edits = true,
            "--allow-removals" => removals = true,
            "-h" | "--help" => return Err("help".into()),
            _ => return Err(format!("unknown argument {a}")),
        }
    }
    let store = store.ok_or("--store is required")?;
    // By default the worker sits beside this binary.
    let worker = match worker {
        Some(w) => w,
        None => std::env::current_exe().map_err(|e| e.to_string())?.with_file_name("sophia-mcp-worker"),
    };
    let mut c = Config::new(store, worker);
    c.backend = backend;
    if let Some(m) = mib {
        c.max_memory = m.checked_mul(1 << 20).ok_or("--max-memory is too large")?;
    }
    if let Some(r) = reader {
        c.reader = r;
    }
    c.allow_edits = edits;
    c.allow_removals = removals;
    Ok(c)
}
