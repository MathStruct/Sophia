//! Byte encodings of the store's types, for key-value backends (redb, and
//! later LMDB). Uses the canonical tag-length-value framing, so values are
//! self-describing and versionable. See `codec.md`.

use sophia_hash::canonical::{ReadError, Reader, Writer};
use sophia_hash::{Digest, Record, Status};

use crate::error::StoreError;
use crate::types::*;

type Result<T> = std::result::Result<T, StoreError>;

fn bad(what: &'static str) -> StoreError {
    StoreError::Codec(what.into())
}

impl From<ReadError> for StoreError {
    fn from(e: ReadError) -> Self {
        StoreError::Codec(e.to_string())
    }
}

fn digest(r: &mut Reader, tag: &str) -> Result<Digest> {
    Digest::from_slice(r.expect(tag)?).ok_or(bad("digest"))
}

fn opt_digest(r: &mut Reader, tag: &str) -> Result<Option<Digest>> {
    let p = r.expect(tag)?;
    if p.is_empty() { Ok(None) } else { Digest::from_slice(p).map(Some).ok_or(bad("digest")) }
}

fn u64_field(r: &mut Reader, tag: &str) -> Result<u64> {
    let p = r.expect(tag)?;
    Ok(u64::from_le_bytes(p.try_into().map_err(|_| bad("u64"))?))
}

fn string(r: &mut Reader, tag: &str) -> Result<String> {
    Ok(r.expect_str(tag)?.to_string())
}

fn digests(w: &mut Writer, tag: &str, ds: &[Digest]) {
    let mut v = Vec::with_capacity(ds.len() * 32);
    for d in ds {
        v.extend_from_slice(d.as_bytes());
    }
    w.field(tag, &v);
}

fn read_digests(r: &mut Reader, tag: &str) -> Result<Vec<Digest>> {
    let p = r.expect(tag)?;
    if p.len() % 32 != 0 {
        return Err(bad("digest list"));
    }
    Ok(p.chunks(32).map(|c| Digest::from_slice(c).unwrap()).collect())
}

pub fn encode_record(r: &Record) -> Vec<u8> {
    let mut w = Writer::with_capacity(r.bytes.len() + r.display.len() + 160);
    w.field("id", r.id.as_bytes());
    w.str("lang", &r.lang);
    w.str("normaliser", &r.normaliser);
    w.str("kind", &r.kind);
    w.field("component", r.component.as_bytes());
    w.u32("index", r.index);
    w.u32("size", r.component_size);
    w.field("bytes", &r.bytes);
    w.field("display", &r.display);
    w.str("status", r.status.as_str());
    digests(&mut w, "refs", &r.refs);
    digests(&mut w, "nominal_refs", &r.nominal_refs);
    w.finish()
}

pub fn decode_record(b: &[u8]) -> Result<Record> {
    let mut r = Reader::new(b);
    Ok(Record {
        id: digest(&mut r, "id")?,
        lang: string(&mut r, "lang")?,
        normaliser: string(&mut r, "normaliser")?,
        kind: string(&mut r, "kind")?,
        component: digest(&mut r, "component")?,
        index: r.expect_u32("index")?,
        component_size: r.expect_u32("size")?,
        bytes: r.expect("bytes")?.to_vec(),
        display: r.expect("display")?.to_vec(),
        status: Status::parse(r.expect_str("status")?).ok_or(bad("status"))?,
        refs: read_digests(&mut r, "refs")?,
        nominal_refs: read_digests(&mut r, "nominal_refs")?,
    })
}

pub fn encode_name(n: &NameEntry) -> Vec<u8> {
    let mut w = Writer::new();
    w.str("path", &n.path);
    let (k, d) = match n.target {
        Target::Content(d) => ("content", d),
        Target::Nominal(d) => ("nominal", d),
    };
    w.str("target_kind", k);
    w.field("target", d.as_bytes());
    w.str("visibility", n.visibility.as_str());
    w.u64("seq", n.seq);
    w.finish()
}

pub fn decode_name(b: &[u8]) -> Result<NameEntry> {
    let mut r = Reader::new(b);
    let path = string(&mut r, "path")?;
    let kind = r.expect_str("target_kind")?;
    let d = digest(&mut r, "target")?;
    let target = match kind {
        "content" => Target::Content(d),
        "nominal" => Target::Nominal(d),
        _ => return Err(bad("target kind")),
    };
    let visibility = Visibility::parse(r.expect_str("visibility")?).ok_or(bad("visibility"))?;
    Ok(NameEntry { path, target, visibility, seq: u64_field(&mut r, "seq")? })
}

fn opt(d: &Option<Digest>) -> &[u8] {
    d.as_ref().map(|d| &d.as_bytes()[..]).unwrap_or(&[])
}

pub fn encode_log(e: &LogEntry) -> Vec<u8> {
    let mut w = Writer::new();
    w.u64("seq", e.seq);
    w.str("op", e.op.as_str());
    w.str("subject", &e.subject);
    w.field("old", opt(&e.old));
    w.field("new", opt(&e.new));
    w.str("actor", &e.actor);
    w.u64("at", e.at);
    w.finish()
}

pub fn decode_log(b: &[u8]) -> Result<LogEntry> {
    let mut r = Reader::new(b);
    Ok(LogEntry {
        seq: u64_field(&mut r, "seq")?,
        op: LogOp::parse(r.expect_str("op")?).ok_or(bad("log op"))?,
        subject: string(&mut r, "subject")?,
        old: opt_digest(&mut r, "old")?,
        new: opt_digest(&mut r, "new")?,
        actor: string(&mut r, "actor")?,
        at: u64_field(&mut r, "at")?,
    })
}

pub fn encode_nominal(n: &Nominal) -> Vec<u8> {
    let mut w = Writer::new();
    w.field("id", n.id.as_bytes());
    w.str("lang", &n.lang);
    w.str("kind", &n.kind);
    w.str("label", &n.label);
    w.u64("created", n.created_seq);
    w.u64("version", n.version);
    w.finish()
}

pub fn decode_nominal(b: &[u8]) -> Result<Nominal> {
    let mut r = Reader::new(b);
    Ok(Nominal {
        id: digest(&mut r, "id")?,
        lang: string(&mut r, "lang")?,
        kind: string(&mut r, "kind")?,
        label: string(&mut r, "label")?,
        created_seq: u64_field(&mut r, "created")?,
        version: u64_field(&mut r, "version")?,
    })
}

pub fn encode_relation(e: &RelationEntry) -> Vec<u8> {
    let mut w = Writer::new();
    w.field("nominal", e.nominal.as_bytes());
    w.field("member", e.member.as_bytes());
    w.u64("added", e.added_seq);
    w.field("removed", &e.removed_seq.map(|s| s.to_le_bytes().to_vec()).unwrap_or_default());
    w.finish()
}

pub fn decode_relation(b: &[u8]) -> Result<RelationEntry> {
    let mut r = Reader::new(b);
    let nominal = digest(&mut r, "nominal")?;
    let member = digest(&mut r, "member")?;
    let added_seq = u64_field(&mut r, "added")?;
    let p = r.expect("removed")?;
    let removed_seq = if p.is_empty() { None } else { Some(u64::from_le_bytes(p.try_into().map_err(|_| bad("u64"))?)) };
    Ok(RelationEntry { nominal, member, added_seq, removed_seq })
}

pub fn encode_lineage(e: &LineageEntry) -> Vec<u8> {
    let mut w = Writer::new();
    w.field("lineage", e.lineage.as_bytes());
    w.field("id", e.id.as_bytes());
    w.u64("seq", e.seq);
    w.field("supersedes", opt(&e.supersedes));
    w.finish()
}

pub fn decode_lineage(b: &[u8]) -> Result<LineageEntry> {
    let mut r = Reader::new(b);
    Ok(LineageEntry {
        lineage: digest(&mut r, "lineage")?,
        id: digest(&mut r, "id")?,
        seq: u64_field(&mut r, "seq")?,
        supersedes: opt_digest(&mut r, "supersedes")?,
    })
}

pub fn encode_annotation(a: &Annotation) -> Vec<u8> {
    let mut w = Writer::new();
    w.field("id", a.id.as_bytes());
    w.str("kind", &a.kind);
    digests(&mut w, "subjects", &a.subjects);
    w.field("payload", &a.payload);
    w.str("checker", a.checker.as_deref().unwrap_or(""));
    w.str("checker_version", a.checker_version.as_deref().unwrap_or(""));
    w.str("status", &a.status);
    digests(&mut w, "inputs", &a.inputs);
    w.finish()
}

pub fn decode_annotation(b: &[u8]) -> Result<Annotation> {
    let mut r = Reader::new(b);
    let none_if_empty = |s: String| if s.is_empty() { None } else { Some(s) };
    Ok(Annotation {
        id: digest(&mut r, "id")?,
        kind: string(&mut r, "kind")?,
        subjects: read_digests(&mut r, "subjects")?,
        payload: r.expect("payload")?.to_vec(),
        checker: none_if_empty(string(&mut r, "checker")?),
        checker_version: none_if_empty(string(&mut r, "checker_version")?),
        status: string(&mut r, "status")?,
        inputs: read_digests(&mut r, "inputs")?,
    })
}

pub fn encode_inputs(inputs: &[Input]) -> Vec<u8> {
    let mut w = Writer::new();
    for i in inputs {
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
    w.finish()
}

pub fn decode_inputs(b: &[u8]) -> Result<Vec<Input>> {
    let mut r = Reader::new(b);
    let mut out = Vec::new();
    while !r.is_empty() {
        match r.peek_tag()? {
            "in" => out.push(Input::Content(digest(&mut r, "in")?)),
            "rel" => {
                let mut x = r.enter("rel")?;
                out.push(Input::Relation { nominal: digest(&mut x, "n")?, version: u64_field(&mut x, "v")? });
            }
            _ => return Err(bad("input")),
        }
    }
    Ok(out)
}

pub fn encode_fact(f: &Fact) -> Vec<u8> {
    let mut w = Writer::new();
    w.field("id", f.id.as_bytes());
    w.str("kind", &f.kind);
    w.field("subject", f.subject.as_bytes());
    w.field("value", &f.value);
    w.str("by", &f.by);
    w.str("by_version", &f.by_version);
    w.field("inputs", &encode_inputs(&f.inputs));
    w.u64("at", f.derived_at);
    w.finish()
}

pub fn decode_fact(b: &[u8]) -> Result<Fact> {
    let mut r = Reader::new(b);
    Ok(Fact {
        id: digest(&mut r, "id")?,
        kind: string(&mut r, "kind")?,
        subject: digest(&mut r, "subject")?,
        value: r.expect("value")?.to_vec(),
        by: string(&mut r, "by")?,
        by_version: string(&mut r, "by_version")?,
        inputs: decode_inputs(r.expect("inputs")?)?,
        derived_at: u64_field(&mut r, "at")?,
    })
}
