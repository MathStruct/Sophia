//! Sophia's own query language (SPEC-007): a storage-independent query IR
//! (stratified Datalog over the store's base relations), its Julia-flavoured
//! syntax, canonical form and content identity, a native executor that runs
//! on every backend, a magic-set rewrite, and a translation to SQL.
//!
//! Queries are first-class: a query is stored as a record of the language
//! `sophia-query`, has a content id like any definition, and its results are
//! identified by (query id, snapshot, parameters). See `sophia_query.md`.

pub mod canon;
pub mod check;
pub mod eval;
pub mod ir;
pub mod magic;
pub mod plan;
pub mod sql;
pub mod syntax;

use sophia_hash::canonical::{Reader, Writer};
use sophia_hash::{Digest, Record};
use sophia_store::{Backend, ReadTx, Store, StoreError, Target, Visibility};

pub use check::Checked;
pub use eval::{Limits, Stats};
pub use ir::{Names, Query, Sort, Val};
pub use syntax::Profile;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{message} (at byte {at})")]
    Parse { at: usize, message: String },
    #[error("{0}")]
    Check(String),
    /// The query is valid, but this executor or translation cannot run it.
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("limit reached: {0}")]
    Limit(String),
    #[error("codec: {0}")]
    Codec(String),
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl Error {
    pub(crate) fn parse(at: usize, message: impl Into<String>) -> Self {
        Error::Parse { at, message: message.into() }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Which program the executors run (SPEC-007 §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Strategy {
    /// The magic-set rewrite when it applies, else the query as written.
    #[default]
    Auto,
    /// The query as written: full bottom-up evaluation.
    Plain,
}

/// A program the executors can run, with its strata.
#[derive(Debug, Clone)]
pub struct Program {
    pub query: Query,
    pub pred_names: Vec<String>,
    pub strata: Vec<Vec<u32>>,
}

/// A parsed, canonical, checked query, ready to run.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub query: Query,
    pub names: Names,
    pub checked: Checked,
    /// The canonical bytes, and the content id they give.
    pub bytes: Vec<u8>,
    pub id: Digest,
    /// The magic-set rewrite, when it applies.
    pub magic: Option<Program>,
}

/// Parses, canonicalises and checks query text.
pub fn prepare_text(text: &str) -> Result<Prepared> {
    let (q, n) = syntax::parse(text)?;
    prepare(&q, &n)
}

/// Canonicalises and checks a query given as IR.
pub fn prepare(q: &Query, names: &Names) -> Result<Prepared> {
    let (query, names) = canon::canonicalise(q, names);
    let checked = check::check(&query, &names)?;
    let bytes = canon::encode(&query);
    let id = canon::query_id(&bytes);
    let magic = magic::rewrite(&query, &names).map(|(q, pred_names)| {
        let strata = check::stratify(&q).expect("the rewrite checks stratification");
        Program { query: q, pred_names, strata }
    });
    Ok(Prepared { query, names, checked, bytes, id, magic })
}

/// Rows of a result.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rows {
    pub columns: Vec<String>,
    pub sorts: Vec<Sort>,
    pub rows: Vec<Vec<Val>>,
    pub truncated: bool,
    /// Executor statistics (not part of the result; zero for cached rows).
    pub stats: Stats,
}

impl Rows {
    /// The rows sorted, for comparisons across executors.
    pub fn sorted(mut self) -> Self {
        self.rows.sort();
        self
    }

    /// As a store query table: ids as lowercase hex text, like the `sql` views.
    pub fn to_table(&self) -> sophia_store::query::Table {
        use sophia_store::query::Value;
        sophia_store::query::Table {
            columns: self.columns.clone(),
            rows: self
                .rows
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|v| match v {
                            Val::Id(d) => Value::Text(d.to_hex()),
                            Val::Text(s) => Value::Text(s.clone()),
                            Val::Int(i) => Value::Int(*i),
                        })
                        .collect()
                })
                .collect(),
            truncated: self.truncated,
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        for (c, s) in self.columns.iter().zip(&self.sorts) {
            w.str("col", c);
            w.u32("sort", *s as u32);
        }
        w.u32("truncated", self.truncated as u32);
        for r in &self.rows {
            let o = w.open("row");
            for v in r {
                match v {
                    Val::Id(d) => w.field("id", d.as_bytes()),
                    Val::Text(s) => w.str("s", s),
                    Val::Int(i) => w.u64("i", *i as u64),
                }
            }
            w.close(o);
        }
        w.finish()
    }

    fn decode(bytes: &[u8]) -> Result<Rows> {
        let e = |e: sophia_hash::canonical::ReadError| Error::Codec(e.to_string());
        let mut r = Reader::new(bytes);
        let mut out = Rows::default();
        while r.peek_tag().map_err(e)? == "col" {
            out.columns.push(r.expect_str("col").map_err(e)?.into());
            out.sorts.push(match r.expect_u32("sort").map_err(e)? {
                0 => Sort::Id,
                1 => Sort::Text,
                _ => Sort::Int,
            });
        }
        out.truncated = r.expect_u32("truncated").map_err(e)? != 0;
        while !r.is_empty() {
            let mut row = r.enter("row").map_err(e)?;
            let mut vals = Vec::new();
            while !row.is_empty() {
                let (t, p) = row.next().map_err(e)?;
                vals.push(match t {
                    "id" => Val::Id(Digest::from_slice(p).ok_or_else(|| Error::Codec("id".into()))?),
                    "s" => Val::Text(String::from_utf8(p.to_vec()).map_err(|_| Error::Codec("text".into()))?),
                    _ => Val::Int(u64::from_le_bytes(p.try_into().map_err(|_| Error::Codec("int".into()))?) as i64),
                });
            }
            out.rows.push(vals);
        }
        Ok(out)
    }
}

impl Prepared {
    pub fn print(&self, profile: Profile) -> String {
        syntax::print(&self.query, &self.names, profile)
    }

    pub fn columns(&self) -> Vec<String> {
        self.names.columns(&self.query)
    }

    /// The program a strategy runs.
    pub fn program(&self, strategy: Strategy) -> Program {
        match (strategy, &self.magic) {
            (Strategy::Auto, Some(m)) => m.clone(),
            _ => Program { query: self.query.clone(), pred_names: self.names.preds.clone(), strata: self.checked.strata.clone() },
        }
    }

    /// The program a strategy runs, printed in Datalog notation.
    pub fn explain(&self, strategy: Strategy) -> String {
        let p = self.program(strategy);
        let names = Names {
            query: self.names.query.clone(),
            params: self.names.params.clone(),
            preds: p.pred_names.clone(),
            rule_vars: Vec::new(),
            select_vars: self.names.select_vars.clone(),
        };
        syntax::print(&p.query, &names, Profile::Datalog)
    }

    /// Runs the query with the native executor, inside one read transaction.
    pub fn run(&self, tx: &dyn ReadTx, params: &[Val], limits: Limits, strategy: Strategy) -> Result<Rows> {
        let p = self.program(strategy);
        let mut exec = eval::Exec::new(tx, params, limits);
        let out = eval::run(&mut exec, &p.query, &p.strata)?;
        Ok(Rows { columns: self.columns(), sorts: self.checked.columns.clone(), rows: out.rows, truncated: out.truncated, stats: exec.stats })
    }

    /// The SQL translation (SPEC-007 §7).
    pub fn to_sql(&self, params: &[Val], strategy: Strategy) -> Result<String> {
        self.check_params(params)?;
        let p = self.program(strategy);
        sql::to_sql(&p.query, &p.strata, params)
    }

    fn check_params(&self, params: &[Val]) -> Result<()> {
        if params.len() != self.query.params.len() || params.iter().zip(&self.query.params).any(|(v, s)| v.sort() != *s) {
            return Err(Error::Check(format!(
                "the query takes parameters ({}), given ({})",
                self.query.params.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
                params.iter().map(|v| v.sort().as_str()).collect::<Vec<_>>().join(", ")
            )));
        }
        Ok(())
    }

    /// The query as a store record. Id constants the store knows become the
    /// record's references, so a stored query is a dependant of what it
    /// names, like any definition.
    pub fn record(&self, tx: &dyn ReadTx) -> Result<Record> {
        let mut refs = Vec::new();
        let mut nominal_refs = Vec::new();
        for d in canon::constants(&self.query) {
            if tx.has_record(&d)? {
                refs.push(d);
            } else if tx.nominal(&d)?.is_some() {
                nominal_refs.push(d);
            }
        }
        Ok(canon::record(&self.query, &self.names, refs, nominal_refs))
    }

    /// Prepares a stored query from its record.
    pub fn from_record(r: &Record) -> Result<Prepared> {
        if r.lang != canon::LANG {
            return Err(Error::Check(format!("{} is a {} record, not a query", r.id.short(8), r.lang)));
        }
        let q = canon::decode(&r.bytes)?;
        let names = canon::decode_names(&r.display)?;
        let p = prepare(&q, &names)?;
        if p.id != r.id {
            return Err(Error::Codec(format!("stored query {} is not in canonical form", r.id.short(8))));
        }
        Ok(p)
    }
}

/// A run with SQLite executing the SQL translation.
#[cfg(feature = "sqlite")]
pub fn run_sql(
    db: &sophia_store::sqlite::Sqlite,
    p: &Prepared,
    params: &[Val],
    limits: Limits,
    strategy: Strategy,
) -> Result<Rows> {
    use sophia_store::query::{QueryLimits, Value};
    let sql = p.to_sql(params, strategy)?;
    let ql = QueryLimits { max_rows: limits.max_rows, max_steps_k: limits.max_steps_k.min(u32::MAX as u64) as u32 };
    let t = db.query_compiled(&sql, ql)?;
    let mut rows = Vec::with_capacity(t.rows.len());
    for r in t.rows {
        let mut out = Vec::with_capacity(r.len());
        for (v, s) in r.into_iter().zip(&p.checked.columns) {
            out.push(match (v, s) {
                (Value::Blob(b), Sort::Id) => Val::Id(Digest::from_slice(&b).ok_or_else(|| Error::Codec("id".into()))?),
                (Value::Text(t), Sort::Text) => Val::Text(t),
                (Value::Int(i), Sort::Int) => Val::Int(i),
                (v, s) => return Err(Error::Codec(format!("SQL returned {v:?} for a {} column", s.as_str()))),
            });
        }
        rows.push(out);
    }
    Ok(Rows { columns: p.columns(), sorts: p.checked.columns.clone(), rows, truncated: t.truncated, stats: Stats::default() })
}

/// The meta key under which a store keeps its random instance id.
const STORE_ID: &str = "store-id";

/// The snapshot a result is identified against (SPEC-007 §4.3): the store
/// instance and its log position. v0 is coarse: any logged change, even an
/// unrelated one, gives a new snapshot (SP-010).
pub fn snapshot(tx: &dyn ReadTx) -> Result<Digest> {
    let mut w = Writer::new();
    w.str("sophia", "snapshot-v0");
    w.field("store", &tx.meta(STORE_ID)?.unwrap_or_default());
    w.u64("seq", tx.last_seq()?);
    Ok(Digest::of(&w.finish()))
}

/// Gives the store a random instance id, once, so snapshots of two stores
/// at the same log position differ.
pub fn ensure_store_id<B: Backend>(store: &Store<B>) -> Result<()> {
    if store.meta(STORE_ID)?.is_none() {
        store.set_meta(STORE_ID, &sophia_store::logic::fresh_nonce())?;
    }
    Ok(())
}

/// Whether a result came from the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cache {
    Hit,
    Miss,
}

/// Runs a query, keyed by its result id: a second run against the same
/// snapshot with the same parameters reads the stored rows instead of
/// evaluating. Cached rows live in meta rows, which are not logged, so
/// caching does not change the snapshot.
pub fn run_cached<B: Backend>(
    store: &Store<B>,
    p: &Prepared,
    params: &[Val],
    limits: Limits,
) -> Result<(Rows, Digest, Cache)> {
    ensure_store_id(store)?;
    let (rid, rows) = store.read(|tx| {
        let rid = canon::result_id(&p.id, &snapshot(tx).map_err(|e| StoreError::Backend(e.to_string()))?, params);
        let cached = tx.meta(&format!("query-result/{}", rid.to_hex()))?;
        Ok((rid, cached))
    })?;
    if let Some(bytes) = rows {
        return Ok((Rows::decode(&bytes)?, rid, Cache::Hit));
    }
    let rows = store.read(|tx| p.run(tx, params, limits, Strategy::Auto).map_err(|e| StoreError::Backend(e.to_string())))?;
    if !rows.truncated {
        store.set_meta(&format!("query-result/{}", rid.to_hex()), &rows.encode())?;
    }
    Ok((rows, rid, Cache::Miss))
}

/// Stores a query as a definition, optionally under a name. Returns its id.
pub fn save<B: Backend>(store: &Store<B>, p: &Prepared, path: Option<&str>) -> Result<Digest> {
    let r = store.read(|tx| p.record(tx).map_err(|e| StoreError::Backend(e.to_string())))?;
    let names: Vec<(String, usize, Visibility)> = path.map(|n| (n.to_string(), 0, Visibility::Public)).into_iter().collect();
    store.add(&[r], &names)?;
    Ok(p.id)
}

/// Loads a stored query by id or by name.
pub fn load<B: Backend>(store: &Store<B>, id_or_path: &str) -> Result<Prepared> {
    let id = match id_or_path.parse::<Digest>() {
        Ok(d) => d,
        Err(_) => match store.resolve(id_or_path)?.map(|n| n.target) {
            Some(Target::Content(d)) => d,
            _ => return Err(Error::Check(format!("no query named {id_or_path}"))),
        },
    };
    let r = store.record(&id)?.ok_or_else(|| Error::Check(format!("no record {}", id.short(8))))?;
    Prepared::from_record(&r)
}
