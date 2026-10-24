//! Canonical encoding and decoding of sophialang definitions (SPEC-003 §3),
//! and their display data. See `encode.md`.
//!
//! What is hashed: kind, declared type, body, usages, implicitness, `Unsafe`
//! marks, literal values (floats by bits), primitive names, field names,
//! hole types (holes numbered by first visit), unresolved paths, declared
//! rule weights. What is not: binder names and constructor names, which are
//! display data.

use sophia_hash::Digest;
use sophia_hash::canonical::{ReadError, Reader, Writer};

use crate::definition::{Body, Constructor, Definition, Field, Kind};
use crate::term::{Aspect, Binder, Case, Lit, Term, Usage};

/// How a `Term::Draft` is written while encoding one member of a component.
#[derive(Debug, Clone, Copy)]
pub enum Resolved {
    /// A member of the same component, by canonical index.
    Member(u32),
    /// A definition already hashed (an earlier component).
    Ref(Digest),
}

/// Encodes a definition. `draft(i)` says how `Term::Draft(i)` is written;
/// definitions without drafts may pass a function that panics.
pub fn encode(def: &Definition, draft: &dyn Fn(u32) -> Resolved) -> Vec<u8> {
    let mut e = Encoder { w: Writer::with_capacity(256), holes: 0, draft };
    e.definition(def);
    e.w.finish()
}

/// Encodes a definition that contains no drafts.
pub fn encode_resolved(def: &Definition) -> Vec<u8> {
    encode(def, &|i| panic!("unresolved draft {i} in encode_resolved"))
}

struct Encoder<'a> {
    w: Writer,
    holes: u32,
    draft: &'a dyn Fn(u32) -> Resolved,
}

impl Encoder<'_> {
    fn definition(&mut self, d: &Definition) {
        let o = self.w.open("def");
        self.w.str("kind", d.kind.as_str());
        if let Some(t) = &d.ty {
            let o = self.w.open("ty");
            self.term(t);
            self.w.close(o);
        }
        let o2 = self.w.open("body");
        match &d.body {
            Body::None => self.w.field("none", &[]),
            Body::Term(t) => {
                let o = self.w.open("term");
                self.term(t);
                self.w.close(o);
            }
            Body::Constructors(cs) => {
                let o = self.w.open("cons");
                for c in cs {
                    let oc = self.w.open("con");
                    self.term(&c.ty);
                    self.w.close(oc);
                }
                self.w.close(o);
            }
            Body::Fields(fs) => {
                let o = self.w.open("fields");
                for f in fs {
                    let of = self.w.open("field");
                    self.w.str("name", &f.name);
                    self.term(&f.ty);
                    self.w.close(of);
                }
                self.w.close(o);
            }
            Body::Rule { vars, head, premises, weight } => {
                let o = self.w.open("rule");
                let ov = self.w.open("vars");
                for b in vars {
                    self.binder(b);
                }
                self.w.close(ov);
                self.term(head);
                let op = self.w.open("prem");
                for p in premises {
                    self.term(p);
                }
                self.w.close(op);
                if let Some(l) = weight {
                    let ow = self.w.open("weight");
                    self.lit(l);
                    self.w.close(ow);
                }
                self.w.close(o);
            }
        }
        self.w.close(o2);
        self.w.close(o);
    }

    fn binder(&mut self, b: &Binder) {
        let usage = match b.usage {
            Usage::Zero => 0u8,
            Usage::One => 1,
            Usage::Many => 2,
        };
        self.w.field("B", &[usage, b.implicit as u8]);
    }

    fn lit(&mut self, l: &Lit) {
        match l {
            Lit::Int64(v) => self.w.field("i64", &v.to_le_bytes()),
            Lit::UInt64(v) => self.w.u64("u64", *v),
            Lit::Float64(bits) => self.w.u64("f64", *bits),
            Lit::String(s) => self.w.str("str", s),
            Lit::Bool(b) => self.w.field("bool", &[*b as u8]),
            Lit::Char(c) => self.w.u32("char", *c as u32),
        }
    }

    /// One level per call, through `deep::grow` (SP-015): any depth.
    fn term(&mut self, t: &Term) {
        sophia_hash::deep::grow(|| self.term_step(t))
    }

    fn term_step(&mut self, t: &Term) {
        match t {
            Term::Var(i) => self.w.u32("v", *i),
            Term::Ref(d) => self.w.field("r", d.as_bytes()),
            Term::Member(k) => self.w.u32("m", *k),
            Term::Draft(i) => match (self.draft)(*i) {
                Resolved::Member(k) => self.w.u32("m", k),
                Resolved::Ref(d) => self.w.field("r", d.as_bytes()),
            },
            Term::Nominal(n) => self.w.field("n", n.as_bytes()),
            Term::Con { ty, index } => {
                let o = self.w.open("c");
                self.term(ty);
                self.w.u32("i", *index);
                self.w.close(o);
            }
            Term::Lam { binder, domain, body } => {
                let o = self.w.open("l");
                self.binder(binder);
                if let Some(d) = domain {
                    let od = self.w.open("dom");
                    self.term(d);
                    self.w.close(od);
                }
                self.term(body);
                self.w.close(o);
            }
            Term::App(h, args) => {
                let o = self.w.open("a");
                self.term(h);
                for a in args {
                    self.term(a);
                }
                self.w.close(o);
            }
            Term::Pi { binder, domain, codomain } => {
                let o = self.w.open("p");
                self.binder(binder);
                self.term(domain);
                self.term(codomain);
                self.w.close(o);
            }
            Term::Let { binder, ty, value, body } => {
                let o = self.w.open("e");
                self.binder(binder);
                if let Some(t) = ty {
                    let ot = self.w.open("ty");
                    self.term(t);
                    self.w.close(ot);
                }
                self.term(value);
                self.term(body);
                self.w.close(o);
            }
            Term::Match { scrutinee, data, motive, cases } => {
                let o = self.w.open("x");
                self.term(scrutinee);
                if let Some(d) = data {
                    let od = self.w.open("data");
                    self.term(d);
                    self.w.close(od);
                }
                if let Some(m) = motive {
                    let om = self.w.open("mot");
                    self.term(m);
                    self.w.close(om);
                }
                for c in cases {
                    let oc = self.w.open("case");
                    self.w.u32("i", c.constructor);
                    let ob = self.w.open("bs");
                    for b in &c.binders {
                        self.binder(b);
                    }
                    self.w.close(ob);
                    self.term(&c.body);
                    self.w.close(oc);
                }
                self.w.close(o);
            }
            Term::Type(l) => self.w.u32("t", *l),
            Term::Prop => self.w.field("P", &[]),
            Term::Lit(l) => self.lit(l),
            Term::Prim(name) => self.w.str("o", name),
            Term::Hole(ty) => {
                let o = self.w.open("?");
                self.w.u32("h", self.holes);
                self.holes += 1;
                self.term(ty);
                self.w.close(o);
            }
            Term::Unresolved(path) => {
                let o = self.w.open("!");
                for s in path {
                    self.w.str("s", s);
                }
                self.w.close(o);
            }
            Term::Unsafe(a, t) => {
                let o = self.w.open("U");
                self.w.str("aspect", a.as_str());
                self.term(t);
                self.w.close(o);
            }
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error(transparent)]
    Read(#[from] ReadError),
    #[error("unknown {what} {found:?}")]
    Unknown { what: &'static str, found: String },
    #[error("malformed {0}")]
    Malformed(&'static str),
}

/// Fields whose payload is a sequence of fields (definition structure and
/// terms); every other tag is a leaf.
const CONTAINERS: [&str; 26] = [
    "def", "ty", "body", "term", "cons", "con", "fields", "field", "rule", "vars", "prem", "weight", "c", "l", "dom",
    "a", "p", "e", "x", "data", "mot", "case", "bs", "?", "!", "U",
];

/// The definitions called directly in canonical bytes: the `Ref` heads of
/// applications, each once, in first-seen order. Reads the bytes with an
/// explicit stack and builds no terms, so it runs in constant call depth
/// whatever the nesting (the recursive decoder, `Term::walk` and dropping a
/// deep `Term` all recurse once per level, which overflowed a 2 MiB worker
/// stack on generated definitions; Moonkale P-229).
pub fn called_refs(bytes: &[u8]) -> Result<Vec<Digest>, DecodeError> {
    let mut out: Vec<Digest> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![Reader::new(bytes)];
    while let Some(mut r) = stack.pop() {
        while !r.is_empty() {
            let (tag, payload) = r.next()?;
            if !CONTAINERS.contains(&tag) {
                continue;
            }
            if tag == "a" {
                let mut head = Reader::new(payload);
                if let Ok(("r", d)) = head.next() {
                    let d = digest_of(d)?;
                    if seen.insert(d) {
                        out.push(d);
                    }
                }
            }
            stack.push(Reader::new(payload));
        }
    }
    Ok(out)
}

/// Decodes canonical bytes into a definition. Binder and constructor names
/// are empty until [`apply_display`] fills them in.
pub fn decode(bytes: &[u8]) -> Result<Definition, DecodeError> {
    let mut r = Reader::new(bytes);
    let mut d = r.enter("def")?;
    let kind = d.expect_str("kind")?;
    let kind = Kind::parse(kind).ok_or_else(|| DecodeError::Unknown { what: "kind", found: kind.into() })?;
    let ty = if d.peek_tag()? == "ty" { Some(term(&mut d.enter("ty")?)?) } else { None };
    let mut b = d.enter("body")?;
    let body = match b.peek_tag()? {
        "none" => {
            b.expect("none")?;
            Body::None
        }
        "term" => Body::Term(term(&mut b.enter("term")?)?),
        "cons" => {
            let mut cs = b.enter("cons")?;
            let mut out = Vec::new();
            while !cs.is_empty() {
                out.push(Constructor { name: String::new(), ty: term(&mut cs.enter("con")?)? });
            }
            Body::Constructors(out)
        }
        "fields" => {
            let mut fs = b.enter("fields")?;
            let mut out = Vec::new();
            while !fs.is_empty() {
                let mut f = fs.enter("field")?;
                let name = f.expect_str("name")?.to_string();
                out.push(Field { name, ty: term(&mut f)? });
            }
            Body::Fields(out)
        }
        "rule" => {
            let mut ru = b.enter("rule")?;
            let mut vs = ru.enter("vars")?;
            let mut vars = Vec::new();
            while !vs.is_empty() {
                vars.push(binder(&mut vs)?);
            }
            let head = term(&mut ru)?;
            let mut ps = ru.enter("prem")?;
            let mut premises = Vec::new();
            while !ps.is_empty() {
                premises.push(term(&mut ps)?);
            }
            let weight = if !ru.is_empty() { Some(lit_in(&mut ru.enter("weight")?)?) } else { None };
            Body::Rule { vars, head, premises, weight }
        }
        other => return Err(DecodeError::Unknown { what: "body", found: other.into() }),
    };
    Ok(Definition { kind, ty, body })
}

fn binder(r: &mut Reader) -> Result<Binder, DecodeError> {
    let p = r.expect("B")?;
    let [u, i] = p else { return Err(DecodeError::Malformed("binder")) };
    let usage = match u {
        0 => Usage::Zero,
        1 => Usage::One,
        2 => Usage::Many,
        _ => return Err(DecodeError::Malformed("usage")),
    };
    Ok(Binder { name: String::new(), usage, implicit: *i != 0 })
}

fn u64_of(p: &[u8]) -> Result<u64, DecodeError> {
    Ok(u64::from_le_bytes(p.try_into().map_err(|_| DecodeError::Malformed("u64"))?))
}

fn u32_of(p: &[u8]) -> Result<u32, DecodeError> {
    Ok(u32::from_le_bytes(p.try_into().map_err(|_| DecodeError::Malformed("u32"))?))
}

fn digest_of(p: &[u8]) -> Result<Digest, DecodeError> {
    Digest::from_slice(p).ok_or(DecodeError::Malformed("digest"))
}

fn lit_in(r: &mut Reader) -> Result<Lit, DecodeError> {
    let (tag, p) = r.next()?;
    lit(tag, p)?.ok_or_else(|| DecodeError::Unknown { what: "literal", found: tag.into() })
}

fn lit(tag: &str, p: &[u8]) -> Result<Option<Lit>, DecodeError> {
    Ok(Some(match tag {
        "i64" => Lit::Int64(u64_of(p)? as i64),
        "u64" => Lit::UInt64(u64_of(p)?),
        "f64" => Lit::Float64(u64_of(p)?),
        "str" => Lit::String(String::from_utf8(p.to_vec()).map_err(|_| DecodeError::Malformed("string"))?),
        "bool" => match p {
            [0] => Lit::Bool(false),
            [1] => Lit::Bool(true),
            _ => return Err(DecodeError::Malformed("bool")),
        },
        "char" => Lit::Char(char::from_u32(u32_of(p)?).ok_or(DecodeError::Malformed("char"))?),
        _ => return Ok(None),
    }))
}

/// One level per call, through `deep::grow` (SP-015): any depth.
fn term(r: &mut Reader) -> Result<Term, DecodeError> {
    sophia_hash::deep::grow(|| term_step(r))
}

fn term_step(r: &mut Reader) -> Result<Term, DecodeError> {
    let tag = r.peek_tag()?;
    Ok(match tag {
        "v" => Term::Var(r.expect_u32("v")?),
        "r" => Term::Ref(digest_of(r.expect("r")?)?),
        "m" => Term::Member(r.expect_u32("m")?),
        "n" => Term::Nominal(digest_of(r.expect("n")?)?),
        "c" => {
            let mut c = r.enter("c")?;
            let ty = term(&mut c)?;
            Term::Con { ty: Box::new(ty), index: c.expect_u32("i")? }
        }
        "l" => {
            let mut l = r.enter("l")?;
            let binder = binder(&mut l)?;
            let domain = if l.peek_tag()? == "dom" { Some(Box::new(term(&mut l.enter("dom")?)?)) } else { None };
            Term::Lam { binder, domain, body: Box::new(term(&mut l)?) }
        }
        "a" => {
            let mut a = r.enter("a")?;
            let h = term(&mut a)?;
            let mut args = Vec::new();
            while !a.is_empty() {
                args.push(term(&mut a)?);
            }
            Term::App(Box::new(h), args)
        }
        "p" => {
            let mut p = r.enter("p")?;
            let binder = binder(&mut p)?;
            let domain = Box::new(term(&mut p)?);
            Term::Pi { binder, domain, codomain: Box::new(term(&mut p)?) }
        }
        "e" => {
            let mut e = r.enter("e")?;
            let binder = binder(&mut e)?;
            let ty = if e.peek_tag()? == "ty" { Some(Box::new(term(&mut e.enter("ty")?)?)) } else { None };
            let value = Box::new(term(&mut e)?);
            Term::Let { binder, ty, value, body: Box::new(term(&mut e)?) }
        }
        "x" => {
            let mut x = r.enter("x")?;
            let scrutinee = Box::new(term(&mut x)?);
            let data = if !x.is_empty() && x.peek_tag()? == "data" {
                Some(Box::new(term(&mut x.enter("data")?)?))
            } else {
                None
            };
            let motive = if !x.is_empty() && x.peek_tag()? == "mot" {
                Some(Box::new(term(&mut x.enter("mot")?)?))
            } else {
                None
            };
            let mut cases = Vec::new();
            while !x.is_empty() {
                let mut c = x.enter("case")?;
                let constructor = c.expect_u32("i")?;
                let mut bs = c.enter("bs")?;
                let mut binders = Vec::new();
                while !bs.is_empty() {
                    binders.push(binder(&mut bs)?);
                }
                cases.push(Case { constructor, binders, body: term(&mut c)? });
            }
            Term::Match { scrutinee, data, motive, cases }
        }
        "t" => Term::Type(r.expect_u32("t")?),
        "P" => {
            r.expect("P")?;
            Term::Prop
        }
        "o" => Term::Prim(r.expect_str("o")?.to_string()),
        "?" => {
            let mut h = r.enter("?")?;
            h.expect_u32("h")?;
            Term::Hole(Box::new(term(&mut h)?))
        }
        "!" => {
            let mut u = r.enter("!")?;
            let mut path = Vec::new();
            while !u.is_empty() {
                path.push(u.expect_str("s")?.to_string());
            }
            Term::Unresolved(path)
        }
        "U" => {
            let mut u = r.enter("U")?;
            let a = u.expect_str("aspect")?;
            let a = Aspect::parse(a).ok_or_else(|| DecodeError::Unknown { what: "aspect", found: a.into() })?;
            Term::Unsafe(a, Box::new(term(&mut u)?))
        }
        _ => {
            let (tag, p) = r.next()?;
            return lit(tag, p)?.map(Term::Lit).ok_or_else(|| DecodeError::Unknown { what: "term", found: tag.into() });
        }
    })
}

/// Visits every display name of a definition (binder names and constructor
/// names) in canonical stream order.
pub fn for_each_name(def: &mut Definition, f: &mut impl FnMut(&mut String)) {
    fn term(t: &mut Term, f: &mut impl FnMut(&mut String)) {
        sophia_hash::deep::grow(|| term_step(t, f))
    }
    fn term_step(t: &mut Term, f: &mut impl FnMut(&mut String)) {
        match t {
            Term::Con { ty, .. } => term(ty, f),
            Term::Lam { binder, domain, body } => {
                f(&mut binder.name);
                if let Some(d) = domain {
                    term(d, f);
                }
                term(body, f);
            }
            Term::App(h, args) => {
                term(h, f);
                args.iter_mut().for_each(|a| term(a, f));
            }
            Term::Pi { binder, domain, codomain } => {
                f(&mut binder.name);
                term(domain, f);
                term(codomain, f);
            }
            Term::Let { binder, ty, value, body } => {
                f(&mut binder.name);
                if let Some(t) = ty {
                    term(t, f);
                }
                term(value, f);
                term(body, f);
            }
            Term::Match { scrutinee, data, motive, cases } => {
                term(scrutinee, f);
                if let Some(d) = data {
                    term(d, f);
                }
                if let Some(m) = motive {
                    term(m, f);
                }
                for c in cases {
                    c.binders.iter_mut().for_each(|b| f(&mut b.name));
                    term(&mut c.body, f);
                }
            }
            Term::Hole(t) | Term::Unsafe(_, t) => term(t, f),
            _ => {}
        }
    }
    if let Some(t) = &mut def.ty {
        term(t, f);
    }
    match &mut def.body {
        Body::None => {}
        Body::Term(t) => term(t, f),
        Body::Constructors(cs) => {
            for c in cs {
                f(&mut c.name);
                term(&mut c.ty, f);
            }
        }
        Body::Fields(fs) => fs.iter_mut().for_each(|x| term(&mut x.ty, f)),
        Body::Rule { vars, head, premises, .. } => {
            vars.iter_mut().for_each(|b| f(&mut b.name));
            term(head, f);
            premises.iter_mut().for_each(|p| term(p, f));
        }
    }
}

/// The display data of a definition: its names, in stream order, framed.
pub fn display_of(def: &Definition) -> Vec<u8> {
    let mut def = def.clone();
    let mut w = Writer::new();
    for_each_name(&mut def, &mut |n| w.str("n", n));
    w.finish()
}

/// Fills a decoded definition's names from its display data. Missing names
/// stay empty; extra names are ignored.
pub fn apply_display(def: &mut Definition, display: &[u8]) -> Result<(), DecodeError> {
    let mut r = Reader::new(display);
    let mut names = Vec::new();
    while !r.is_empty() {
        names.push(r.expect_str("n")?.to_string());
    }
    let mut it = names.into_iter();
    for_each_name(def, &mut |n| {
        if let Some(x) = it.next() {
            *n = x;
        }
    });
    Ok(())
}
