//! A hard memory bound for untrusted queries (SR-108, B-017): the query runs
//! in a worker process whose data segment the kernel limits
//! (`RLIMIT_DATA`), so no allocation of the executor or of the backend's C
//! library can pass it. A run over the bound ends the worker, never the
//! caller, and comes back as [`Error::MemoryBound`]. See `bounded.md`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use sophia_hash::canonical::{Reader, Writer};
use sophia_store::{Backend, StoreError};

use crate::{Error, Limits, Prepared, Result, Rows, Strategy, Val, canon};

/// A store file the worker opens read-only. redb and DuckDB lock their
/// file: the worker can open them only while no other process holds them
/// open for writing. SQLite (WAL) and LMDB have no such restriction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreFile {
    Sqlite(PathBuf),
    Redb(PathBuf),
    Lmdb(PathBuf),
    DuckDb(PathBuf),
}

impl StoreFile {
    fn tag(&self) -> (u32, &Path) {
        match self {
            StoreFile::Sqlite(p) => (0, p),
            StoreFile::Redb(p) => (1, p),
            StoreFile::Lmdb(p) => (2, p),
            StoreFile::DuckDb(p) => (3, p),
        }
    }
}

/// Where the worker binary is and how much memory it may use.
#[derive(Debug, Clone)]
pub struct Bound {
    /// The `sophia-query-worker` executable.
    pub worker: PathBuf,
    /// The worker's whole data segment in bytes (heap and private
    /// anonymous mappings): the executor, the backend's own allocations and
    /// the worker's fixed overhead, a few MiB. File mappings (LMDB's map) do
    /// not count.
    pub max_memory: u64,
    /// Time allowed on top of `Limits::deadline` before the worker is killed
    /// (the executor stops itself at the deadline; this catches a worker that
    /// does not).
    pub grace: Duration,
}

impl Bound {
    pub fn new(worker: impl Into<PathBuf>, max_memory: u64) -> Self {
        Bound { worker: worker.into(), max_memory, grace: Duration::from_secs(5) }
    }
}

const STDERR_TAIL: usize = 2000;
/// What Rust's allocation-error handler prints before it aborts.
const ALLOC_FAILED: &str = "memory allocation of";

/// Runs `p` in a worker process under `bound`, on the store `store` opened
/// read-only. The rows are those [`Prepared::run`] gives (without `stats`).
///
/// - Over the memory bound: [`Error::MemoryBound`], whether the executor's
///   allocation failed (the worker aborts), the backend reported that it is
///   out of memory, or the kernel killed the worker.
/// - Past `limits.deadline` plus `bound.grace`: the worker is killed and the
///   run fails with [`Error::Limit`].
/// - Any other error of the query (parse, check, limits, store) comes back
///   as the same variant with its message.
///
/// Only on Linux, where `RLIMIT_DATA` covers anonymous mappings (since
/// 4.7); elsewhere it returns [`Error::Unsupported`].
pub fn run_bounded(bound: &Bound, store: &StoreFile, p: &Prepared, params: &[Val], limits: Limits, strategy: Strategy) -> Result<Rows> {
    if !cfg!(target_os = "linux") {
        return Err(Error::Unsupported("a hard memory bound needs Linux (RLIMIT_DATA)".into()));
    }
    let request = encode_request(store, p, params, limits, strategy);
    let mut child = Command::new(&bound.worker)
        .arg(bound.max_memory.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Worker(format!("cannot start {}: {e}", bound.worker.display())))?;
    let mut stdin = child.stdin.take().expect("piped");
    let stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");
    // The request goes in on its own thread: a worker that dies early must
    // not leave the caller blocked on a full pipe.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&request);
    });
    // Each reader reports when its pipe reaches end of file: the worker
    // closes both when it exits, so the caller wakes as soon as the answer
    // is complete instead of on a polling tick (B-066).
    let (closed_tx, closed_rx) = mpsc::channel::<()>();
    let reader = |mut pipe: Box<dyn Read + Send>, tx: mpsc::Sender<()>| {
        std::thread::spawn(move || {
            let mut v = Vec::new();
            let _ = pipe.read_to_end(&mut v);
            let _ = tx.send(());
            v
        })
    };
    let out = reader(Box::new(stdout), closed_tx.clone());
    let err = reader(Box::new(stderr), closed_tx);
    // `None` (no deadline, or one too far to represent) waits for the worker.
    let wait_until = limits.deadline.and_then(|d| Instant::now().checked_add(d)?.checked_add(bound.grace));
    let past_deadline = || wait_until.is_some_and(|t| Instant::now() >= t);
    let mut killed = false;
    let mut open = 2;
    while open > 0 {
        let r = match wait_until {
            None => closed_rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            Some(t) => closed_rx.recv_timeout(t.saturating_duration_since(Instant::now())),
        };
        match r {
            Ok(()) => open -= 1,
            Err(RecvTimeoutError::Timeout) => {
                killed = true;
                break;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    // Both pipes are closed: the worker has exited or is exiting. A worker
    // that closed them and still runs is waited for up to the deadline; the
    // pause grows from 20 µs, since an exiting worker is reaped within a few.
    let mut pause = Duration::from_micros(20);
    let status = loop {
        if killed {
            let _ = child.kill();
            break child.wait().map_err(|e| Error::Worker(e.to_string()))?;
        }
        if let Some(s) = child.try_wait().map_err(|e| Error::Worker(e.to_string()))? {
            break s;
        }
        if past_deadline() {
            killed = true;
            continue;
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(2));
    };
    // The writer is never joined, and after a kill neither are the readers:
    // a process the worker forked can hold the pipes open past its death
    // (Mistral's review of B-066), and joining would wait for it. Such a
    // thread ends when the last holder of its pipe exits.
    drop(writer);
    if killed {
        return Err(Error::Limit("wall-clock deadline: the worker was killed".into()));
    }
    let out = out.join().unwrap_or_default();
    let err = err.join().unwrap_or_default();
    let err = String::from_utf8_lossy(&err[err.len().saturating_sub(STDERR_TAIL)..]).into_owned();
    if status.success() {
        return decode_response(&out, bound.max_memory);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        // SIGABRT after Rust's allocation-error message, or SIGKILL from the
        // kernel's out-of-memory killer.
        if (status.signal() == Some(6) && err.contains(ALLOC_FAILED)) || status.signal() == Some(9) {
            return Err(Error::MemoryBound { limit: bound.max_memory });
        }
    }
    if err.contains(ALLOC_FAILED) {
        return Err(Error::MemoryBound { limit: bound.max_memory });
    }
    Err(Error::Worker(format!("worker failed ({status}): {}", err.trim())))
}

fn encode_request(store: &StoreFile, p: &Prepared, params: &[Val], limits: Limits, strategy: Strategy) -> Vec<u8> {
    let mut w = Writer::new();
    let (kind, path) = store.tag();
    w.u32("store", kind);
    w.field("path", path.as_os_str().as_encoded_bytes());
    w.field("query", &p.bytes);
    w.field("names", &canon::encode_names(&p.names));
    w.field("id", p.id.as_bytes());
    w.field("params", &Rows { rows: vec![params.to_vec()], ..Rows::default() }.encode());
    w.u64("max_rows", limits.max_rows as u64);
    w.u64("max_steps_k", limits.max_steps_k);
    w.u64("max_derived", limits.max_derived);
    w.u64("max_bytes", limits.max_bytes);
    w.u64("deadline_ms", limits.deadline.map_or(u64::MAX, |d| d.as_millis().min(u64::MAX as u128 - 1) as u64));
    w.u32("strategy", matches!(strategy, Strategy::Plain) as u32);
    w.finish()
}

fn codec(e: sophia_hash::canonical::ReadError) -> Error {
    Error::Codec(format!("worker protocol: {e}"))
}

fn u64_of(r: &mut Reader, tag: &str) -> Result<u64> {
    let b = r.expect(tag).map_err(codec)?;
    Ok(u64::from_le_bytes(b.try_into().map_err(|_| Error::Codec(format!("worker protocol: {tag}")))?))
}

struct Request {
    store: StoreFile,
    prepared: Prepared,
    params: Vec<Val>,
    limits: Limits,
    strategy: Strategy,
}

fn decode_request(bytes: &[u8]) -> Result<Request> {
    let mut r = Reader::new(bytes);
    let kind = r.expect_u32("store").map_err(codec)?;
    // The caller wrote these bytes from a `Path` on this machine; on Unix a
    // path is any bytes, UTF-8 or not.
    let path = PathBuf::from(<std::ffi::OsStr as std::os::unix::ffi::OsStrExt>::from_bytes(r.expect("path").map_err(codec)?));
    let store = match kind {
        0 => StoreFile::Sqlite(path),
        1 => StoreFile::Redb(path),
        2 => StoreFile::Lmdb(path),
        3 => StoreFile::DuckDb(path),
        k => return Err(Error::Codec(format!("worker protocol: store kind {k}"))),
    };
    let query = canon::decode(r.expect("query").map_err(codec)?)?;
    let names = canon::decode_names(r.expect("names").map_err(codec)?)?;
    let id = r.expect("id").map_err(codec)?;
    let prepared = crate::prepare(&query, &names)?;
    if prepared.id.as_bytes() != id {
        return Err(Error::Codec("worker protocol: the query is not the one sent".into()));
    }
    let params = Rows::decode(r.expect("params").map_err(codec)?)?.rows.pop().unwrap_or_default();
    let max_rows = u64_of(&mut r, "max_rows")?;
    let max_steps_k = u64_of(&mut r, "max_steps_k")?;
    let max_derived = u64_of(&mut r, "max_derived")?;
    let max_bytes = u64_of(&mut r, "max_bytes")?;
    let deadline_ms = u64_of(&mut r, "deadline_ms")?;
    let strategy = if r.expect_u32("strategy").map_err(codec)? == 1 { Strategy::Plain } else { Strategy::Auto };
    let limits = Limits {
        max_rows: usize::try_from(max_rows).unwrap_or(usize::MAX),
        max_steps_k,
        max_derived,
        max_bytes,
        deadline: (deadline_ms != u64::MAX).then(|| Duration::from_millis(deadline_ms)),
    };
    Ok(Request { store, prepared, params, limits, strategy })
}

/// Error kinds on the wire.
const KINDS: [&str; 8] = ["parse", "check", "unsupported", "limit", "codec", "store", "memory", "worker"];

fn encode_error(e: &Error) -> Vec<u8> {
    let (kind, at, msg) = match e {
        Error::Parse { at, message } => (0, *at as u64, message.clone()),
        Error::Check(m) => (1, 0, m.clone()),
        Error::Unsupported(m) => (2, 0, m.clone()),
        Error::Limit(m) => (3, 0, m.clone()),
        Error::Codec(m) => (4, 0, m.clone()),
        Error::Store(s) => (5, 0, s.to_string()),
        Error::MemoryBound { .. } => (6, 0, String::new()),
        Error::Worker(m) => (7, 0, m.clone()),
    };
    let mut w = Writer::new();
    w.str("error", KINDS[kind]);
    w.u64("at", at);
    w.str("message", &msg);
    w.finish()
}

fn decode_response(bytes: &[u8], limit: u64) -> Result<Rows> {
    let mut r = Reader::new(bytes);
    match r.peek_tag().map_err(codec)? {
        "rows" => Rows::decode(r.expect("rows").map_err(codec)?),
        "error" => {
            let kind = r.expect_str("error").map_err(codec)?.to_string();
            let at = u64_of(&mut r, "at")?;
            let m = r.expect_str("message").map_err(codec)?.to_string();
            Err(match kind.as_str() {
                "parse" => Error::Parse { at: at as usize, message: m },
                "check" => Error::Check(m),
                "unsupported" => Error::Unsupported(m),
                "limit" => Error::Limit(m),
                "codec" => Error::Codec(m),
                // The store error's variant is not kept, only its message.
                "store" => Error::Store(StoreError::Backend(m)),
                "memory" => Error::MemoryBound { limit },
                _ => Error::Worker(m),
            })
        }
        t => Err(Error::Codec(format!("worker protocol: response {t}"))),
    }
}

/// Whether a backend's error says it ran out of memory: SQLite's
/// `SQLITE_NOMEM` ("out of memory"), DuckDB's "Out of Memory Error".
fn out_of_memory(e: &Error) -> bool {
    matches!(e, Error::Store(_)) && e.to_string().to_ascii_lowercase().contains("out of memory")
}

/// The worker's `main`: takes the memory bound in bytes as its only
/// argument, limits its own data segment to it, reads one request from
/// stdin and writes one response to stdout. Exits 0 when it wrote a
/// response (rows or an error), 2 on a bad invocation.
pub fn worker_main() -> ExitCode {
    let Some(max) = std::env::args().nth(1).and_then(|a| a.parse::<u64>().ok()) else {
        eprintln!("usage: sophia-query-worker <max-memory-bytes> < request");
        return ExitCode::from(2);
    };
    if let Err(e) = limit_data(max) {
        eprintln!("sophia-query-worker: cannot set the memory bound: {e}");
        return ExitCode::from(2);
    }
    let mut input = Vec::new();
    let response = match std::io::stdin().read_to_end(&mut input) {
        // A request larger than the bound: reading it fails, it does not abort.
        Err(e) if e.kind() == std::io::ErrorKind::OutOfMemory => encode_error(&Error::MemoryBound { limit: max }),
        Err(e) => {
            eprintln!("sophia-query-worker: {e}");
            return ExitCode::from(2);
        }
        Ok(_) => respond(&input, max),
    };
    let mut out = std::io::stdout().lock();
    if out.write_all(&response).and_then(|()| out.flush()).is_err() {
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

fn respond(input: &[u8], max: u64) -> Vec<u8> {
    match decode_request(input).and_then(|req| serve(&req)) {
        Ok(rows) => {
            let mut w = Writer::new();
            w.field("rows", &rows.encode());
            w.finish()
        }
        Err(e) if out_of_memory(&e) => encode_error(&Error::MemoryBound { limit: max }),
        Err(e) => encode_error(&e),
    }
}

#[cfg(target_os = "linux")]
fn limit_data(max: u64) -> std::io::Result<()> {
    let lim = libc::rlimit { rlim_cur: max as libc::rlim_t, rlim_max: max as libc::rlim_t };
    // SAFETY: `setrlimit` reads the struct it is given and nothing else.
    if unsafe { libc::setrlimit(libc::RLIMIT_DATA, &lim) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn limit_data(_max: u64) -> std::io::Result<()> {
    Err(std::io::Error::other("a hard memory bound needs Linux (RLIMIT_DATA)"))
}

fn serve(req: &Request) -> Result<Rows> {
    let f = |tx: &dyn sophia_store::ReadTx| req.prepared.run(tx, &req.params, req.limits, req.strategy);
    match &req.store {
        #[cfg(feature = "sqlite")]
        StoreFile::Sqlite(p) => read(&sophia_store::sqlite::Sqlite::open_read_only(p)?, f),
        #[cfg(feature = "redb")]
        StoreFile::Redb(p) => read(&sophia_store::redb_store::Redb::open_read_only(p)?, f),
        #[cfg(feature = "lmdb")]
        StoreFile::Lmdb(p) => {
            read(&sophia_store::lmdb_store::Lmdb::open_read_only(p, sophia_store::lmdb_store::Lmdb::DEFAULT_MAP_SIZE)?, f)
        }
        #[cfg(feature = "duckdb")]
        StoreFile::DuckDb(p) => read(&sophia_store::duckdb_store::DuckDb::open_read_only(p)?, f),
        #[allow(unreachable_patterns)]
        other => Err(Error::Unsupported(format!("this worker was built without the backend for {other:?}"))),
    }
}

/// One read transaction on `b`, keeping the query's own error.
fn read<B: Backend>(b: &B, f: impl FnOnce(&dyn sophia_store::ReadTx) -> Result<Rows>) -> Result<Rows> {
    let mut failure = None;
    let out = b.read(|tx| {
        f(tx).map_err(|e| {
            let msg = e.to_string();
            failure = Some(e);
            StoreError::Backend(msg)
        })
    });
    out.map_err(|e| failure.take().unwrap_or(Error::Store(e)))
}
