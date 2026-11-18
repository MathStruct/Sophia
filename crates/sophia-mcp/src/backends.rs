//! The store a server opens, on any of the four backends (B-064). One enum
//! implements [`Backend`] by dispatch, so the server holds one
//! `Interface<AnyBackend>` whatever the backend. See `backends.md`.

use std::path::Path;

use sophia_query::bounded::StoreFile;
use sophia_store::backend::Result;
use sophia_store::sqlite::Sqlite;
use sophia_store::{Backend, ReadTx, StoreError, WriteTx};

/// Which backend a store file is in. LMDB's store is a directory; the
/// others are one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Sqlite,
    Redb,
    Lmdb,
    DuckDb,
}

impl Kind {
    /// `sqlite`, `redb`, `lmdb` or `duckdb`.
    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "sqlite" => Kind::Sqlite,
            "redb" => Kind::Redb,
            "lmdb" => Kind::Lmdb,
            "duckdb" => Kind::DuckDb,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Sqlite => "sqlite",
            Kind::Redb => "redb",
            Kind::Lmdb => "lmdb",
            Kind::DuckDb => "duckdb",
        }
    }

    /// Whether a server may write while its worker reads. redb and DuckDB
    /// lock their file against every other process while it is open for
    /// writing, so the worker could not open it (Daniel, 2026-11-18: such
    /// servers are read-only).
    pub fn serves_writes(self) -> bool {
        matches!(self, Kind::Sqlite | Kind::Lmdb)
    }

    /// What the worker is told to open.
    pub fn store_file(self, path: &Path) -> StoreFile {
        let p = path.to_path_buf();
        match self {
            Kind::Sqlite => StoreFile::Sqlite(p),
            Kind::Redb => StoreFile::Redb(p),
            Kind::Lmdb => StoreFile::Lmdb(p),
            Kind::DuckDb => StoreFile::DuckDb(p),
        }
    }

    /// Whether `path` is there in this backend's shape (a directory for
    /// LMDB, a file otherwise). The server never creates a store.
    pub fn exists(self, path: &Path) -> bool {
        if self == Kind::Lmdb { path.join("data.mdb").is_file() } else { path.is_file() }
    }
}

/// An open store on one of the backends.
pub enum AnyBackend {
    Sqlite(Sqlite),
    #[cfg(feature = "redb")]
    Redb(sophia_store::redb_store::Redb),
    #[cfg(feature = "lmdb")]
    Lmdb(sophia_store::lmdb_store::Lmdb),
    #[cfg(feature = "duckdb")]
    DuckDb(sophia_store::duckdb_store::DuckDb),
}

fn unbuilt(k: Kind) -> String {
    format!("this sophia-mcp was built without the {} backend (cargo feature {0})", k.as_str())
}

impl AnyBackend {
    /// Opens an existing store: read-only (nothing is created or written),
    /// or read-write. The store is first opened read-only and read, so a
    /// file that is not a Sophia store (an empty one, another database) is
    /// refused before a read-write open could initialise it (Codex's review
    /// of B-064).
    pub fn open(kind: Kind, path: &Path, writable: bool) -> std::result::Result<AnyBackend, String> {
        let not_a_store = |e: String| format!("{} is not a Sophia {} store: {e}", path.display(), kind.as_str());
        let ro = Self::open_as(kind, path, false).map_err(not_a_store)?;
        ro.read(|tx| {
            tx.record_count()?;
            tx.name("sophia")?;
            tx.last_seq()?;
            tx.meta("sophia")?;
            Ok(())
        })
        .map_err(|e| not_a_store(e.to_string()))?;
        if !writable {
            return Ok(ro);
        }
        drop(ro);
        Self::open_as(kind, path, true)
    }

    fn open_as(kind: Kind, path: &Path, writable: bool) -> std::result::Result<AnyBackend, String> {
        let s = |e: StoreError| e.to_string();
        Ok(match kind {
            Kind::Sqlite if writable => AnyBackend::Sqlite(Sqlite::open(path).map_err(s)?),
            Kind::Sqlite => AnyBackend::Sqlite(Sqlite::open_read_only(path).map_err(s)?),
            #[cfg(feature = "redb")]
            Kind::Redb if !writable => AnyBackend::Redb(sophia_store::redb_store::Redb::open_read_only(path).map_err(s)?),
            #[cfg(feature = "lmdb")]
            Kind::Lmdb if writable => AnyBackend::Lmdb(sophia_store::lmdb_store::Lmdb::open(path).map_err(s)?),
            #[cfg(feature = "lmdb")]
            Kind::Lmdb => AnyBackend::Lmdb(
                sophia_store::lmdb_store::Lmdb::open_read_only(path, sophia_store::lmdb_store::Lmdb::DEFAULT_MAP_SIZE).map_err(s)?,
            ),
            #[cfg(feature = "duckdb")]
            Kind::DuckDb if !writable => AnyBackend::DuckDb(sophia_store::duckdb_store::DuckDb::open_read_only(path).map_err(s)?),
            k if writable && !k.serves_writes() => {
                return Err(format!("a {} store is served read-only: its worker cannot read while the server writes", k.as_str()));
            }
            #[allow(unreachable_patterns)]
            k => return Err(unbuilt(k)),
        })
    }
}

macro_rules! each {
    ($self:ident, $b:ident => $e:expr) => {
        match $self {
            AnyBackend::Sqlite($b) => $e,
            #[cfg(feature = "redb")]
            AnyBackend::Redb($b) => $e,
            #[cfg(feature = "lmdb")]
            AnyBackend::Lmdb($b) => $e,
            #[cfg(feature = "duckdb")]
            AnyBackend::DuckDb($b) => $e,
        }
    };
}

impl Backend for AnyBackend {
    fn name(&self) -> &'static str {
        each!(self, b => b.name())
    }

    fn read<R>(&self, f: impl FnOnce(&dyn ReadTx) -> Result<R>) -> Result<R> {
        each!(self, b => b.read(f))
    }

    fn write<R>(&self, f: impl FnOnce(&mut dyn WriteTx) -> Result<R>) -> Result<R> {
        each!(self, b => b.write(f))
    }

    fn size_on_disk(&self) -> Option<u64> {
        each!(self, b => b.size_on_disk())
    }
}
