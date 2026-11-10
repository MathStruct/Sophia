//! Results of read-only text queries (SPEC-001 §11, SPEC-004 §5). See
//! `query.md`.

/// One cell of a query result.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

/// Rows from a text query.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    /// The row cap or the byte budget cut the result.
    pub truncated: bool,
}

/// Limits for a read-only query.
#[derive(Debug, Clone, Copy)]
pub struct QueryLimits {
    pub max_rows: usize,
    /// Virtual-machine steps, in units of 1000, before the query is aborted.
    pub max_steps_k: u32,
    /// Bytes of result values before the result is cut (`truncated`). It
    /// also bounds every single value the query computes: SQLite refuses a
    /// longer string or blob (`SQLITE_LIMIT_LENGTH`, at most 16 MiB), and
    /// DuckDB runs under `memory_limit` (Fable's review, S-3).
    pub max_bytes: usize,
}

impl Default for QueryLimits {
    fn default() -> Self {
        QueryLimits { max_rows: 10_000, max_steps_k: 100_000, max_bytes: 64 << 20 }
    }
}

/// The largest single value a guarded query may compute: `max_bytes`, at
/// most 16 MiB.
pub fn max_value_bytes(limits: &QueryLimits) -> usize {
    limits.max_bytes.min(16 << 20)
}

impl Value {
    /// The bytes this value adds to a result, for the byte budget.
    pub fn size(&self) -> usize {
        match self {
            Value::Text(s) => s.len(),
            Value::Blob(b) => b.len(),
            _ => 8,
        }
    }
}

/// A backend that answers read-only `sql` queries (SQLite, DuckDB).
pub trait ReadOnlyQuery {
    fn query_readonly(&self, sql: &str, limits: QueryLimits) -> Result<Table, crate::StoreError>;
}

/// The views a read-only `sql` query may use (SPEC-001 §11).
pub const VIEWS: [&str; 9] =
    ["definitions", "names", "refs", "nominals", "relation", "lineage", "annotations", "facts", "log"];
