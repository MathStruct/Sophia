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

    /// Writes one term with an explicit stack (SP-015): any depth on every
    /// target. Fields come out in the same order as a recursive pre-order
    /// walk, so the bytes, and the ids, are unchanged.
    fn term(&mut self, t: &Term) {
        enum W<'t> {
            Visit(&'t Term),
            /// Open a wrapper field (`dom`, `ty`, `data`, `mot`) around a term.
            Wrap(&'static str, &'t Term),
            Case(&'t Case),
            Close(sophia_hash::canonical::Open),
            /// Close, then write a trailing u32 field (`Con`'s index).
            CloseAfter(sophia_hash::canonical::Open, u32),
        }
        let mut work = vec![W::Visit(t)];
        while let Some(w) = work.pop() {
            let t = match w {
                W::Close(o) => {
                    self.w.close(o);
                    continue;
                }
                W::CloseAfter(o, i) => {
                    self.w.u32("i", i);
                    self.w.close(o);
                    continue;
                }
                W::Wrap(tag, t) => {
                    let o = self.w.open(tag);
                    work.push(W::Close(o));
                    work.push(W::Visit(t));
                    continue;
                }
                W::Case(c) => {
                    let oc = self.w.open("case");
                    self.w.u32("i", c.constructor);
                    let ob = self.w.open("bs");
                    for b in &c.binders {
                        self.binder(b);
                    }
                    self.w.close(ob);
                    work.push(W::Close(oc));
                    work.push(W::Visit(&c.body));
                    continue;
                }
                W::Visit(t) => t,
            };
            // Items are pushed in reverse: the last pushed is written first.
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
                    work.push(W::CloseAfter(o, *index));
                    work.push(W::Visit(ty));
                }
                Term::Lam { binder, domain, body } => {
                    let o = self.w.open("l");
                    self.binder(binder);
                    work.push(W::Close(o));
                    work.push(W::Visit(body));
                    if let Some(d) = domain {
                        work.push(W::Wrap("dom", d));
                    }
                }
                Term::App(h, args) => {
                    let o = self.w.open("a");
                    work.push(W::Close(o));
                    work.extend(args.iter().rev().map(W::Visit));
                    work.push(W::Visit(h));
                }
                Term::Pi { binder, domain, codomain } => {
                    let o = self.w.open("p");
                    self.binder(binder);
                    work.push(W::Close(o));
                    work.push(W::Visit(codomain));
                    work.push(W::Visit(domain));
                }
                Term::Let { binder, ty, value, body } => {
                    let o = self.w.open("e");
                    self.binder(binder);
                    work.push(W::Close(o));
                    work.push(W::Visit(body));
                    work.push(W::Visit(value));
                    if let Some(t) = ty {
                        work.push(W::Wrap("ty", t));
                    }
                }
                Term::Match { scrutinee, data, motive, cases } => {
                    let o = self.w.open("x");
                    work.push(W::Close(o));
                    work.extend(cases.iter().rev().map(W::Case));
                    if let Some(m) = motive {
                        work.push(W::Wrap("mot", m));
                    }
                    if let Some(d) = data {
                        work.push(W::Wrap("data", d));
                    }
                    work.push(W::Visit(scrutinee));
                }
                Term::Type(l) => self.w.u32("t", *l),
                Term::Prop => self.w.field("P", &[]),
                Term::Lit(l) => self.lit(l),
                Term::Prim(name) => self.w.str("o", name),
                Term::Hole(ty) => {
                    let o = self.w.open("?");
                    self.w.u32("h", self.holes);
                    self.holes += 1;
                    work.push(W::Close(o));
                    work.push(W::Visit(ty));
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
                    work.push(W::Close(o));
                    work.push(W::Visit(t));
                }
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

/// Decodes one term with an explicit stack of frames, one per composite
/// term being read: any depth on every target, wasm included (SP-015).
fn term(r: &mut Reader) -> Result<Term, DecodeError> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut pending = match start(r)? {
        Started::Done(t) => return Ok(t),
        Started::Frame(f) => {
            stack.push(f);
            None
        }
    };
    loop {
        let top = stack.last_mut().expect("a frame");
        if let Some(out) = pending.take() {
            top.accept(out);
        }
        match top.next()? {
            Next::Child(Started::Done(t)) => pending = Some(Out::Term(t)),
            Next::Child(Started::Frame(f)) => stack.push(f),
            Next::Finished(out) => {
                stack.pop();
                if stack.is_empty() {
                    return match out {
                        Out::Term(t) => Ok(t),
                        Out::Case(_) => Err(DecodeError::Malformed("case outside a match")),
                    };
                }
                pending = Some(out);
            }
        }
    }
}

enum Started<'a> {
    Done(Term),
    Frame(Frame<'a>),
}

enum Out {
    Term(Term),
    Case(Case),
}

enum Next<'a> {
    Child(Started<'a>),
    Finished(Out),
}

enum Shape {
    Con,
    Lam { binder: Binder, has_dom: bool },
    App,
    Pi { binder: Binder },
    Let { binder: Binder, has_ty: bool },
    /// Stage: 0 scrutinee, 1 data?, 2 motive?, 3 cases.
    Match { stage: u8, data: Option<Term>, motive: Option<Term>, cases: Vec<Case> },
    Case { constructor: u32, binders: Vec<Binder> },
    Hole,
    Unsafe(Aspect),
}

/// A composite term being read: its own reader and the subterms so far.
struct Frame<'a> {
    kind: Shape,
    r: Reader<'a>,
    kids: Vec<Term>,
}

/// Reads a leaf term whole, or opens a frame for a composite one.
fn start<'a>(r: &mut Reader<'a>) -> Result<Started<'a>, DecodeError> {
    let tag = r.peek_tag()?;
    let frame = |kind, r| Ok(Started::Frame(Frame { kind, r, kids: Vec::new() }));
    Ok(Started::Done(match tag {
        "v" => Term::Var(r.expect_u32("v")?),
        "r" => Term::Ref(digest_of(r.expect("r")?)?),
        "m" => Term::Member(r.expect_u32("m")?),
        "n" => Term::Nominal(digest_of(r.expect("n")?)?),
        "c" => return frame(Shape::Con, r.enter("c")?),
        "l" => {
            let mut l = r.enter("l")?;
            let binder = binder(&mut l)?;
            let has_dom = l.peek_tag()? == "dom";
            return frame(Shape::Lam { binder, has_dom }, l);
        }
        "a" => return frame(Shape::App, r.enter("a")?),
        "p" => {
            let mut p = r.enter("p")?;
            let binder = binder(&mut p)?;
            return frame(Shape::Pi { binder }, p);
        }
        "e" => {
            let mut e = r.enter("e")?;
            let binder = binder(&mut e)?;
            let has_ty = e.peek_tag()? == "ty";
            return frame(Shape::Let { binder, has_ty }, e);
        }
        "x" => return frame(Shape::Match { stage: 0, data: None, motive: None, cases: Vec::new() }, r.enter("x")?),
        "t" => Term::Type(r.expect_u32("t")?),
        "P" => {
            r.expect("P")?;
            Term::Prop
        }
        "o" => Term::Prim(r.expect_str("o")?.to_string()),
        "?" => {
            let mut h = r.enter("?")?;
            h.expect_u32("h")?;
            return frame(Shape::Hole, h);
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
            return frame(Shape::Unsafe(a), u);
        }
        _ => {
            let (tag, p) = r.next()?;
            return lit(tag, p)?.map(|l| Started::Done(Term::Lit(l))).ok_or_else(|| DecodeError::Unknown { what: "term", found: tag.into() });
        }
    }))
}

impl<'a> Frame<'a> {
    fn accept(&mut self, out: Out) {
        match (&mut self.kind, out) {
            (Shape::Match { stage, data, motive, cases }, out) => match (*stage, out) {
                (0, Out::Term(t)) => {
                    self.kids.push(t);
                    *stage = 1;
                }
                (1, Out::Term(t)) => {
                    *data = Some(t);
                    *stage = 2;
                }
                (2, Out::Term(t)) => {
                    *motive = Some(t);
                    *stage = 3;
                }
                (_, Out::Case(c)) => cases.push(c),
                (_, Out::Term(t)) => self.kids.push(t),
            },
            (_, Out::Term(t)) => self.kids.push(t),
            (_, Out::Case(_)) => {}
        }
    }

    /// The next subterm to read, or the finished term.
    fn next(&mut self) -> Result<Next<'a>, DecodeError> {
        let n = self.kids.len();
        let child = |r: &mut Reader<'a>| -> Result<Next<'a>, DecodeError> { Ok(Next::Child(start(r)?)) };
        let wrapped = |r: &mut Reader<'a>, tag: &str| -> Result<Next<'a>, DecodeError> {
            let mut w = r.enter(tag)?;
            Ok(Next::Child(start(&mut w)?))
        };
        let mut kids = std::mem::take(&mut self.kids);
        let done = |t: Term| Ok(Next::Finished(Out::Term(t)));
        let res = match &mut self.kind {
            Shape::Con if n == 0 => child(&mut self.r),
            Shape::Con => done(Term::Con { ty: Box::new(kids.pop().unwrap_or_default()), index: self.r.expect_u32("i")? }),
            Shape::Lam { has_dom: true, .. } if n == 0 => wrapped(&mut self.r, "dom"),
            Shape::Lam { has_dom, .. } if n < 1 + *has_dom as usize => child(&mut self.r),
            Shape::Lam { binder, has_dom } => {
                let body = kids.pop().unwrap_or_default();
                let domain = if *has_dom { kids.pop().map(Box::new) } else { None };
                done(Term::Lam { binder: binder.clone(), domain, body: Box::new(body) })
            }
            Shape::App if n == 0 || !self.r.is_empty() => child(&mut self.r),
            Shape::App => {
                let h = kids.remove(0);
                done(Term::App(Box::new(h), std::mem::take(&mut kids)))
            }
            Shape::Pi { .. } if n < 2 => child(&mut self.r),
            Shape::Pi { binder } => {
                let codomain = kids.pop().unwrap_or_default();
                let domain = kids.pop().unwrap_or_default();
                done(Term::Pi { binder: binder.clone(), domain: Box::new(domain), codomain: Box::new(codomain) })
            }
            Shape::Let { has_ty: true, .. } if n == 0 => wrapped(&mut self.r, "ty"),
            Shape::Let { has_ty, .. } if n < 2 + *has_ty as usize => child(&mut self.r),
            Shape::Let { binder, has_ty } => {
                let body = kids.pop().unwrap_or_default();
                let value = kids.pop().unwrap_or_default();
                let ty = if *has_ty { kids.pop().map(Box::new) } else { None };
                done(Term::Let { binder: binder.clone(), ty, value: Box::new(value), body: Box::new(body) })
            }
            Shape::Match { stage, data, motive, cases } => loop {
                match *stage {
                    0 => break child(&mut self.r),
                    1 if !self.r.is_empty() && self.r.peek_tag()? == "data" => break wrapped(&mut self.r, "data"),
                    1 => *stage = 2,
                    2 if !self.r.is_empty() && self.r.peek_tag()? == "mot" => break wrapped(&mut self.r, "mot"),
                    2 => *stage = 3,
                    _ if !self.r.is_empty() => {
                        let mut c = self.r.enter("case")?;
                        let constructor = c.expect_u32("i")?;
                        let mut bs = c.enter("bs")?;
                        let mut binders = Vec::new();
                        while !bs.is_empty() {
                            binders.push(binder(&mut bs)?);
                        }
                        break Ok(Next::Child(Started::Frame(Frame { kind: Shape::Case { constructor, binders }, r: c, kids: Vec::new() })));
                    }
                    _ => {
                        let scrutinee = kids.pop().unwrap_or_default();
                        break done(Term::Match {
                            scrutinee: Box::new(scrutinee),
                            data: data.take().map(Box::new),
                            motive: motive.take().map(Box::new),
                            cases: std::mem::take(cases),
                        });
                    }
                }
            },
            Shape::Case { .. } if n == 0 => child(&mut self.r),
            Shape::Case { constructor, binders } => Ok(Next::Finished(Out::Case(Case {
                constructor: *constructor,
                binders: std::mem::take(binders),
                body: kids.pop().unwrap_or_default(),
            }))),
            Shape::Hole if n == 0 => child(&mut self.r),
            Shape::Hole => done(Term::Hole(Box::new(kids.pop().unwrap_or_default()))),
            Shape::Unsafe(_) if n == 0 => child(&mut self.r),
            Shape::Unsafe(a) => done(Term::Unsafe(*a, Box::new(kids.pop().unwrap_or_default()))),
        };
        self.kids = kids;
        res
    }
}

/// Visits every display name of a definition (binder names and constructor
/// names) in canonical stream order.
pub fn for_each_name(def: &mut Definition, f: &mut impl FnMut(&mut String)) {
    /// One term, pre-order, with an explicit stack: any depth (SP-015).
    fn term(t: &mut Term, f: &mut impl FnMut(&mut String)) {
        enum W<'t> {
            T(&'t mut Term),
            C(&'t mut Case),
        }
        let mut stack = vec![W::T(t)];
        while let Some(w) = stack.pop() {
            let t = match w {
                W::T(t) => t,
                W::C(c) => {
                    c.binders.iter_mut().for_each(|b| f(&mut b.name));
                    stack.push(W::T(&mut c.body));
                    continue;
                }
            };
            // Children pushed in reverse, so they are visited in order.
            match t {
                Term::Con { ty, .. } | Term::Hole(ty) | Term::Unsafe(_, ty) => stack.push(W::T(ty)),
                Term::Lam { binder, domain, body } => {
                    f(&mut binder.name);
                    stack.push(W::T(body));
                    if let Some(d) = domain {
                        stack.push(W::T(d));
                    }
                }
                Term::App(h, args) => {
                    stack.extend(args.iter_mut().rev().map(W::T));
                    stack.push(W::T(h));
                }
                Term::Pi { binder, domain, codomain } => {
                    f(&mut binder.name);
                    stack.push(W::T(codomain));
                    stack.push(W::T(domain));
                }
                Term::Let { binder, ty, value, body } => {
                    f(&mut binder.name);
                    stack.push(W::T(body));
                    stack.push(W::T(value));
                    if let Some(t) = ty {
                        stack.push(W::T(t));
                    }
                }
                Term::Match { scrutinee, data, motive, cases } => {
                    stack.extend(cases.iter_mut().rev().map(W::C));
                    if let Some(m) = motive {
                        stack.push(W::T(m));
                    }
                    if let Some(d) = data {
                        stack.push(W::T(d));
                    }
                    stack.push(W::T(scrutinee));
                }
                _ => {}
            }
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
