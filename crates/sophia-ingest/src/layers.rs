//! One derivative at two layers (experiment E-6): the output of
//! `tests/julia-semantics/layer_programs.jl` (for each function of the E-5 fixture
//! and an argument type, Zygote's derived program at Julia IR, Enzyme's at
//! LLVM IR, and both gradients at fixed points against a high-precision
//! reference) turned into store objects that exist already:
//!
//! - each derived program is a **record** of its layer's language
//!   (`julia-ir`: Zygote's program, whose pieces are records it references;
//!   `llvm-ir`: Enzyme's module, its text as Julia printed it with addresses
//!   and session numbers replaced);
//! - each derivation is a **fact** about f (kind `derived-program`, SPEC-006
//!   §6a): the tool, the mode, the argument type and the program's id, with
//!   f's closure as inputs;
//! - the claim that the two compute the same function is an **annotation**
//!   on both programs (kind `equivalent-tested`, status `tested`, SPEC-006 §5)
//!   whose payload is the test: the points, both values, the reference and
//!   the tolerance.
//!
//! See `layer_programs.md` and [[Semantic Boundary Experiments]] §E-6.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use sophia_hash::canonical::Writer as Canonical;
use sophia_hash::{Digest, Record};
use sophia_store::{Annotation, Backend, Fact, Input, Store, StoreError};

use crate::derivative::{DerivativeLedger, Derived, Oracle, derivative_ledger, parse_derivatives};
use crate::module::ModuleError;
use crate::source::single;

/// Zygote's derived program and its pieces: Julia IR, as `derivatives.jl`
/// prints it.
pub const JULIA_IR: &str = "julia-ir";
/// Enzyme's derived program: an LLVM module.
pub const LLVM_IR: &str = "llvm-ir";
pub const PROGRAM_NORMALISER: &str = "sophia-e6/zygote-program@0";
pub const PIECE_NORMALISER: &str = "sophia-e6/zygote-piece@0";
/// The module text as `layer_programs.jl` prints it: Julia objects' addresses
/// replaced by what they point to, session-numbered names renumbered by
/// first appearance.
pub const MODULE_NORMALISER: &str = "sophia-e6/llvm-module@0";
/// Record kinds.
pub const PROGRAM: &str = "derived-program";
pub const PIECE: &str = "derived-piece";
/// The fact kind: f was differentiated into a program.
pub const DERIVED: &str = "derived-program";
/// Annotation kinds: the two programs agreed at every point, or did not.
pub const EQUIVALENT_TESTED: &str = "equivalent-tested";
pub const EQUIVALENT_FAILED: &str = "equivalent-test-failed";
/// The checker named on the annotation.
pub const CHECKER: (&str, &str) = ("sophia-e6/numerical", "0");
/// Two gradients agree when they differ by at most this many units of
/// `eps(T)` times `max(1, |reference|)`.
pub const TOLERANCE_ULPS: f64 = 4.0;

/// One `enzyme` line: what Enzyme made for one gradient.
#[derive(Debug, Clone)]
pub struct EnzymeRow {
    pub label: String,
    pub target: String,
    pub argtype: String,
    /// `ok`, or `refused:` and the exception Enzyme threw.
    pub status: String,
    /// The first `autodiff` call (Enzyme's transformation, compilation, a run).
    pub ns_first: u64,
    /// Exporting the module again (`enzyme_code_llvm`).
    pub ns_export: u64,
    pub raw_bytes: usize,
    /// Digest of the text as Julia printed it, addresses included.
    pub raw_digest: String,
    /// Distinct embedded addresses, and how many were named.
    pub addresses: usize,
    pub described: usize,
    pub bytes: usize,
    /// Digest of the normalised text (`module` line), `-` if refused.
    pub digest: String,
}

/// One `point` line: both gradients at `x` and the reference. A value that
/// does not read as a number is NaN (`x`, `reference`) or `None`, so the
/// point fails the test instead of disappearing from it.
#[derive(Debug, Clone)]
pub struct Point {
    pub label: String,
    pub target: String,
    pub argtype: String,
    pub x: f64,
    pub zygote: Option<f64>,
    pub enzyme: Option<f64>,
    pub reference: f64,
}

/// What `layer_programs.jl` printed.
#[derive(Debug, Default)]
pub struct Layers {
    /// The Zygote side, as `derivatives.jl` prints it.
    pub zygote: Oracle,
    pub enzyme: Vec<EnzymeRow>,
    pub points: Vec<Point>,
    /// Normalised module texts by digest.
    pub modules: HashMap<String, String>,
}

/// A Julia float as `repr` prints it (`0.5`, `Inf`; a `Float32` as `0.5f0`,
/// `1.0f-5`, `Inf32`, `NaN32`), widened exactly to `f64`; `None` for `-`,
/// `nothing`, an error or anything else.
fn float(s: &str) -> Option<f64> {
    match s {
        "Inf" | "Inf32" => return Some(f64::INFINITY),
        "-Inf" | "-Inf32" => return Some(f64::NEG_INFINITY),
        "NaN" | "NaN32" => return Some(f64::NAN),
        _ => {}
    }
    if !s.bytes().all(|b| b.is_ascii_digit() || b"+-.ef".contains(&b)) {
        return None;
    }
    // A Float32 is read as one, then widened: its decimal is the shortest
    // that rounds to it in Float32, not its value.
    match s.split_once('f') {
        Some((m, e)) => format!("{m}e{e}").parse::<f32>().ok().map(f64::from),
        None => s.parse().ok(),
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

pub fn parse_layers(tsv: &str) -> Layers {
    let mut l = Layers { zygote: parse_derivatives(tsv), ..Layers::default() };
    for line in tsv.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["enzyme", label, target, ty, status, first, export, raw_bytes, raw_digest, addresses, described, bytes, d] => {
                l.enzyme.push(EnzymeRow {
                    label: label.to_string(),
                    target: target.to_string(),
                    argtype: ty.to_string(),
                    status: status.to_string(),
                    ns_first: first.parse().unwrap_or(0),
                    ns_export: export.parse().unwrap_or(0),
                    raw_bytes: raw_bytes.parse().unwrap_or(0),
                    raw_digest: raw_digest.to_string(),
                    addresses: addresses.parse().unwrap_or(0),
                    described: described.parse().unwrap_or(0),
                    bytes: bytes.parse().unwrap_or(0),
                    digest: d.to_string(),
                })
            }
            ["point", label, target, ty, x, zy, en, reference] => l.points.push(Point {
                label: label.to_string(),
                target: target.to_string(),
                argtype: ty.to_string(),
                x: float(x).unwrap_or(f64::NAN),
                zygote: float(zy),
                enzyme: float(en),
                reference: float(reference).unwrap_or(f64::NAN),
            }),
            ["module", d, text] => {
                l.modules.insert(d.to_string(), unescape(text));
            }
            _ => {}
        }
    }
    l
}

/// A piece of Zygote's program as a record.
pub fn piece_record(text: &str) -> Record {
    single(JULIA_IR, PIECE_NORMALISER, PIECE, text.as_bytes().to_vec(), Vec::new(), Vec::new())
}

/// Zygote's program as a record: its root (the gradient's own instance) and,
/// sorted, each reached instance's signature, the kind of its piece and the
/// piece's record id. It references the piece records. `pieces` maps the
/// oracle's piece digest to the piece's record id.
pub fn zygote_program(d: &Derived, pieces: &HashMap<String, Digest>) -> Option<Record> {
    let mut c = Canonical::new();
    c.str("root", d.pieces.first().map(|x| x.0.as_str()).unwrap_or(""));
    let mut ps: Vec<(&str, &str, Digest)> = Vec::new();
    for (sig, kind, dg) in &d.pieces {
        ps.push((sig, kind, *pieces.get(dg)?));
    }
    ps.sort();
    let mut refs = BTreeSet::new();
    for (sig, kind, id) in &ps {
        let o = c.open("piece");
        c.str("sig", sig);
        c.str("kind", kind);
        c.field("id", id.as_bytes());
        c.close(o);
        refs.insert(*id);
    }
    Some(single(JULIA_IR, PROGRAM_NORMALISER, PROGRAM, c.finish(), Vec::new(), refs.into_iter().collect()))
}

/// Enzyme's program as a record: the normalised module text.
pub fn enzyme_program(text: &str) -> Record {
    single(LLVM_IR, MODULE_NORMALISER, PROGRAM, text.as_bytes().to_vec(), Vec::new(), Vec::new())
}

/// How far apart the gradients are, in units of `eps(T) · max(1, |ref|)`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Agreement {
    pub points: usize,
    /// Zygote against Enzyme: the claim.
    pub between: f64,
    /// Each against the reference: recorded, not claimed (a conditioned
    /// function is far from it in both).
    pub zygote: f64,
    pub enzyme: f64,
    /// Whether every point had both gradients within [`TOLERANCE_ULPS`].
    pub tested: bool,
}

fn eps(argtype: &str) -> Option<f64> {
    match argtype {
        "Float64" => Some(f64::EPSILON),
        "Float32" => Some(f32::EPSILON as f64),
        _ => None,
    }
}

pub fn agreement(points: &[&Point]) -> Agreement {
    let mut a = Agreement { points: points.len(), tested: !points.is_empty(), ..Agreement::default() };
    for p in points {
        let Some(e) = eps(&p.argtype) else {
            a.tested = false;
            continue;
        };
        // `f64::max` ignores NaN: an unreadable reference must fail here.
        if !p.x.is_finite() || !p.reference.is_finite() {
            a.tested = false;
            continue;
        }
        let unit = e * p.reference.abs().max(1.0);
        match (p.zygote, p.enzyme) {
            (Some(z), Some(n)) => {
                let d = (z - n).abs() / unit;
                // A NaN difference is a disagreement.
                if d.is_nan() || d > TOLERANCE_ULPS {
                    a.tested = false;
                }
                a.between = a.between.max(if d.is_nan() { f64::INFINITY } else { d });
                a.zygote = a.zygote.max((z - p.reference).abs() / unit);
                a.enzyme = a.enzyme.max((n - p.reference).abs() / unit);
            }
            _ => a.tested = false,
        }
    }
    a
}

/// The test as the annotation's payload: tolerance, reference and every
/// point with both values (as bits, so the payload is exact).
fn payload(points: &[&Point]) -> Vec<u8> {
    let mut c = Canonical::new();
    c.str("obligation", "gradient(zygote, x) = gradient(enzyme, x)");
    c.str("tolerance", &format!("{TOLERANCE_ULPS} eps(T) max(1, |ref|)"));
    c.str("reference", "central difference of f, h = 2^-120, 512-bit BigFloat");
    for p in points {
        let o = c.open("point");
        c.u64("x", p.x.to_bits());
        c.u64("zygote", p.zygote.map(f64::to_bits).unwrap_or(u64::MAX));
        c.u64("enzyme", p.enzyme.map(f64::to_bits).unwrap_or(u64::MAX));
        c.u64("reference", p.reference.to_bits());
        c.close(o);
    }
    c.finish()
}

/// One function at one argument type, at both layers.
#[derive(Debug, Clone)]
pub struct Pair {
    pub target: String,
    pub argtype: String,
    /// f's definition: the facts' subject.
    pub root: Digest,
    pub zygote: Record,
    pub enzyme: Record,
    /// Zygote's fact, then Enzyme's.
    pub facts: [Fact; 2],
    pub annotation: Annotation,
    pub agreement: Agreement,
}

/// Everything E-6 stores, and what could not be stored.
#[derive(Debug, Default)]
pub struct LayerReport {
    pub pairs: Vec<Pair>,
    /// Zygote's pieces, each once.
    pub pieces: Vec<Record>,
    /// (target, argtype, why) for a gradient without both programs.
    pub skipped: Vec<(String, String, String)>,
}

impl LayerReport {
    /// Every record to store: pieces first, then the programs.
    pub fn records(&self) -> Vec<Record> {
        let mut out = self.pieces.clone();
        for p in &self.pairs {
            out.push(p.zygote.clone());
            out.push(p.enzyme.clone());
        }
        out
    }
}

fn sha256(text: &str) -> String {
    use sha2::Digest as _;
    sha2::Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn digest(s: &str) -> Option<Digest> {
    s.parse().ok()
}

fn versions(o: &Oracle, label: &str, names: &[&str]) -> String {
    let v = o.versions.get(label);
    names
        .iter()
        .map(|n| {
            let ver = v.and_then(|s| s.iter().find(|(p, _)| p == n)).map(|(_, x)| x.as_str()).unwrap_or("?");
            format!("{n} {ver}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// What a derivation's fact names besides the program: the layer, the
/// tool, every version it ran with, the mode.
struct Derivation<'a> {
    layer: &'a str,
    tool: &'a str,
    /// `name version` pairs, comma-separated.
    versions: &'a str,
    mode: &'a str,
}

impl Derivation<'_> {
    fn fact(&self, argtype: &str, program: &Record, root: Digest, inputs: &[Input]) -> Fact {
        let mut c = Canonical::new();
        c.str("layer", self.layer);
        c.str("versions", self.versions);
        c.str("mode", self.mode);
        c.str("argtype", argtype);
        c.field("program", program.id.as_bytes());
        let version =
            self.versions.split(", ").find_map(|v| v.split_once(' ').filter(|(n, _)| *n == self.tool).map(|(_, x)| x)).unwrap_or("?");
        Fact::new(DERIVED, root, c.finish(), (self.tool, version), inputs.to_vec(), 0)
    }
}

/// Builds E-6's store objects from `layer_programs.jl`'s output for the package at
/// `dir` (module `module`), whose label in the output is `label`.
pub fn layer_records(module: &str, label: &str, dir: &Path, tsv: &str) -> Result<(Layers, DerivativeLedger, LayerReport), ModuleError> {
    let l = parse_layers(tsv);
    let (_, ledger) = derivative_ledger(module, &[(label, dir.to_path_buf())], tsv)?;
    let mut report = LayerReport::default();
    // Pieces, each once, by the oracle's digest.
    let mut piece_ids = HashMap::new();
    let mut seen = BTreeMap::new();
    for (d, text) in &l.zygote.texts {
        let r = piece_record(text);
        piece_ids.insert(d.clone(), r.id);
        seen.insert(r.id, r);
    }
    let zygote_versions = versions(&l.zygote, label, &["julia", "Zygote", "ChainRulesCore", "ChainRules", "IRTools"]);
    let enzyme_versions = versions(&l.zygote, label, &["julia", "Enzyme", "EnzymeCore", "LLVM"]);
    let mut used = BTreeSet::new();
    for e in l.enzyme.iter().filter(|e| e.label == label) {
        let skip = |why: &str| (e.target.clone(), e.argtype.clone(), why.to_string());
        let Some(d) = l.zygote.derived.iter().find(|d| d.label == label && d.target == e.target && d.argtype == e.argtype) else {
            report.skipped.push(skip("no Zygote row"));
            continue;
        };
        if !d.complete {
            report.skipped.push(skip("Zygote's program is not complete (runtime dispatch)"));
            continue;
        }
        if e.status != "ok" {
            report.skipped.push(skip(&format!("Enzyme {}", e.status)));
            continue;
        }
        let Some(text) = l.modules.get(&e.digest) else {
            report.skipped.push(skip("no module text"));
            continue;
        };
        if sha256(text) != e.digest {
            report.skipped.push(skip("the module text does not match its digest"));
            continue;
        }
        let Some(row) = ledger.row(label, &e.target, &e.argtype) else {
            report.skipped.push(skip("no key row"));
            continue;
        };
        // Which of several methods a gradient starts from is dispatch, which
        // the ids do not say: no subject for the fact.
        if row.roots.len() > 1 {
            report.skipped.push(skip(&format!("f has {} methods: the gradient's is decided by dispatch", row.roots.len())));
            continue;
        }
        let Some(root) = row.roots.first().and_then(|r| digest(r)) else {
            report.skipped.push(skip("f has no id"));
            continue;
        };
        let Some(zygote) = zygote_program(d, &piece_ids) else {
            report.skipped.push(skip("a piece has no text"));
            continue;
        };
        let enzyme = enzyme_program(text);
        // The closure's ids, sorted (a refused definition has none and is
        // left out; the fixture has none).
        let inputs: Vec<Input> = row.closure_ids.iter().filter_map(|i| digest(i)).map(Input::Content).collect();
        let facts = [
            Derivation { layer: JULIA_IR, tool: "Zygote", versions: &zygote_versions, mode: "reverse, Context{false}" }
                .fact(&e.argtype, &zygote, root, &inputs),
            Derivation { layer: LLVM_IR, tool: "Enzyme", versions: &enzyme_versions, mode: "Reverse, Active return, Active argument" }
                .fact(&e.argtype, &enzyme, root, &inputs),
        ];
        let points: Vec<&Point> = l.points.iter().filter(|p| p.label == label && p.target == e.target && p.argtype == e.argtype).collect();
        let agreement = agreement(&points);
        let kind = if agreement.tested { EQUIVALENT_TESTED } else { EQUIVALENT_FAILED };
        let mut annotation = Annotation::new(kind, vec![zygote.id, enzyme.id], payload(&points), Some(CHECKER));
        annotation.status = if agreement.tested { "tested" } else { "failed" }.into();
        annotation.inputs = vec![root];
        used.extend(zygote.refs.iter().copied());
        report.pairs.push(Pair {
            target: e.target.clone(),
            argtype: e.argtype.clone(),
            root,
            zygote,
            enzyme,
            facts,
            annotation,
            agreement,
        });
    }
    report.pieces = seen.into_values().filter(|r| used.contains(&r.id)).collect();
    Ok((l, ledger, report))
}

/// Writes the report into a store: the records, the facts, the annotations.
pub fn store_layers<B: Backend>(store: &Store<B>, report: &LayerReport) -> Result<(), StoreError> {
    store.put_records(&report.records())?;
    let facts: Vec<Fact> = report.pairs.iter().flat_map(|p| p.facts.iter().cloned()).collect();
    store.record_facts(&facts)?;
    let anns: Vec<Annotation> = report.pairs.iter().map(|p| p.annotation.clone()).collect();
    store.annotate_many(&anns)?;
    Ok(())
}

/// An LLVM module's top-level functions (`define … { … }`), each as text,
/// and the rest (header, declarations, attributes, metadata) as one text: to
/// measure how much modules would share if functions were records.
pub fn split_module(text: &str) -> (String, Vec<String>) {
    let mut rest = String::new();
    let mut funcs = Vec::new();
    let mut cur: Option<String> = None;
    for line in text.lines() {
        if let Some(c) = cur.as_mut() {
            c.push_str(line);
            c.push('\n');
            if line == "}" {
                funcs.push(cur.take().unwrap_or_default());
            }
        } else if line.starts_with("define ") {
            cur = Some(format!("{line}\n"));
        } else {
            rest.push_str(line);
            rest.push('\n');
        }
    }
    if let Some(c) = cur {
        rest.push_str(&c);
    }
    (rest, funcs)
}
