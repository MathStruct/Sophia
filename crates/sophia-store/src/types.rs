//! The store's record types (SPEC-001 §5 to §8). See `types.md`.

use sophia_hash::canonical::Writer;
use sophia_hash::{Digest, NominalId};

/// What a name points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// A content-addressed definition.
    Content(Digest),
    /// A nominal entity (a generic function, a nominal type).
    Nominal(NominalId),
}

impl Target {
    pub fn digest(&self) -> Digest {
        match self {
            Target::Content(d) | Target::Nominal(d) => *d,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Visibility {
    #[default]
    Public,
    Private,
}

impl Visibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "public" => Some(Visibility::Public),
            "private" => Some(Visibility::Private),
            _ => None,
        }
    }
}

/// A name: a dotted path pointing at one target. `seq` is the log position
/// of its last change, and serves as its version for optimistic checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameEntry {
    pub path: String,
    pub target: Target,
    pub visibility: Visibility,
    pub seq: u64,
}

/// What a log entry records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogOp {
    AddDefinition,
    AddName,
    MoveName,
    RemoveName,
    NewNominal,
    AddMethod,
    RemoveMethod,
    Annotate,
    Fact,
    Setting,
}

impl LogOp {
    pub const ALL: [LogOp; 10] = [
        LogOp::AddDefinition,
        LogOp::AddName,
        LogOp::MoveName,
        LogOp::RemoveName,
        LogOp::NewNominal,
        LogOp::AddMethod,
        LogOp::RemoveMethod,
        LogOp::Annotate,
        LogOp::Fact,
        LogOp::Setting,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            LogOp::AddDefinition => "add-definition",
            LogOp::AddName => "add-name",
            LogOp::MoveName => "move-name",
            LogOp::RemoveName => "remove-name",
            LogOp::NewNominal => "new-nominal",
            LogOp::AddMethod => "add-method",
            LogOp::RemoveMethod => "remove-method",
            LogOp::Annotate => "annotate",
            LogOp::Fact => "fact",
            LogOp::Setting => "setting",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        LogOp::ALL.into_iter().find(|o| o.as_str() == s)
    }
}

/// One row of the append-only log: the history and the change feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub seq: u64,
    pub op: LogOp,
    /// A name path, a definition id in hex, a nominal id, a setting key.
    pub subject: String,
    pub old: Option<Digest>,
    pub new: Option<Digest>,
    pub actor: String,
    /// Unix seconds.
    pub at: u64,
}

/// A nominal entity (SPEC-001 §4.3). `version` is the log position of the
/// last change to its relation (for a generic function, its method table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nominal {
    pub id: NominalId,
    pub lang: String,
    pub kind: String,
    /// The name it was created under, for display; names live in the name table.
    pub label: String,
    pub created_seq: u64,
    pub version: u64,
}

/// Membership of a definition in a nominal entity's relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationEntry {
    pub nominal: NominalId,
    pub member: Digest,
    pub added_seq: u64,
    pub removed_seq: Option<u64>,
}

/// One content in a lineage (SPEC-001 §5.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageEntry {
    pub lineage: Digest,
    pub id: Digest,
    pub seq: u64,
    pub supersedes: Option<Digest>,
}

/// A claim about definitions: a doc, a proof, an `unsafe` mark, an
/// equivalence (SPEC-001 §8). Its id hashes what is claimed; `status` and
/// `inputs` record what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    pub id: Digest,
    pub kind: String,
    pub subjects: Vec<Digest>,
    pub payload: Vec<u8>,
    pub checker: Option<String>,
    pub checker_version: Option<String>,
    pub status: String,
    pub inputs: Vec<Digest>,
}

impl Annotation {
    pub fn new(
        kind: &str,
        subjects: Vec<Digest>,
        payload: Vec<u8>,
        checker: Option<(&str, &str)>,
    ) -> Self {
        let mut w = Writer::new();
        w.str("sophia", "annotation-v0");
        w.str("kind", kind);
        for s in &subjects {
            w.field("subject", s.as_bytes());
        }
        w.field("payload", &payload);
        if let Some((c, v)) = checker {
            w.str("checker", c);
            w.str("version", v);
        }
        Annotation {
            id: Digest::of(&w.finish()),
            kind: kind.into(),
            subjects,
            payload,
            checker: checker.map(|c| c.0.into()),
            checker_version: checker.map(|c| c.1.into()),
            status: "asserted".into(),
            inputs: Vec::new(),
        }
    }
}

/// What a derived fact depended on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Input {
    /// An immutable definition.
    Content(Digest),
    /// A nominal entity's relation, at a version.
    Relation { nominal: NominalId, version: u64 },
}

/// Something a tool computed, reproducibly, from its inputs (SPEC-001 §8):
/// a typing, a dispatch resolution, a test result, a compiled artefact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub id: Digest,
    pub kind: String,
    pub subject: Digest,
    pub value: Vec<u8>,
    pub by: String,
    pub by_version: String,
    pub inputs: Vec<Input>,
    pub derived_at: u64,
}

impl Fact {
    pub fn new(
        kind: &str,
        subject: Digest,
        value: Vec<u8>,
        by: (&str, &str),
        inputs: Vec<Input>,
        derived_at: u64,
    ) -> Self {
        let mut w = Writer::new();
        w.str("sophia", "fact-v0");
        w.str("kind", kind);
        w.field("subject", subject.as_bytes());
        w.field("value", &value);
        w.str("by", by.0);
        w.str("version", by.1);
        for i in &inputs {
            match i {
                Input::Content(d) => w.field("in", d.as_bytes()),
                Input::Relation { nominal, version } => {
                    let o = w.open("rel");
                    w.field("n", nominal.as_bytes());
                    w.u64("v", *version);
                    w.close(o);
                }
            }
        }
        Fact {
            id: Digest::of(&w.finish()),
            kind: kind.into(),
            subject,
            value,
            by: by.0.into(),
            by_version: by.1.into(),
            inputs,
            derived_at,
        }
    }
}

/// A page of changes for `changes_since` (SPEC-004 §6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Changes {
    pub entries: Vec<LogEntry>,
    /// The seq to pass next time.
    pub next: u64,
    /// More entries exist after `next`.
    pub more: bool,
}
