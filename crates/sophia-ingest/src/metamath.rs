//! A Metamath frontend ([[Real Code Corpora]] rung 1): reads a `.mm`
//! database and turns every assertion (`$a`, `$p`) into a Sophia record of
//! the language `metamath`, content-addressed in a Merkle way. See
//! `metamath.md`.
//!
//! The normal form of an assertion's **statement**, which is its identity:
//! - its **mandatory hypotheses in order** (the theorem's interface: callers'
//!   proofs depend on the order): floating ones as (typecode, variable),
//!   essential ones as (typecode, math string);
//! - its typecode and math string;
//! - its mandatory disjoint-variable pairs, sorted.
//!
//! A theorem's **proof** is a separate record of kind `proof` (evidence, not
//! identity: SPEC-002 §3). It names the statement it proves and lists its
//! steps (packed as varints into a list of what it uses, each listed once,
//! as Metamath's compressed proofs do): mandatory hypothesis by position, a
//! dummy variable's floating
//! hypothesis by (typecode, variable), an earlier theorem by **its proof's
//! content id** (the justification the author chose), an axiom, definition
//! or syntax constructor by its statement's content id, a saved step, a
//! save marker, an unknown step. So a proof's identity covers the whole
//! chain of proofs below it, and its references are what it rests on
//! directly: other proofs and assumed statements. Which statement it proves is
//! part of its content (`of`) and, in the store, a `proof` annotation on
//! both (SPEC-001 §8): "proves" and "uses" stay distinguishable.
//! So two libraries that prove the same statement differently share the
//! statement and keep both proofs (measured: set.mm and iset.mm).
//!
//! Variables are numbered by first occurrence (mandatory hypotheses, then the
//! statement, then dummies in the proof), constants are kept as text. So an
//! assertion's id does not depend on its label or its variable names, and a
//! renamed or repeated theorem is stored once. A compressed and a normal
//! proof of the same theorem are different content (they are different
//! proofs as written).

use std::collections::{BTreeSet, HashMap};

use sophia_hash::canonical::Writer;
use sophia_hash::merkle::{component_digest, member_digest};
use sophia_hash::{Digest, Record, Status};

pub const LANG: &str = "metamath";
pub const NORMALISER: &str = "sophia-metamath@2";

#[derive(Debug, thiserror::Error)]
pub enum MmError {
    #[error("{0} (near token {1})")]
    Syntax(String, usize),
    #[error("file inclusion ($[ … $]) is not supported")]
    Include,
}

/// One assertion, ready to store: its statement, and for a theorem its
/// proof.
#[derive(Debug, Clone)]
pub struct Assertion {
    pub label: String,
    /// The role in this library: `axiom`, `definition`, `theorem` or
    /// `syntax`. Not part of the record: recorded as an annotation.
    pub role: String,
    /// The statement: hypotheses, disjoint pairs, typecode and math string.
    pub record: Record,
    /// The proof (kind `proof`): references the statements of every
    /// assertion it uses; names the statement it proves in its content.
    pub proof: Option<Record>,
}

fn record(id: Digest, component: Digest, kind: &str, bytes: Vec<u8>, display: Vec<u8>, refs: Vec<Digest>) -> Record {
    Record {
        id,
        lang: LANG.into(),
        normaliser: NORMALISER.into(),
        kind: kind.into(),
        component,
        index: 0,
        component_size: 1,
        bytes,
        display,
        status: Status::Complete,
        refs,
        nominal_refs: Vec::new(),
    }
}

#[derive(Debug, Default, Clone)]
struct Frame {
    /// variable → (label, typecode, declaration order)
    floats: Vec<(String, String, String, usize)>,
    /// (label, typecode, symbols, declaration order)
    essentials: Vec<(String, String, Vec<String>, usize)>,
    disjoint: Vec<(String, String)>,
}

/// A mandatory hypothesis with its declaration order and label.
type Mandatory = (usize, String, Hyp);

/// A mandatory hypothesis in order.
enum Hyp {
    F { tc: String, var: String },
    E { tc: String, syms: Vec<String> },
}

struct Parser<'a> {
    toks: Vec<&'a str>,
    pos: usize,
    frames: Vec<Frame>,
    vars: BTreeSet<String>,
    /// label → content id of every assertion so far
    ids: HashMap<String, Digest>,
    /// label → content id of the proof of every theorem so far
    proofs: HashMap<String, Digest>,
    /// label → (typecode, variable) of every floating hypothesis ever seen
    out: Vec<Assertion>,
    order: usize,
}

fn tokens(text: &str) -> Result<Vec<&str>, MmError> {
    let mut out = Vec::new();
    let mut it = text.split_ascii_whitespace();
    while let Some(t) = it.next() {
        match t {
            "$(" => {
                for c in it.by_ref() {
                    if c == "$)" {
                        break;
                    }
                }
            }
            "$[" => return Err(MmError::Include),
            t => out.push(t),
        }
    }
    Ok(out)
}

impl<'a> Parser<'a> {
    fn err<T>(&self, m: impl Into<String>) -> Result<T, MmError> {
        Err(MmError::Syntax(m.into(), self.pos))
    }

    fn until(&mut self, end: &str) -> Result<Vec<&'a str>, MmError> {
        let start = self.pos;
        while self.pos < self.toks.len() && self.toks[self.pos] != end {
            self.pos += 1;
        }
        if self.pos == self.toks.len() {
            self.pos = start;
            return self.err(format!("missing {end}"));
        }
        let v = self.toks[start..self.pos].to_vec();
        self.pos += 1;
        Ok(v)
    }

    fn active_floats(&self) -> impl Iterator<Item = &(String, String, String, usize)> {
        self.frames.iter().flat_map(|f| f.floats.iter())
    }

    /// Mandatory hypotheses of an assertion with this statement, in order,
    /// and the mandatory disjoint pairs.
    fn mandatory(&self, stmt: &[&str]) -> (Vec<Mandatory>, Vec<(String, String)>) {
        let mut used: BTreeSet<&str> = stmt.iter().copied().filter(|s| self.vars.contains(*s)).collect();
        let essentials: Vec<_> = self.frames.iter().flat_map(|f| f.essentials.iter()).collect();
        for (_, _, syms, _) in &essentials {
            used.extend(syms.iter().map(String::as_str).filter(|s| self.vars.contains(*s)));
        }
        let mut hyps: Vec<Mandatory> = Vec::new();
        for (var, label, tc, order) in self.active_floats() {
            if used.contains(var.as_str()) {
                hyps.push((*order, label.clone(), Hyp::F { tc: tc.clone(), var: var.clone() }));
            }
        }
        for (label, tc, syms, order) in essentials {
            hyps.push((*order, label.clone(), Hyp::E { tc: tc.clone(), syms: syms.clone() }));
        }
        hyps.sort_by_key(|h| h.0);
        let mut dv = Vec::new();
        for f in &self.frames {
            for (a, b) in &f.disjoint {
                if used.contains(a.as_str()) && used.contains(b.as_str()) {
                    dv.push((a.clone(), b.clone()));
                }
            }
        }
        (hyps, dv)
    }

    fn assertion(&mut self, label: &str, kind_tok: &str) -> Result<(), MmError> {
        let body = if kind_tok == "$a" { self.until("$.")? } else { self.until("$=")? };
        let Some((&tc, stmt)) = body.split_first() else { return self.err("empty statement") };
        let proof = if kind_tok == "$p" { Some(self.until("$.")?) } else { None };
        let (hyps, dv) = self.mandatory(stmt);

        // Variables by first occurrence: mandatory hypotheses, statement.
        let mut vnum: HashMap<String, u32> = HashMap::new();
        let number = |v: &str, vnum: &mut HashMap<String, u32>| {
            let n = vnum.len() as u32;
            *vnum.entry(v.to_string()).or_insert(n)
        };
        for (_, _, h) in &hyps {
            match h {
                Hyp::F { var, .. } => {
                    number(var, &mut vnum);
                }
                Hyp::E { syms, .. } => {
                    for s in syms.iter().filter(|s| self.vars.contains(s.as_str())) {
                        number(s, &mut vnum);
                    }
                }
            }
        }
        for s in stmt.iter().filter(|s| self.vars.contains(**s)) {
            number(s, &mut vnum);
        }
        let math = |w: &mut Writer, syms: &[&str], vnum: &HashMap<String, u32>| {
            for s in syms {
                match vnum.get(*s) {
                    Some(v) => w.u32("v", *v),
                    None => w.str("c", s),
                }
            }
        };

        let mut w = Writer::new();
        w.str("mm", "assertion-v0");
        for (_, _, h) in &hyps {
            match h {
                Hyp::F { tc, var } => {
                    let o = w.open("f");
                    w.str("tc", tc);
                    w.u32("v", vnum[var]);
                    w.close(o);
                }
                Hyp::E { tc, syms } => {
                    let o = w.open("e");
                    w.str("tc", tc);
                    let s: Vec<&str> = syms.iter().map(String::as_str).collect();
                    math(&mut w, &s, &vnum);
                    w.close(o);
                }
            }
        }
        let mut pairs: Vec<(u32, u32)> = dv
            .iter()
            .filter_map(|(a, b)| Some((*vnum.get(a)?, *vnum.get(b)?)))
            .map(|(a, b)| (a.min(b), a.max(b)))
            .collect();
        pairs.sort();
        pairs.dedup();
        for (a, b) in pairs {
            let o = w.open("d");
            w.u32("a", a);
            w.u32("b", b);
            w.close(o);
        }
        let o = w.open("stmt");
        w.str("tc", tc);
        math(&mut w, stmt, &vnum);
        w.close(o);

        // The statement is the assertion's identity. A proof is evidence
        // about it, stored as its own record (SPEC-002 §3: proofs never
        // change semantic identity).
        let bytes = w.finish();
        let component = component_digest(LANG, NORMALISER, &bytes);
        let id = member_digest(&component, 0);
        // A record's kind must follow from its content: a statement can be an
        // axiom in one library and a theorem in another. Axiom and
        // definition are per-library annotations; theorem is "has a proof".
        let kind = if tc == "|-" { "statement" } else { "syntax" };
        let role = match (kind_tok, tc) {
            ("$p", _) => "theorem",
            ("$a", "|-") if label.starts_with("df-") => "definition",
            ("$a", "|-") => "axiom",
            _ => "syntax",
        };
        let mut disp = Writer::new();
        disp.str("label", label);
        let mut names: Vec<(&String, &u32)> = vnum.iter().collect();
        names.sort_by_key(|(_, n)| **n);
        for (v, _) in names {
            disp.str("var", v);
        }
        let display = disp.finish();
        let statement = record(id, component, kind, bytes, display.clone(), Vec::new());

        let mut proof_record = None;
        if let Some(proof) = proof {
            // Steps go into a compact form: the distinct things used are
            // listed once (proofs and assumed statements by content id,
            // dummy variables by typecode and number), and the steps are
            // varints into those lists, as in Metamath's compressed proofs.
            #[derive(Clone, Copy)]
            enum St {
                Hyp(u64),
                Use(u64),
                Dummy(u64),
                Saved(u64),
                Save,
                Unknown,
            }
            let mut uses: Vec<(bool, Digest)> = Vec::new();
            let mut dummies: Vec<(String, u32)> = Vec::new();
            let mut steps: Vec<St> = Vec::new();
            let hyp_labels: Vec<&str> = hyps.iter().map(|h| h.1.as_str()).collect();
            let floats: HashMap<String, (String, String)> =
                self.active_floats().map(|(v, l, tc, _)| (l.clone(), (tc.clone(), v.clone()))).collect();
            let mut step = |lab: &str, vnum: &mut HashMap<String, u32>| -> Result<St, MmError> {
                let mut index_of = |u: (bool, Digest)| match uses.iter().position(|x| *x == u) {
                    Some(i) => i as u64,
                    None => {
                        uses.push(u);
                        uses.len() as u64 - 1
                    }
                };
                Ok(if lab == "?" {
                    St::Unknown
                } else if let Some(i) = hyp_labels.iter().position(|h| *h == lab) {
                    St::Hyp(i as u64)
                } else if let Some((tc, var)) = floats.get(lab) {
                    let n = vnum.len() as u32;
                    let v = *vnum.entry(var.clone()).or_insert(n);
                    let key = (tc.clone(), v);
                    St::Dummy(match dummies.iter().position(|x| *x == key) {
                        Some(i) => i as u64,
                        None => {
                            dummies.push(key);
                            dummies.len() as u64 - 1
                        }
                    })
                } else if let Some(d) = self.proofs.get(lab) {
                    // A theorem used: its proof, the justification the
                    // author chose (the proof names its statement).
                    St::Use(index_of((true, *d)))
                } else if let Some(d) = self.ids.get(lab) {
                    // An axiom, a definition, a syntax constructor: assumed.
                    St::Use(index_of((false, *d)))
                } else {
                    return Err(MmError::Syntax(format!("{label}: unknown label {lab} in proof"), self.pos));
                })
            };
            if proof.first() == Some(&"(") {
                let close = proof.iter().position(|t| *t == ")").ok_or(MmError::Syntax(format!("{label}: unclosed ("), self.pos))?;
                let labels = &proof[1..close];
                let letters: String = proof[close + 1..].concat();
                let m = hyp_labels.len() as u64;
                let n = labels.len() as u64;
                let mut num: u64 = 0;
                for c in letters.bytes() {
                    match c {
                        b'A'..=b'T' => {
                            let k = num * 20 + (c - b'A') as u64 + 1;
                            num = 0;
                            steps.push(if k <= m {
                                St::Hyp(k - 1)
                            } else if k <= m + n {
                                step(labels[(k - m - 1) as usize], &mut vnum)?
                            } else {
                                St::Saved(k - m - n - 1)
                            });
                        }
                        b'U'..=b'Y' => num = num * 5 + (c - b'U') as u64 + 1,
                        b'Z' => steps.push(St::Save),
                        b'?' => steps.push(St::Unknown),
                        _ => return self.err(format!("{label}: bad compressed proof character {}", c as char)),
                    }
                }
            } else {
                for lab in &proof {
                    steps.push(step(lab, &mut vnum)?);
                }
            }
            let mut pw = Writer::new();
            pw.str("mm", "proof-v1");
            pw.field("of", id.as_bytes());
            for (is_proof, d) in &uses {
                pw.field(if *is_proof { "proof" } else { "ref" }, d.as_bytes());
            }
            for (tc, v) in &dummies {
                let o = pw.open("dummy");
                pw.str("tc", tc);
                pw.u32("v", *v);
                pw.close(o);
            }
            let mut packed = Vec::with_capacity(steps.len() * 2);
            let mut varint = |mut x: u64| loop {
                let b = (x & 0x7f) as u8;
                x >>= 7;
                if x == 0 {
                    packed.push(b);
                    break;
                }
                packed.push(b | 0x80);
            };
            for st in &steps {
                varint(match *st {
                    St::Hyp(i) => i << 3,
                    St::Use(i) => (i << 3) | 1,
                    St::Dummy(i) => (i << 3) | 2,
                    St::Saved(i) => (i << 3) | 3,
                    St::Save => 4,
                    St::Unknown => 5,
                });
            }
            pw.field("steps", &packed);
            let refs: Vec<Digest> = uses.iter().map(|(_, d)| *d).collect();
            let pbytes = pw.finish();
            let pcomp = component_digest(LANG, NORMALISER, &pbytes);
            let pid = member_digest(&pcomp, 0);
            self.proofs.insert(label.to_string(), pid);
            proof_record = Some(record(pid, pcomp, "proof", pbytes, display, refs));
        }
        self.ids.insert(label.to_string(), id);
        self.out.push(Assertion { label: label.to_string(), role: role.into(), record: statement, proof: proof_record });
        Ok(())
    }

    fn run(&mut self) -> Result<(), MmError> {
        while self.pos < self.toks.len() {
            let t = self.toks[self.pos];
            self.pos += 1;
            self.order += 1;
            match t {
                "${" => self.frames.push(Frame::default()),
                "$}" => {
                    if self.frames.len() <= 1 {
                        return self.err("unbalanced $}");
                    }
                    self.frames.pop();
                }
                "$c" => {
                    self.until("$.")?;
                }
                "$v" => {
                    for v in self.until("$.")? {
                        self.vars.insert(v.to_string());
                    }
                }
                "$d" => {
                    let vs = self.until("$.")?;
                    let f = self.frames.last_mut().unwrap();
                    for i in 0..vs.len() {
                        for j in i + 1..vs.len() {
                            f.disjoint.push((vs[i].to_string(), vs[j].to_string()));
                        }
                    }
                }
                label => {
                    let Some(&k) = self.toks.get(self.pos) else { return self.err("unexpected end") };
                    self.pos += 1;
                    match k {
                        "$f" => {
                            let v = self.until("$.")?;
                            let [tc, var] = v[..] else { return self.err(format!("{label}: $f needs a typecode and a variable")) };
                            let order = self.order;
                            self.frames.last_mut().unwrap().floats.push((var.into(), label.into(), tc.into(), order));
                        }
                        "$e" => {
                            let v = self.until("$.")?;
                            let Some((tc, syms)) = v.split_first() else { return self.err("empty $e") };
                            let order = self.order;
                            self.frames.last_mut().unwrap().essentials.push((
                                label.into(),
                                tc.to_string(),
                                syms.iter().map(|s| s.to_string()).collect(),
                                order,
                            ));
                        }
                        "$a" | "$p" => self.assertion(label, k)?,
                        other => return self.err(format!("unexpected {other} after {label}")),
                    }
                }
            }
        }
        Ok(())
    }
}

/// Parses a Metamath database into assertions, in file order.
pub fn parse(text: &str) -> Result<Vec<Assertion>, MmError> {
    let mut p = Parser {
        toks: tokens(text)?,
        pos: 0,
        frames: vec![Frame::default()],
        vars: BTreeSet::new(),
        ids: HashMap::new(),
        proofs: HashMap::new(),
        out: Vec::new(),
        order: 0,
    };
    p.run()?;
    Ok(p.out)
}

/// The store name of a theorem's proof: `mmproof.<db>.<label>`.
pub fn proof_name_for(db: &str, label: &str) -> String {
    name_for(db, label).replacen("mm.", "mmproof.", 1)
}

/// The store name for a label in a database: `mm.<db>.<label>`, with the
/// label in backquotes when it is not a plain identifier.
pub fn name_for(db: &str, label: &str) -> String {
    let plain = label.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && label.chars().all(|c| c.is_alphanumeric() || c == '_');
    if plain { format!("mm.{db}.{label}") } else { format!("mm.{db}.`{label}`") }
}
