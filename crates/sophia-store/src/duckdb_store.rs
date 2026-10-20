//! The DuckDB backend: the SQLite backend's tables and views on DuckDB's
//! columnar engine, for analytical queries and bulk scans (SPEC-001 §12,
//! [[Database Comparison]]). See `duckdb_store.md`.

use std::path::Path;
use std::sync::Mutex;

use duckdb::{Connection, OptionalExt, Transaction, params};
use sophia_hash::{Digest, NominalId, Record, Status};

use crate::backend::{Backend, ReadTx, Result, WriteTx};
use crate::codec::{decode_inputs, encode_inputs};
use crate::error::StoreError;
use crate::query::{QueryLimits, Table, Value};
use crate::types::*;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS t_definition(
  id BLOB PRIMARY KEY, lang TEXT NOT NULL, normaliser TEXT NOT NULL, kind TEXT NOT NULL,
  component BLOB NOT NULL, idx INTEGER NOT NULL, size INTEGER NOT NULL,
  bytes BLOB NOT NULL, display BLOB NOT NULL, status TEXT NOT NULL,
  refs BLOB NOT NULL, nominal_refs BLOB NOT NULL);
CREATE INDEX IF NOT EXISTS t_definition_component ON t_definition(component, idx);
CREATE INDEX IF NOT EXISTS t_definition_kind ON t_definition(kind);
CREATE TABLE IF NOT EXISTS t_ref(src BLOB NOT NULL, dst BLOB NOT NULL, PRIMARY KEY(src, dst));
CREATE INDEX IF NOT EXISTS t_ref_dst ON t_ref(dst);
CREATE TABLE IF NOT EXISTS t_nominal_ref(src BLOB NOT NULL, dst BLOB NOT NULL, PRIMARY KEY(src, dst));
CREATE TABLE IF NOT EXISTS t_nominal(
  id BLOB PRIMARY KEY, lang TEXT NOT NULL, kind TEXT NOT NULL, label TEXT NOT NULL,
  created_seq INTEGER NOT NULL, version INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS t_relation(
  nominal BLOB NOT NULL, member BLOB NOT NULL, added_seq INTEGER NOT NULL, removed_seq INTEGER,
  PRIMARY KEY(nominal, member));
CREATE TABLE IF NOT EXISTS t_name(
  path TEXT PRIMARY KEY, target_kind TEXT NOT NULL, target BLOB NOT NULL,
  visibility TEXT NOT NULL, seq INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS t_name_target ON t_name(target);
CREATE TABLE IF NOT EXISTS t_lineage(
  lineage BLOB NOT NULL, id BLOB NOT NULL, seq INTEGER NOT NULL, supersedes BLOB,
  PRIMARY KEY(lineage, id));
CREATE INDEX IF NOT EXISTS t_lineage_id ON t_lineage(id);
CREATE TABLE IF NOT EXISTS t_annotation(
  id BLOB PRIMARY KEY, kind TEXT NOT NULL, subjects BLOB NOT NULL, payload BLOB NOT NULL,
  checker TEXT, checker_version TEXT, status TEXT NOT NULL, inputs BLOB NOT NULL);
CREATE TABLE IF NOT EXISTS t_annotation_subject(subject BLOB NOT NULL, id BLOB NOT NULL, PRIMARY KEY(subject, id));
CREATE TABLE IF NOT EXISTS t_fact(
  id BLOB PRIMARY KEY, kind TEXT NOT NULL, subject BLOB NOT NULL, value BLOB NOT NULL,
  "by" TEXT NOT NULL, by_version TEXT NOT NULL, inputs BLOB NOT NULL, derived_at INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS t_fact_subject ON t_fact(subject);
CREATE TABLE IF NOT EXISTS t_log(
  seq INTEGER PRIMARY KEY, op TEXT NOT NULL, subject TEXT NOT NULL, old BLOB, new BLOB,
  actor TEXT NOT NULL, logged_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS t_meta(key TEXT PRIMARY KEY, value BLOB NOT NULL);

CREATE VIEW IF NOT EXISTS definitions AS
  SELECT lower(hex(id)) AS id, lang, kind, status, normaliser, lower(hex(component)) AS component_id,
         idx, size AS component_size, octet_length(bytes) AS bytes FROM t_definition;
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
  SELECT lower(hex(id)) AS id, kind, lower(hex(subject)) AS subject_id, "by", by_version, derived_at FROM t_fact;
CREATE VIEW IF NOT EXISTS log AS
  SELECT seq, op, subject, lower(hex(old)) AS old_id, lower(hex(new)) AS new_id, actor, logged_at AS at FROM t_log;
"#;

fn u(r: &duckdb::Row, i: usize) -> duckdb::Result<u64> {
    Ok(r.get::<_, i64>(i)? as u64)
}

fn be(e: duckdb::Error) -> StoreError {
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

/// The DuckDB backend.
pub struct DuckDb {
    conn: Mutex<Connection>,
    path: Option<std::path::PathBuf>,
}

impl DuckDb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path.as_ref()).map_err(be)?;
        Self::init(conn, Some(path.as_ref().to_path_buf()))
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory().map_err(be)?, None)
    }

    fn init(conn: Connection, path: Option<std::path::PathBuf>) -> Result<Self> {
        conn.execute_batch(SCHEMA).map_err(be)?;
        conn.execute_batch("SET enable_external_access = false;").map_err(be)?;
        Ok(DuckDb { conn: Mutex::new(conn), path })
    }

    /// Runs a read-only `sql` query. DuckDB has no authorizer, so the guard
    /// is structural and weaker than SQLite's:
    /// - the text must be a single statement, and is run as a subquery
    ///   (`SELECT * FROM (<query>)`), which admits only queries;
    /// - it runs inside a transaction that is always rolled back;
    /// - external access (files, extensions) is switched off for the
    ///   connection;
    /// - a timer interrupts it after `max_steps_k` milliseconds;
    /// - rows past `max_rows` set `truncated`.
    ///
    /// It does **not** restrict the query to the documented views: tables can
    /// be read too (read-only). See `duckdb_store.md`.
    pub fn query_readonly(&self, sql: &str, limits: QueryLimits) -> Result<Table> {
        let body = sql.trim().trim_end_matches(';');
        if body.contains(';') {
            return Err(StoreError::QueryRefused("one statement only".into()));
        }
        let mut conn = self.conn.lock().unwrap();
        let handle = conn.interrupt_handle();
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let timer = {
            let done = done.clone();
            let ms = limits.max_steps_k as u64;
            std::thread::spawn(move || {
                let t = std::time::Instant::now();
                while t.elapsed().as_millis() < ms as u128 {
                    if done.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
                if !done.load(std::sync::atomic::Ordering::Relaxed) {
                    handle.interrupt();
                }
            })
        };
        let tx = conn.transaction().map_err(be)?;
        let result = (|| -> Result<Table> {
            let wrapped = format!("SELECT * FROM ({body}) AS sophia_query LIMIT {}", limits.max_rows + 1);
            let mut stmt = tx.prepare(&wrapped).map_err(|e| StoreError::QueryRefused(e.to_string()))?;
            let mut rows = stmt.query([]).map_err(|e| StoreError::QueryRefused(e.to_string()))?;
            let mut out_rows = Vec::new();
            let mut ncols = 0;
            while let Some(row) = rows.next().map_err(|e| StoreError::QueryRefused(e.to_string()))? {
                ncols = row.as_ref().column_count();
                let mut out = Vec::with_capacity(ncols);
                for i in 0..ncols {
                    use duckdb::types::Value as V;
                    out.push(match row.get::<_, V>(i).map_err(be)? {
                        V::Null => Value::Null,
                        V::Boolean(b) => Value::Int(b as i64),
                        V::TinyInt(v) => Value::Int(v as i64),
                        V::SmallInt(v) => Value::Int(v as i64),
                        V::Int(v) => Value::Int(v as i64),
                        V::BigInt(v) => Value::Int(v),
                        V::UTinyInt(v) => Value::Int(v as i64),
                        V::USmallInt(v) => Value::Int(v as i64),
                        V::UInt(v) => Value::Int(v as i64),
                        V::UBigInt(v) => Value::Int(v as i64),
                        V::Float(v) => Value::Real(v as f64),
                        V::Double(v) => Value::Real(v),
                        V::Text(s) => Value::Text(s),
                        V::Blob(b) => Value::Blob(b),
                        other => Value::Text(format!("{other:?}")),
                    });
                }
                out_rows.push(out);
            }
            drop(rows);
            let columns: Vec<String> = stmt.column_names();
            let _ = ncols;
            let truncated = out_rows.len() > limits.max_rows;
            out_rows.truncate(limits.max_rows);
            Ok(Table { columns, rows: out_rows, truncated })
        })();
        let _ = tx.rollback();
        done.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = timer.join();
        result
    }
}

impl crate::query::ReadOnlyQuery for DuckDb {
    fn query_readonly(&self, sql: &str, limits: QueryLimits) -> Result<Table> {
        DuckDb::query_readonly(self, sql, limits)
    }
}

impl Backend for DuckDb {
    fn name(&self) -> &'static str {
        "duckdb"
    }

    fn read<R>(&self, f: impl FnOnce(&dyn ReadTx) -> Result<R>) -> Result<R> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(be)?;
        let r = f(&DuckTx { tx: &tx });
        tx.rollback().map_err(be)?;
        r
    }

    fn write<R>(&self, f: impl FnOnce(&mut dyn WriteTx) -> Result<R>) -> Result<R> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(be)?;
        let r = f(&mut DuckTx { tx: &tx });
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
        conn.execute_batch("CHECKPOINT;").ok()?;
        drop(conn);
        std::fs::metadata(p).ok().map(|m| m.len())
    }
}

struct DuckTx<'a> {
    tx: &'a Transaction<'a>,
}

/// A `t_definition` row before its digests are checked.
type RecordRow = (Vec<u8>, String, String, String, Vec<u8>, u32, u32, Vec<u8>, Vec<u8>, String, Vec<u8>, Vec<u8>);

const RECORD_COLS: &str = "id, lang, normaliser, kind, component, idx, size, bytes, display, status, refs, nominal_refs";

impl DuckTx<'_> {
    fn record_row(&self, row: &duckdb::Row) -> duckdb::Result<RecordRow> {
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

    fn name_row(row: &duckdb::Row) -> duckdb::Result<(String, String, Vec<u8>, String, u64)> {
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

impl ReadTx for DuckTx<'_> {
    fn record(&self, id: &Digest) -> Result<Option<Record>> {
        let mut st = self.tx.prepare_cached(&format!("SELECT {RECORD_COLS} FROM t_definition WHERE id = $1")).map_err(be)?;
        let row = st.query_row([id.as_bytes().as_slice()], |r| self.record_row(r)).optional().map_err(be)?;
        row.map(|r| self.finish_record(r)).transpose()
    }

    fn has_record(&self, id: &Digest) -> Result<bool> {
        let mut st = self.tx.prepare_cached("SELECT 1 FROM t_definition WHERE id = $1").map_err(be)?;
        st.exists([id.as_bytes().as_slice()]).map_err(be)
    }

    fn component(&self, component: &Digest) -> Result<Vec<Record>> {
        let mut st = self
            .tx
            .prepare_cached(&format!("SELECT {RECORD_COLS} FROM t_definition WHERE component = $1 ORDER BY idx, id"))
            .map_err(be)?;
        let rows: Vec<_> = st.query_map([component.as_bytes().as_slice()], |r| self.record_row(r)).map_err(be)?.collect();
        rows.into_iter().map(|r| self.finish_record(r.map_err(be)?)).collect()
    }

    fn dependants(&self, id: &Digest) -> Result<Vec<Digest>> {
        let mut st = self.tx.prepare_cached("SELECT src FROM t_ref WHERE dst = $1 ORDER BY src").map_err(be)?;
        let v: Vec<_> = st.query_map([id.as_bytes().as_slice()], |r| r.get::<_, Vec<u8>>(0)).map_err(be)?.collect();
        v.into_iter().map(|b| dg(b.map_err(be)?)).collect()
    }

    fn record_count(&self) -> Result<u64> {
        self.tx.query_row("SELECT count(*) FROM t_definition", [], |r| u(r, 0)).map_err(be)
    }

    fn records_of_kind(&self, kind: Option<&str>, limit: usize) -> Result<Vec<Digest>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT id FROM t_definition WHERE (CAST($1 AS VARCHAR) IS NULL OR kind = $1) ORDER BY id LIMIT $2")
            .map_err(be)?;
        let v: Vec<_> = st.query_map(params![kind, limit.min(i64::MAX as usize) as i64], |r| r.get::<_, Vec<u8>>(0)).map_err(be)?.collect();
        v.into_iter().map(|b| dg(b.map_err(be)?)).collect()
    }

    fn name(&self, path: &str) -> Result<Option<NameEntry>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT path, target_kind, target, visibility, seq FROM t_name WHERE path = $1")
            .map_err(be)?;
        st.query_row([path], Self::name_row).optional().map_err(be)?.map(Self::finish_name).transpose()
    }

    fn names_under(&self, prefix: &str, limit: usize) -> Result<Vec<NameEntry>> {
        let mut st = self
            .tx
            .prepare_cached(
                "SELECT path, target_kind, target, visibility, seq FROM t_name
                 WHERE $1 = '' OR path = $1 OR starts_with(path, $1 || '.')
                 ORDER BY path LIMIT $2",
            )
            .map_err(be)?;
        let v: Vec<_> = st.query_map(params![prefix, limit.min(i64::MAX as usize) as i64], Self::name_row).map_err(be)?.collect();
        v.into_iter().map(|r| Self::finish_name(r.map_err(be)?)).collect()
    }

    fn names_of(&self, target: &Digest) -> Result<Vec<NameEntry>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT path, target_kind, target, visibility, seq FROM t_name WHERE target = $1 ORDER BY path")
            .map_err(be)?;
        let v: Vec<_> = st.query_map([target.as_bytes().as_slice()], Self::name_row).map_err(be)?.collect();
        v.into_iter().map(|r| Self::finish_name(r.map_err(be)?)).collect()
    }

    fn last_seq(&self) -> Result<u64> {
        self.tx.query_row("SELECT coalesce(max(seq), 0) FROM t_log", [], |r| u(r, 0)).map_err(be)
    }

    fn log_since(&self, seq: u64, limit: usize) -> Result<Vec<LogEntry>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT seq, op, subject, old, new, actor, logged_at FROM t_log WHERE seq > $1 ORDER BY seq LIMIT $2")
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
            .prepare_cached("SELECT lang, kind, label, created_seq, version FROM t_nominal WHERE id = $1")
            .map_err(be)?;
        st.query_row([id.as_bytes().as_slice()], |r| {
            Ok(Nominal { id: *id, lang: r.get(0)?, kind: r.get(1)?, label: r.get(2)?, created_seq: u(r, 3)?, version: u(r, 4)? })
        })
        .optional()
        .map_err(be)
    }

    fn relation(&self, id: &NominalId) -> Result<Vec<RelationEntry>> {
        let mut st = self
            .tx
            .prepare_cached(
                "SELECT member, added_seq FROM t_relation WHERE nominal = $1 AND removed_seq IS NULL ORDER BY added_seq, member",
            )
            .map_err(be)?;
        let v: Vec<_> =
            st.query_map([id.as_bytes().as_slice()], |r| Ok((r.get::<_, Vec<u8>>(0)?, u(r, 1)?))).map_err(be)?.collect();
        v.into_iter()
            .map(|r| {
                let (m, a) = r.map_err(be)?;
                Ok(RelationEntry { nominal: *id, member: dg(m)?, added_seq: a, removed_seq: None })
            })
            .collect()
    }

    fn lineages_of(&self, id: &Digest) -> Result<Vec<Digest>> {
        let mut st = self.tx.prepare_cached("SELECT lineage FROM t_lineage WHERE id = $1 ORDER BY lineage").map_err(be)?;
        let v: Vec<_> = st.query_map([id.as_bytes().as_slice()], |r| r.get::<_, Vec<u8>>(0)).map_err(be)?.collect();
        v.into_iter().map(|b| dg(b.map_err(be)?)).collect()
    }

    fn lineage(&self, lineage: &Digest) -> Result<Vec<LineageEntry>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT id, seq, supersedes FROM t_lineage WHERE lineage = $1 ORDER BY seq, id")
            .map_err(be)?;
        let v: Vec<_> = st
            .query_map([lineage.as_bytes().as_slice()], |r| {
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

    fn annotations_of(&self, subject: &Digest) -> Result<Vec<Annotation>> {
        let mut st = self
            .tx
            .prepare_cached(
                "SELECT a.id, a.kind, a.subjects, a.payload, a.checker, a.checker_version, a.status, a.inputs
                 FROM t_annotation a JOIN t_annotation_subject s ON s.id = a.id WHERE s.subject = $1 ORDER BY a.id",
            )
            .map_err(be)?;
        let v: Vec<_> = st
            .query_map([subject.as_bytes().as_slice()], |r| {
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
                "SELECT id, kind, value, \"by\", by_version, inputs, derived_at FROM t_fact WHERE subject = $1 ORDER BY derived_at, id",
            )
            .map_err(be)?;
        let v: Vec<_> = st
            .query_map([subject.as_bytes().as_slice()], |r| {
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
        self.tx.query_row("SELECT value FROM t_meta WHERE key = $1", [key], |r| r.get(0)).optional().map_err(be)
    }

    fn meta_prefix(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let mut st = self
            .tx
            .prepare_cached("SELECT key, value FROM t_meta WHERE starts_with(key, $1) ORDER BY key")
            .map_err(be)?;
        let v: Vec<_> = st.query_map([prefix], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))).map_err(be)?.collect();
        v.into_iter().map(|r| r.map_err(be)).collect()
    }

    fn kind_of(&self, id: &Digest) -> Result<Option<String>> {
        let mut st = self.tx.prepare_cached("SELECT kind FROM t_definition WHERE id = $1").map_err(be)?;
        let v: Vec<_> = st.query_map([id.as_bytes().as_slice()], |r| r.get::<_, String>(0)).map_err(be)?.collect();
        v.into_iter().next().transpose().map_err(be)
    }

    fn refs_of(&self, id: &Digest) -> Result<Vec<Digest>> {
        let mut st = self.tx.prepare_cached("SELECT dst FROM t_ref WHERE src = $1 ORDER BY dst").map_err(be)?;
        let v: Vec<_> = st.query_map([id.as_bytes().as_slice()], |r| r.get::<_, Vec<u8>>(0)).map_err(be)?.collect();
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

impl WriteTx for DuckTx<'_> {
    fn put_record(&mut self, r: &Record) -> Result<()> {
        let mut st = self
            .tx
            .prepare_cached(
                "INSERT OR IGNORE INTO t_definition(id, lang, normaliser, kind, component, idx, size, bytes, display, status, refs, nominal_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            )
            .map_err(be)?;
        let n = st
            .execute(params![
                r.id.as_bytes().as_slice(),
                r.lang,
                r.normaliser,
                r.kind,
                r.component.as_bytes().as_slice(),
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
        let mut st = self.tx.prepare_cached("INSERT OR IGNORE INTO t_ref(src, dst) VALUES ($1, $2)").map_err(be)?;
        for d in &r.refs {
            st.execute([r.id.as_bytes().as_slice(), d.as_bytes().as_slice()]).map_err(be)?;
        }
        let mut st = self.tx.prepare_cached("INSERT OR IGNORE INTO t_nominal_ref(src, dst) VALUES ($1, $2)").map_err(be)?;
        for d in &r.nominal_refs {
            st.execute([r.id.as_bytes().as_slice(), d.as_bytes().as_slice()]).map_err(be)?;
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
                "INSERT INTO t_name(path, target_kind, target, visibility, seq) VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT(path) DO UPDATE SET target_kind = excluded.target_kind, target = excluded.target, visibility = excluded.visibility, seq = excluded.seq",
            )
            .map_err(be)?
            .execute(params![n.path, kind, n.target.digest().as_bytes().as_slice(), n.visibility.as_str(), n.seq as i64])
            .map_err(be)?;
        Ok(())
    }

    fn remove_name(&mut self, path: &str) -> Result<()> {
        self.tx.prepare_cached("DELETE FROM t_name WHERE path = $1").map_err(be)?.execute([path]).map_err(be)?;
        Ok(())
    }

    fn append_log(&mut self, e: &LogEntry) -> Result<()> {
        self.tx
            .prepare_cached("INSERT INTO t_log(seq, op, subject, old, new, actor, logged_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
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
                "INSERT INTO t_nominal(id, lang, kind, label, created_seq, version) VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT(id) DO UPDATE SET version = excluded.version",
            )
            .map_err(be)?
            .execute(params![n.id.as_bytes().as_slice(), n.lang, n.kind, n.label, n.created_seq as i64, n.version as i64])
            .map_err(be)?;
        Ok(())
    }

    fn put_relation(&mut self, e: &RelationEntry) -> Result<()> {
        self.tx
            .prepare_cached(
                "INSERT INTO t_relation(nominal, member, added_seq, removed_seq) VALUES ($1, $2, $3, $4)
                 ON CONFLICT(nominal, member) DO UPDATE SET added_seq = excluded.added_seq, removed_seq = excluded.removed_seq",
            )
            .map_err(be)?
            .execute(params![e.nominal.as_bytes().as_slice(), e.member.as_bytes().as_slice(), e.added_seq as i64, e.removed_seq.map(|s| s as i64)])
            .map_err(be)?;
        Ok(())
    }

    fn put_lineage(&mut self, e: &LineageEntry) -> Result<()> {
        self.tx
            .prepare_cached("INSERT OR IGNORE INTO t_lineage(lineage, id, seq, supersedes) VALUES ($1, $2, $3, $4)")
            .map_err(be)?
            .execute(params![e.lineage.as_bytes().as_slice(), e.id.as_bytes().as_slice(), e.seq as i64, e.supersedes.map(|d| d.0.to_vec())])
            .map_err(be)?;
        Ok(())
    }

    fn put_annotation(&mut self, a: &Annotation) -> Result<()> {
        self.tx
            .prepare_cached(
                "INSERT INTO t_annotation(id, kind, subjects, payload, checker, checker_version, status, inputs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT(id) DO UPDATE SET status = excluded.status, inputs = excluded.inputs",
            )
            .map_err(be)?
            .execute(params![
                a.id.as_bytes().as_slice(),
                a.kind,
                digests_blob(&a.subjects),
                a.payload,
                a.checker,
                a.checker_version,
                a.status,
                digests_blob(&a.inputs)
            ])
            .map_err(be)?;
        let mut st = self.tx.prepare_cached("INSERT OR IGNORE INTO t_annotation_subject(subject, id) VALUES ($1, $2)").map_err(be)?;
        for s in &a.subjects {
            st.execute([s.as_bytes().as_slice(), a.id.as_bytes().as_slice()]).map_err(be)?;
        }
        Ok(())
    }

    fn put_fact(&mut self, f: &Fact) -> Result<()> {
        self.tx
            .prepare_cached(
                "INSERT OR IGNORE INTO t_fact(id, kind, subject, value, \"by\", by_version, inputs, derived_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .map_err(be)?
            .execute(params![
                f.id.as_bytes().as_slice(),
                f.kind,
                f.subject.as_bytes().as_slice(),
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
            .prepare_cached("INSERT INTO t_meta(key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
            .map_err(be)?
            .execute(params![key, value])
            .map_err(be)?;
        Ok(())
    }

    fn remove_meta(&mut self, key: &str) -> Result<()> {
        self.tx.prepare_cached("DELETE FROM t_meta WHERE key = $1").map_err(be)?.execute([key]).map_err(be)?;
        Ok(())
    }
}
