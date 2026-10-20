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
    /// The row cap cut the result.
    pub truncated: bool,
}

/// Limits for a read-only query.
#[derive(Debug, Clone, Copy)]
pub struct QueryLimits {
    pub max_rows: usize,
    /// Virtual-machine steps, in units of 1000, before the query is aborted.
    pub max_steps_k: u32,
}

impl Default for QueryLimits {
    fn default() -> Self {
        QueryLimits { max_rows: 10_000, max_steps_k: 100_000 }
    }
}

/// A backend that answers read-only `sql` queries (SQLite, DuckDB).
pub trait ReadOnlyQuery {
    fn query_readonly(&self, sql: &str, limits: QueryLimits) -> Result<Table, crate::StoreError>;
}

/// The views a read-only `sql` query may use (SPEC-001 §11).
pub const VIEWS: [&str; 9] =
    ["definitions", "names", "refs", "nominals", "relation", "lineage", "annotations", "facts", "log"];
