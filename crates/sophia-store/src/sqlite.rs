//! The SQLite backend: real tables, the documented query views, and the
//! read-only query guard (SPEC-001 §11, §12; SPEC-004 §5). See `sqlite.md`.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sophia_hash::{Digest, NominalId, Record, Status};

use crate::backend::{Backend, ReadTx, Result, WriteTx};
use crate::codec::{decode_inputs, encode_inputs};
use crate::error::StoreError;
use crate::query::{QueryLimits, Table, VIEWS, Value};
use crate::types::*;

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
CREATE TABLE IF NOT EXISTS t_definition(
  id BLOB NOT NULL UNIQUE, lang TEXT NOT NULL, normaliser TEXT NOT NULL, kind TEXT NOT NULL,
  component BLOB NOT NULL, idx INTEGER NOT NULL, size INTEGER NOT NULL,
  bytes BLOB NOT NULL, display BLOB NOT NULL, status TEXT NOT NULL,
  refs BLOB NOT NULL, nominal_refs BLOB NOT NULL);
CREATE INDEX IF NOT EXISTS t_definition_component ON t_definition(component, idx);
CREATE INDEX IF NOT EXISTS t_definition_kind ON t_definition(kind);
CREATE TABLE IF NOT EXISTS t_ref(src BLOB NOT NULL, dst BLOB NOT NULL, PRIMARY KEY(src, dst)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS t_ref_dst ON t_ref(dst);
CREATE TABLE IF NOT EXISTS t_nominal_ref(src BLOB NOT NULL, dst BLOB NOT NULL, PRIMARY KEY(src, dst)) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS t_nominal(
  id BLOB PRIMARY KEY, lang TEXT NOT NULL, kind TEXT NOT NULL, label TEXT NOT NULL,
  created_seq INTEGER NOT NULL, version INTEGER NOT NULL) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS t_relation(
  nominal BLOB NOT NULL, member BLOB NOT NULL, added_seq INTEGER NOT NULL, removed_seq INTEGER,
  PRIMARY KEY(nominal, member)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS t_relation_member ON t_relation(member);
CREATE TABLE IF NOT EXISTS t_name(
  path TEXT PRIMARY KEY, target_kind TEXT NOT NULL, target BLOB NOT NULL,
  visibility TEXT NOT NULL, seq INTEGER NOT NULL) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS t_name_target ON t_name(target);
CREATE TABLE IF NOT EXISTS t_lineage(
  lineage BLOB NOT NULL, id BLOB NOT NULL, seq INTEGER NOT NULL, supersedes BLOB,
  PRIMARY KEY(lineage, id)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS t_lineage_id ON t_lineage(id);
CREATE TABLE IF NOT EXISTS t_annotation(
  id BLOB PRIMARY KEY, kind TEXT NOT NULL, subjects BLOB NOT NULL, payload BLOB NOT NULL,
  checker TEXT, checker_version TEXT, status TEXT NOT NULL, inputs BLOB NOT NULL) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS t_annotation_subject(subject BLOB NOT NULL, id BLOB NOT NULL, PRIMARY KEY(subject, id)) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS t_fact(
  id BLOB PRIMARY KEY, kind TEXT NOT NULL, subject BLOB NOT NULL, value BLOB NOT NULL,
  by TEXT NOT NULL, by_version TEXT NOT NULL, inputs BLOB NOT NULL, derived_at INTEGER NOT NULL) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS t_fact_subject ON t_fact(subject);
CREATE TABLE IF NOT EXISTS t_log(
  seq INTEGER PRIMARY KEY, op TEXT NOT NULL, subject TEXT NOT NULL, old BLOB, new BLOB,
  actor TEXT NOT NULL, at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS t_meta(key TEXT PRIMARY KEY, value BLOB NOT NULL) WITHOUT ROWID;

CREATE VIEW IF NOT EXISTS definitions AS
  SELECT lower(hex(id)) AS id, lang, kind, status, normaliser, lower(hex(component)) AS component_id,
         idx, size AS component_size, length(bytes) AS bytes FROM t_definition;
CREATE VIEW IF NOT EXISTS names AS
  SELECT path, lower(hex(target)) AS target_id, target_kind, visibility, seq FROM t_name;
CREATE VIEW IF NOT EXISTS refs AS
  SELECT lower(hex(src)) AS from_id, lower(hex(dst)) AS to_id FROM t_ref;
CREATE VIEW IF NOT EXISTS nominals AS
  SELECT lower(hex(id)) AS id, lang, kind, label, version FROM t_nominal;
CREATE VIEW IF NOT EXISTS relation AS
  SELECT lower(hex(nominal)) AS nominal_id, lower(hex(member)) AS member_id, added_seq AS seq
  FROM t_relation WHERE removed_seq IS NULL;
CREATE VIEW IF NOT EXISTS lineage AS
  SELECT lower(hex(lineage)) AS lineage_id, lower(hex(id)) AS id, seq, lower(hex(supersedes)) AS supersedes_id FROM t_lineage;
CREATE VIEW IF NOT EXISTS annotations AS
  SELECT lower(hex(a.id)) AS id, a.kind, lower(hex(s.subject)) AS subject_id, a.status
  FROM t_annotation a JOIN t_annotation_subject s ON s.id = a.id;
CREATE VIEW IF NOT EXISTS facts AS
  SELECT lower(hex(id)) AS id, kind, lower(hex(subject)) AS subject_id, by, by_version, derived_at FROM t_fact;
CREATE VIEW IF NOT EXISTS log AS
  SELECT seq, op, subject, lower(hex(old)) AS old_id, lower(hex(new)) AS new_id, actor, at FROM t_log;
"#;

fn u(r: &rusqlite::Row, i: usize) -> rusqlite::Result<u64> {
    Ok(r.get::<_, i64>(i)? as u64)
}

fn be(e: rusqlite::Error) -> StoreError {
    StoreError::Backend(e.to_string())
}

fn dg(b: Vec<u8>) -> Result<Digest> {
    Digest::from_slice(&b).ok_or_else(|| StoreError::Codec("digest".into()))
}

fn opt_dg(b: Option<Vec<u8>>) -> Result<Option<Digest>> {
    b.map(dg).transpose()
}

fn digests_blob(ds: &[Digest]) -> Vec<u8> {
    ds.iter().flat_map(|d| d.0).collect()
}

fn blob_digests(b: &[u8]) -> Result<Vec<Digest>> {
    if b.len() % 32 != 0 {
        return Err(StoreError::Codec("digest list".into()));
    }
    Ok(b.chunks(32).map(|c| Digest::from_slice(c).unwrap()).collect())
}

/// The SQLite backend.
pub struct Sqlite {
    conn: Mutex<Connection>,
    path: Option<std::path::PathBuf>,
}

impl Sqlite {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path.as_ref()).map_err(be)?;
        Self::init(conn, Some(path.as_ref().to_path_buf()))
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory().map_err(be)?, None)
    }

    /// Opens a store from the bytes of its file, **read-only**, in memory
    /// (SPEC-009 §4: Moonkale's web client, which has the file's bytes but
    /// no file system). Any write fails. The schema is not touched.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // wasm: `:memory:` does not open under sqlite-wasm-rs; a named
        // database in its default in-memory VFS does.
        #[cfg(target_arch = "wasm32")]
        let mut conn = Connection::open("sophia-from-bytes.db").map_err(be)?;
        #[cfg(not(target_arch = "wasm32"))]
        let mut conn = Connection::open_in_memory().map_err(be)?;
        // Sophia's files are in WAL mode (header bytes 18 and 19 are 2), and
        // a WAL database makes SQLite open `-wal` and `-shm` files even to
        // read. In memory there are none: read the copy as a rollback-journal
        // database (1, 1). A complete file needs nothing from a WAL.
        let mut copy = bytes.to_vec();
        if copy.len() > 19 && copy[18] == 2 && copy[19] == 2 {
            copy[18] = 1;
            copy[19] = 1;
        }
        conn.deserialize_read_exact(rusqlite::MAIN_DB, copy.as_slice(), copy.len(), true).map_err(be)?;
        // Sorting and grouping may need temporary storage; there is no file
        // system for it in the browser.
        conn.execute_batch("PRAGMA temp_store = MEMORY;").map_err(be)?;
        Ok(Sqlite { conn: Mutex::new(conn), path: None })
    }

    fn init(conn: Connection, path: Option<std::path::PathBuf>) -> Result<Self> {
        conn.execute_batch(SCHEMA).map_err(be)?;
        Ok(Sqlite { conn: Mutex::new(conn), path })
    }

    /// Runs a read-only `sql` query over the documented views, enforced by an
    /// authorizer (only SELECT, only the views and what they read, only pure
    /// functions), a row cap and a step budget (SPEC-004 §5).
    pub fn query_readonly(&self, sql: &str, limits: QueryLimits) -> Result<Table> {
        self.guarded(sql, limits, |table, accessor| {
            VIEWS.contains(&table) || accessor.is_some_and(|v| VIEWS.contains(&v))
        })
    }

    /// Runs SQL that Sophia generated itself, from a checked query (SPEC-007
    /// §7): it may read the base tables (`t_*`) directly, so their indexes
    /// apply, but it is otherwise guarded exactly like `query_readonly`. Not
    /// for text from users or agents.
    pub fn query_compiled(&self, sql: &str, limits: QueryLimits) -> Result<Table> {
        // The base tables, and the CTEs the translation names `p0`, `p1`, …
        // (SQLite reports a read of a CTE that a rule uses without reading a
        // column of it).
        self.guarded(sql, limits, |table, _| {
            table.starts_with("t_")
                || table.strip_prefix('p').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
    }

    fn guarded(&self, sql: &str, limits: QueryLimits, readable: fn(&str, Option<&str>) -> bool) -> Result<Table> {
        let conn = self.conn.lock().unwrap();
        conn.authorizer(Some(move |ctx: AuthContext<'_>| match ctx.action {
            AuthAction::Select | AuthAction::Recursive => Authorization::Allow,
            AuthAction::Read { table_name, .. } => {
                if readable(table_name, ctx.accessor) { Authorization::Allow } else { Authorization::Deny }
            }
            AuthAction::Function { function_name } => {
                let f = function_name.to_ascii_lowercase();
                if f.contains("load_extension") || f.starts_with("readfile") || f.starts_with("writefile") || f == "edit" {
                    Authorization::Deny
                } else {
                    Authorization::Allow
                }
            }
            _ => Authorization::Deny,
        }))
        .map_err(be)?;
        let mut steps = 0u32;
        let max = limits.max_steps_k;
        conn.progress_handler(
            1000,
            Some(move || {
                steps += 1;
                steps > max
            }),
        )
        .map_err(be)?;

        let result = (|| -> Result<Table> {
            let mut stmt = conn.prepare(sql).map_err(|e| StoreError::QueryRefused(e.to_string()))?;
            if !stmt.readonly() {
                return Err(StoreError::QueryRefused("not a read-only statement".into()));
            }
            let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
            let n = columns.len();
            let mut rows = stmt.query([]).map_err(|e| StoreError::QueryRefused(e.to_string()))?;
            let mut table = Table { columns, rows: Vec::new(), truncated: false };
            while let Some(row) = rows.next().map_err(|e| StoreError::QueryRefused(e.to_string()))? {
                if table.rows.len() == limits.max_rows {
                    table.truncated = true;
                    break;
                }
                let mut out = Vec::with_capacity(n);
                for i in 0..n {
                    use rusqlite::types::ValueRef;
                    out.push(match row.get_ref(i).map_err(be)? {
                        ValueRef::Null => Value::Null,
                        ValueRef::Integer(v) => Value::Int(v),
                        ValueRef::Real(v) => Value::Real(v),
                        ValueRef::Text(t) => Value::Text(String::from_utf8_lossy(t).into_owned()),
                        ValueRef::Blob(b) => Value::Blob(b.to_vec()),
                    });
                }
                table.rows.push(out);
            }
            Ok(table)
        })();

        conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>).map_err(be)?;
        conn.progress_handler(0, None::<fn() -> bool>).map_err(be)?;
        result
    }
}

impl crate::query::ReadOnlyQuery for Sqlite {
    fn query_readonly(&self, sql: &str, limits: QueryLimits) -> Result<Table> {
        Sqlite::query_readonly(self, sql, limits)
    }
}

impl Backend for Sqlite {
    fn name(&self) -> &'static str {
        "sqlite"
    }

    fn read<R>(&self, f: impl FnOnce(&dyn ReadTx) -> Result<R>) -> Result<R> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(be)?;
        let r = f(&SqliteTx { tx: &tx });
        tx.rollback().map_err(be)?;
        r
    }

    fn write<R>(&self, f: impl FnOnce(&mut dyn WriteTx) -> Result<R>) -> Result<R> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(be)?;
        let r = f(&mut SqliteTx { tx: &tx });
        match r {
            Ok(v) => {
                tx.commit().map_err(be)?;
                Ok(v)
            }
            Err(e) => {
                tx.rollback().map_err(be)?;
                Err(e)
            }
        }
    }

    fn size_on_disk(&self) -> Option<u64> {
        let p = self.path.as_ref()?;
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").ok()?;
        drop(conn);
        std::fs::metadata(p).ok().map(|m| m.len())
    }
}

struct SqliteTx<'a> {
    tx: &'a Transaction<'a>,
}

/// A `t_definition` row before its digests are checked.
type RecordRow = (Vec<u8>, String, String, String, Vec<u8>, u32, u32, Vec<u8>, Vec<u8>, String, Vec<u8>, Vec<u8>);

const RECORD_COLS: &str = "id, lang, normaliser, kind, component, idx, size, bytes, display, status, refs, nominal_refs";

impl SqliteTx<'_> {
    fn record_row(&self, row: &rusqlite::Row) -> rusqlite::Result<RecordRow> {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
            row.get(9)?,
            row.get(10)?,
            row.get(11)?,
        ))
    }

    fn finish_record(&self, r: RecordRow) -> Result<Record> {
        Ok(Record {
            id: dg(r.0)?,
            lang: r.1,
            normaliser: r.2,
            kind: r.3,
            component: dg(r.4)?,
            index: r.5,
            component_size: r.6,
            bytes: r.7,
            display: r.8,
            status: Status::parse(&r.9).ok_or_else(|| StoreError::Codec("status".into()))?,
            refs: blob_digests(&r.10)?,
            nominal_refs: blob_digests(&r.11)?,
        })
    }

    fn name_row(row: &rusqlite::Row) -> rusqlite::Result<(String, String, Vec<u8>, String, u64)> {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, u(row, 4)?))
    }

    fn finish_name(r: (String, String, Vec<u8>, String, u64)) -> Result<NameEntry> {
        let d = dg(r.2)?;
        let target = match r.1.as_str() {
            "content" => Target::Content(d),
            "nominal" => Target::Nominal(d),
            _ => return Err(StoreError::Codec("target kind".into())),
        };
        Ok(NameEntry {
            path: r.0,
            target,
            visibility: Visibility::parse(&r.3).ok_or_else(|| StoreError::Codec("visibility".into()))?,
            seq: r.4,
        })
    }
}

impl ReadTx for SqliteTx<'_> {
    fn record(&self, id: &Digest) -> Result<Option<Record>> {
        let mut st = self.tx.prepare_cached(&format!("SELECT {RECORD_COLS} FROM t_definition WHERE id = ?1")).map_err(be)?;
        let row = st.query_row([id.as_bytes()], |r| self.record_row(r)).optional().map_err(be)?;
        row.map(|r| self.finish_record(r)).transpose()
    }

    fn has_record(&self, id: &Digest) -> Result<bool> {
        let mut st = self.tx.prepare_cached("SELECT 1 FROM t_definition WHERE id = ?1").map_err(be)?;
        st.exists([id.as_bytes()]).map_err(be)
    }

    fn component(&self, component: &Digest) -> Result<Vec<Record>> {
        let mut st = self
            .tx
            .prepare_cached(&format!("SELECT {RECORD_COLS} FROM t_definition WHERE component = ?1 ORDER BY idx, id"))
            .map_err(be)?;
        let rows: Vec<_> = st.query_map([component.as_bytes()], |r| self.record_row(r)).map_err(be)?.collect();
        rows.into_iter().map(|r| self.finish_record(r.map_err(be)?)).collect()
    }

    fn dependants(&self, id: &Digest) -> Result<Vec<Digest>> {
        let mut st = self.tx.prepare_cached("SELECT src FROM t_ref WHERE dst = ?1 ORDER BY src").map_err(be)?;
        let v: Vec<_> = st.query_map([id.as_bytes()], |r| r.get::<_, Vec<u8>>(0)).map_err(be)?.collect();
        v.into_iter().map(|b| dg(b.map_err(be)?)).collect()
    }

    fn record_count(&self) -> Result<u64> {
        self.tx.query_row("SELECT count(*) FROM t_definition", [], |r| u(r, 0)).map_err(be)
    }

    fn records_of_kind(&self, kind: Option<&str>, limit: usize) -> Result<Vec<Digest>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT id FROM t_definition WHERE (?1 IS NULL OR kind = ?1) ORDER BY id LIMIT ?2")
            .map_err(be)?;
        let v: Vec<_> = st.query_map(params![kind, limit.min(i64::MAX as usize) as i64], |r| r.get::<_, Vec<u8>>(0)).map_err(be)?.collect();
        v.into_iter().map(|b| dg(b.map_err(be)?)).collect()
    }

    fn name(&self, path: &str) -> Result<Option<NameEntry>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT path, target_kind, target, visibility, seq FROM t_name WHERE path = ?1")
            .map_err(be)?;
        st.query_row([path], Self::name_row).optional().map_err(be)?.map(Self::finish_name).transpose()
    }

    fn names_under(&self, prefix: &str, limit: usize) -> Result<Vec<NameEntry>> {
        let mut st = self
            .tx
            .prepare_cached(
                "SELECT path, target_kind, target, visibility, seq FROM t_name
                 WHERE ?1 = '' OR path = ?1 OR (path >= ?1 || '.' AND path < ?1 || '/')
                 ORDER BY path LIMIT ?2",
            )
            .map_err(be)?;
        let v: Vec<_> = st.query_map(params![prefix, limit.min(i64::MAX as usize) as i64], Self::name_row).map_err(be)?.collect();
        v.into_iter().map(|r| Self::finish_name(r.map_err(be)?)).collect()
    }

    fn names_from(&self, base: &str, from: &str, limit: usize) -> Result<Vec<NameEntry>> {
        // `.` + 1 = `/`: every path under `ns.` sorts below `ns/`.
        let upper = base.strip_suffix('.').map(|b| format!("{b}/"));
        // One lower bound, so the index seek starts at the cursor.
        let start = if from > base { from } else { base };
        let mut st = self
            .tx
            .prepare_cached(
                "SELECT path, target_kind, target, visibility, seq FROM t_name
                 WHERE path >= ?1 AND (?2 IS NULL OR path < ?2)
                 ORDER BY path LIMIT ?3",
            )
            .map_err(be)?;
        let v: Vec<_> = st
            .query_map(params![start, upper, limit.min(i64::MAX as usize) as i64], Self::name_row)
            .map_err(be)?
            .collect();
        v.into_iter().map(|r| Self::finish_name(r.map_err(be)?)).collect()
    }

    fn names_of(&self, target: &Digest) -> Result<Vec<NameEntry>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT path, target_kind, target, visibility, seq FROM t_name WHERE target = ?1 ORDER BY path")
            .map_err(be)?;
        let v: Vec<_> = st.query_map([target.as_bytes()], Self::name_row).map_err(be)?.collect();
        v.into_iter().map(|r| Self::finish_name(r.map_err(be)?)).collect()
    }

    fn last_seq(&self) -> Result<u64> {
        self.tx.query_row("SELECT coalesce(max(seq), 0) FROM t_log", [], |r| u(r, 0)).map_err(be)
    }

    fn log_since(&self, seq: u64, limit: usize) -> Result<Vec<LogEntry>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT seq, op, subject, old, new, actor, at FROM t_log WHERE seq > ?1 ORDER BY seq LIMIT ?2")
            .map_err(be)?;
        let rows: Vec<_> = st
            .query_map(params![seq as i64, limit.min(i64::MAX as usize) as i64], |r| {
                Ok((
                    u(r, 0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<Vec<u8>>>(3)?,
                    r.get::<_, Option<Vec<u8>>>(4)?,
                    r.get::<_, String>(5)?,
                    u(r, 6)?,
                ))
            })
            .map_err(be)?
            .collect();
        rows.into_iter()
            .map(|r| {
                let r = r.map_err(be)?;
                Ok(LogEntry {
                    seq: r.0,
                    op: LogOp::parse(&r.1).ok_or_else(|| StoreError::Codec("log op".into()))?,
                    subject: r.2,
                    old: opt_dg(r.3)?,
                    new: opt_dg(r.4)?,
                    actor: r.5,
                    at: r.6,
                })
            })
            .collect()
    }

    fn nominal(&self, id: &NominalId) -> Result<Option<Nominal>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT lang, kind, label, created_seq, version FROM t_nominal WHERE id = ?1")
            .map_err(be)?;
        st.query_row([id.as_bytes()], |r| {
            Ok(Nominal { id: *id, lang: r.get(0)?, kind: r.get(1)?, label: r.get(2)?, created_seq: u(r, 3)?, version: u(r, 4)? })
        })
        .optional()
        .map_err(be)
    }

    fn relation(&self, id: &NominalId) -> Result<Vec<RelationEntry>> {
        let mut st = self
            .tx
            .prepare_cached(
                "SELECT member, added_seq FROM t_relation WHERE nominal = ?1 AND removed_seq IS NULL ORDER BY added_seq, member",
            )
            .map_err(be)?;
        let v: Vec<_> =
            st.query_map([id.as_bytes()], |r| Ok((r.get::<_, Vec<u8>>(0)?, u(r, 1)?))).map_err(be)?.collect();
        v.into_iter()
            .map(|r| {
                let (m, a) = r.map_err(be)?;
                Ok(RelationEntry { nominal: *id, member: dg(m)?, added_seq: a, removed_seq: None })
            })
            .collect()
    }

    fn lineages_of(&self, id: &Digest) -> Result<Vec<Digest>> {
        let mut st = self.tx.prepare_cached("SELECT lineage FROM t_lineage WHERE id = ?1 ORDER BY lineage").map_err(be)?;
        let v: Vec<_> = st.query_map([id.as_bytes()], |r| r.get::<_, Vec<u8>>(0)).map_err(be)?.collect();
        v.into_iter().map(|b| dg(b.map_err(be)?)).collect()
    }

    fn lineage(&self, lineage: &Digest) -> Result<Vec<LineageEntry>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT id, seq, supersedes FROM t_lineage WHERE lineage = ?1 ORDER BY seq, id")
            .map_err(be)?;
        let v: Vec<_> = st
            .query_map([lineage.as_bytes()], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, u(r, 1)?, r.get::<_, Option<Vec<u8>>>(2)?))
            })
            .map_err(be)?
            .collect();
        v.into_iter()
            .map(|r| {
                let (id, seq, sup) = r.map_err(be)?;
                Ok(LineageEntry { lineage: *lineage, id: dg(id)?, seq, supersedes: opt_dg(sup)? })
            })
            .collect()
    }

    fn memberships_of(&self, id: &Digest) -> Result<Vec<NominalId>> {
        let mut st = self.tx.prepare_cached("SELECT nominal FROM t_relation WHERE member = ?1 AND removed_seq IS NULL ORDER BY nominal").map_err(be)?;
        let v: Vec<_> = st.query_map([id.as_bytes()], |r| r.get::<_, Vec<u8>>(0)).map_err(be)?.collect();
        v.into_iter().map(|b| dg(b.map_err(be)?)).collect()
    }

    fn annotation(&self, id: &Digest) -> Result<Option<Annotation>> {
        // Through its first subject, which reuses the row decoding below.
        let mut st = self.tx.prepare_cached("SELECT subject FROM t_annotation_subject WHERE id = ?1 LIMIT 1").map_err(be)?;
        let v: Vec<_> = st.query_map([id.as_bytes()], |r| r.get::<_, Vec<u8>>(0)).map_err(be)?.collect();
        let Some(subject) = v.into_iter().next() else { return Ok(None) };
        let subject = dg(subject.map_err(be)?)?;
        Ok(self.annotations_of(&subject)?.into_iter().find(|a| a.id == *id))
    }

    fn annotations_of(&self, subject: &Digest) -> Result<Vec<Annotation>> {
        let mut st = self
            .tx
            .prepare_cached(
                "SELECT a.id, a.kind, a.subjects, a.payload, a.checker, a.checker_version, a.status, a.inputs
                 FROM t_annotation a JOIN t_annotation_subject s ON s.id = a.id WHERE s.subject = ?1 ORDER BY a.id",
            )
            .map_err(be)?;
        let v: Vec<_> = st
            .query_map([subject.as_bytes()], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, Vec<u8>>(7)?,
                ))
            })
            .map_err(be)?
            .collect();
        v.into_iter()
            .map(|r| {
                let r = r.map_err(be)?;
                Ok(Annotation {
                    id: dg(r.0)?,
                    kind: r.1,
                    subjects: blob_digests(&r.2)?,
                    payload: r.3,
                    checker: r.4,
                    checker_version: r.5,
                    status: r.6,
                    inputs: blob_digests(&r.7)?,
                })
            })
            .collect()
    }

    fn facts_of(&self, subject: &Digest) -> Result<Vec<Fact>> {
        let mut st = self
            .tx
            .prepare_cached(
                "SELECT id, kind, value, by, by_version, inputs, derived_at FROM t_fact WHERE subject = ?1 ORDER BY derived_at, id",
            )
            .map_err(be)?;
        let v: Vec<_> = st
            .query_map([subject.as_bytes()], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Vec<u8>>(5)?,
                    u(r, 6)?,
                ))
            })
            .map_err(be)?
            .collect();
        v.into_iter()
            .map(|r| {
                let r = r.map_err(be)?;
                Ok(Fact {
                    id: dg(r.0)?,
                    kind: r.1,
                    subject: *subject,
                    value: r.2,
                    by: r.3,
                    by_version: r.4,
                    inputs: decode_inputs(&r.5)?,
                    derived_at: r.6,
                })
            })
            .collect()
    }

    fn meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.tx.query_row("SELECT value FROM t_meta WHERE key = ?1", [key], |r| r.get(0)).optional().map_err(be)
    }

    fn meta_prefix(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT key, value FROM t_meta WHERE substr(key, 1, length(?1)) = ?1 ORDER BY key")
            .map_err(be)?;
        let v: Vec<_> = st.query_map([prefix], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))).map_err(be)?.collect();
        v.into_iter().map(|r| r.map_err(be)).collect()
    }

    fn kind_of(&self, id: &Digest) -> Result<Option<String>> {
        let mut st = self.tx.prepare_cached("SELECT kind FROM t_definition WHERE id = ?1").map_err(be)?;
        st.query_row([id.as_bytes()], |r| r.get(0)).optional().map_err(be)
    }

    fn refs_of(&self, id: &Digest) -> Result<Vec<Digest>> {
        let mut st = self.tx.prepare_cached("SELECT dst FROM t_ref WHERE src = ?1 ORDER BY dst").map_err(be)?;
        let v: Vec<_> = st.query_map([id.as_bytes()], |r| r.get::<_, Vec<u8>>(0)).map_err(be)?.collect();
        v.into_iter().map(|b| dg(b.map_err(be)?)).collect()
    }

    fn all_kinds(&self) -> Result<Vec<(Digest, String)>> {
        let mut st = self.tx.prepare_cached("SELECT id, kind FROM t_definition").map_err(be)?;
        let v: Vec<_> = st.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?))).map_err(be)?.collect();
        v.into_iter().map(|r| { let (i, k) = r.map_err(be)?; Ok((dg(i)?, k)) }).collect()
    }

    fn all_refs(&self) -> Result<Vec<(Digest, Digest)>> {
        let mut st = self.tx.prepare_cached("SELECT src, dst FROM t_ref").map_err(be)?;
        let v: Vec<_> = st.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))).map_err(be)?.collect();
        v.into_iter().map(|r| { let (a, b) = r.map_err(be)?; Ok((dg(a)?, dg(b)?)) }).collect()
    }
}

impl WriteTx for SqliteTx<'_> {
    fn put_record(&mut self, r: &Record) -> Result<()> {
        let mut st = self
            .tx
            .prepare_cached(
                "INSERT OR IGNORE INTO t_definition(id, lang, normaliser, kind, component, idx, size, bytes, display, status, refs, nominal_refs)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )
            .map_err(be)?;
        let n = st
            .execute(params![
                r.id.as_bytes(),
                r.lang,
                r.normaliser,
                r.kind,
                r.component.as_bytes(),
                r.index,
                r.component_size,
                r.bytes,
                r.display,
                r.status.as_str(),
                digests_blob(&r.refs),
                digests_blob(&r.nominal_refs)
            ])
            .map_err(be)?;
        if n == 0 {
            return Ok(());
        }
        let mut st = self.tx.prepare_cached("INSERT OR IGNORE INTO t_ref(src, dst) VALUES (?1, ?2)").map_err(be)?;
        for d in &r.refs {
            st.execute([r.id.as_bytes(), d.as_bytes()]).map_err(be)?;
        }
        let mut st = self.tx.prepare_cached("INSERT OR IGNORE INTO t_nominal_ref(src, dst) VALUES (?1, ?2)").map_err(be)?;
        for d in &r.nominal_refs {
            st.execute([r.id.as_bytes(), d.as_bytes()]).map_err(be)?;
        }
        Ok(())
    }

    fn put_name(&mut self, n: &NameEntry) -> Result<()> {
        let kind = match n.target {
            Target::Content(_) => "content",
            Target::Nominal(_) => "nominal",
        };
        self.tx
            .prepare_cached(
                "INSERT INTO t_name(path, target_kind, target, visibility, seq) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(path) DO UPDATE SET target_kind = ?2, target = ?3, visibility = ?4, seq = ?5",
            )
            .map_err(be)?
            .execute(params![n.path, kind, n.target.digest().as_bytes(), n.visibility.as_str(), n.seq as i64])
            .map_err(be)?;
        Ok(())
    }

    fn remove_name(&mut self, path: &str) -> Result<()> {
        self.tx.prepare_cached("DELETE FROM t_name WHERE path = ?1").map_err(be)?.execute([path]).map_err(be)?;
        Ok(())
    }

    fn append_log(&mut self, e: &LogEntry) -> Result<()> {
        self.tx
            .prepare_cached("INSERT INTO t_log(seq, op, subject, old, new, actor, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .map_err(be)?
            .execute(params![
                e.seq as i64,
                e.op.as_str(),
                e.subject,
                e.old.map(|d| d.0.to_vec()),
                e.new.map(|d| d.0.to_vec()),
                e.actor,
                e.at as i64
            ])
            .map_err(be)?;
        Ok(())
    }

    fn put_nominal(&mut self, n: &Nominal) -> Result<()> {
        self.tx
            .prepare_cached(
                "INSERT INTO t_nominal(id, lang, kind, label, created_seq, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET version = ?6",
            )
            .map_err(be)?
            .execute(params![n.id.as_bytes(), n.lang, n.kind, n.label, n.created_seq as i64, n.version as i64])
            .map_err(be)?;
        Ok(())
    }

    fn put_relation(&mut self, e: &RelationEntry) -> Result<()> {
        self.tx
            .prepare_cached(
                "INSERT INTO t_relation(nominal, member, added_seq, removed_seq) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(nominal, member) DO UPDATE SET added_seq = ?3, removed_seq = ?4",
            )
            .map_err(be)?
            .execute(params![e.nominal.as_bytes(), e.member.as_bytes(), e.added_seq as i64, e.removed_seq.map(|s| s as i64)])
            .map_err(be)?;
        Ok(())
    }

    fn put_lineage(&mut self, e: &LineageEntry) -> Result<()> {
        self.tx
            .prepare_cached("INSERT OR IGNORE INTO t_lineage(lineage, id, seq, supersedes) VALUES (?1, ?2, ?3, ?4)")
            .map_err(be)?
            .execute(params![e.lineage.as_bytes(), e.id.as_bytes(), e.seq as i64, e.supersedes.map(|d| d.0.to_vec())])
            .map_err(be)?;
        Ok(())
    }

    fn put_annotation(&mut self, a: &Annotation) -> Result<()> {
        self.tx
            .prepare_cached(
                "INSERT INTO t_annotation(id, kind, subjects, payload, checker, checker_version, status, inputs)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET status = ?7, inputs = ?8",
            )
            .map_err(be)?
            .execute(params![
                a.id.as_bytes(),
                a.kind,
                digests_blob(&a.subjects),
                a.payload,
                a.checker,
                a.checker_version,
                a.status,
                digests_blob(&a.inputs)
            ])
            .map_err(be)?;
        let mut st = self.tx.prepare_cached("INSERT OR IGNORE INTO t_annotation_subject(subject, id) VALUES (?1, ?2)").map_err(be)?;
        for s in &a.subjects {
            st.execute([s.as_bytes(), a.id.as_bytes()]).map_err(be)?;
        }
        Ok(())
    }

    fn remove_fact(&mut self, id: &Digest) -> Result<()> {
        self.tx.prepare_cached("DELETE FROM t_fact WHERE id = ?1").map_err(be)?.execute(params![id.as_bytes()]).map_err(be)?;
        Ok(())
    }

    fn put_fact(&mut self, f: &Fact) -> Result<()> {
        self.tx
            .prepare_cached(
                "INSERT OR IGNORE INTO t_fact(id, kind, subject, value, by, by_version, inputs, derived_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .map_err(be)?
            .execute(params![
                f.id.as_bytes(),
                f.kind,
                f.subject.as_bytes(),
                f.value,
                f.by,
                f.by_version,
                encode_inputs(&f.inputs),
                f.derived_at as i64
            ])
            .map_err(be)?;
        Ok(())
    }

    fn set_meta(&mut self, key: &str, value: &[u8]) -> Result<()> {
        self.tx
            .prepare_cached("INSERT INTO t_meta(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2")
            .map_err(be)?
            .execute(params![key, value])
            .map_err(be)?;
        Ok(())
    }

    fn remove_meta(&mut self, key: &str) -> Result<()> {
        self.tx.prepare_cached("DELETE FROM t_meta WHERE key = ?1").map_err(be)?.execute([key]).map_err(be)?;
        Ok(())
    }
}
