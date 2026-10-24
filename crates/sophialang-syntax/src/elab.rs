//! Elaboration: surface items → core definitions. Resolves names (locals,
//! the items of the same text, the store, built-ins, constructors), turns
//! binders into de Bruijn indices, and reports what did not resolve as
//! warnings (unresolved names are allowed: SPEC-003 §4). See `elab.md`.

use sophia_hash::Digest;
use sophialang_core::samples::builtin;
use sophialang_core::{Binder, Body, Case, Constructor, Definition, Field, Kind, Lit, Term, Usage};

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::resolve::{Env, Found, resolve_global};

/// One parsed definition. `Term::Draft(i)` refers to the i-th item.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Definition { name: String, def: Definition },
    /// A relation is nominal (S-ADR-0009): the store creates it, and its
    /// signature joins it.
    Relation { name: String, signature: Definition },
}

impl Item {
    pub fn name(&self) -> &str {
        match self {
            Item::Definition { name, .. } | Item::Relation { name, .. } => name,
        }
    }

    pub fn definition(&self) -> &Definition {
        match self {
            Item::Definition { def, .. } => def,
            Item::Relation { signature, .. } => signature,
        }
    }
}

pub struct Elab<'a> {
    pub env: &'a dyn Env,
    pub ns: &'a str,
    items: &'a [SItem],
    scope: Vec<String>,
    pub warnings: Vec<Diagnostic>,
}

type R<T> = Result<T, Diagnostic>;

pub fn elaborate(env: &dyn Env, ns: &str, items: &[SItem]) -> R<(Vec<Item>, Vec<Diagnostic>)> {
    for (i, a) in items.iter().enumerate() {
        if items[..i].iter().any(|b| b.name == a.name) {
            return Err(Diagnostic::error(a.name_span, format!("`{}` is defined twice in this text", a.name)));
        }
    }
    let mut e = Elab { env, ns, items, scope: Vec::new(), warnings: Vec::new() };
    let out = items.iter().map(|it| e.item(it)).collect::<R<Vec<_>>>()?;
    Ok((out, e.warnings))
}

fn any() -> Term {
    Term::Nominal(builtin("Any"))
}

impl Elab<'_> {
    fn item(&mut self, it: &SItem) -> R<Item> {
        self.scope.clear();
        let name = it.name.clone();
        let def = match &it.body {
            SBody::Sugar { params, ret, body } => {
                let mut binders = Vec::new();
                let mut domains = Vec::new();
                for p in params {
                    domains.push(match &p.ty {
                        Some(t) => self.expr(t)?,
                        None => any(),
                    });
                    binders.push(Binder { name: p.name.clone(), usage: p.usage, implicit: p.implicit });
                    self.scope.push(p.name.clone());
                }
                let ret = self.expr(ret)?;
                let body = self.expr(body)?;
                self.scope.clear();
                let mut ty = ret;
                for (b, d) in binders.iter().zip(domains).rev() {
                    ty = Term::Pi { binder: b.clone(), domain: Box::new(d), codomain: Box::new(ty) };
                }
                let mut value = body;
                for b in binders.iter().rev() {
                    value = Term::Lam { binder: Binder::named(b.name.clone()), domain: None, body: Box::new(value) };
                }
                Definition::method(ty, value)
            }
            SBody::Typed { ty, body } => {
                let ty = self.expr(ty)?;
                Definition::method(ty, self.expr(body)?)
            }
            SBody::Constant { ty, value } => {
                let ty = self.expr(ty)?;
                Definition::constant(ty, self.expr(value)?)
            }
            SBody::Data { ty, constructors } => {
                let ty = self.expr(ty)?;
                let cs = constructors
                    .iter()
                    .map(|(n, t)| Ok(Constructor { name: n.clone(), ty: self.expr(t)? }))
                    .collect::<R<Vec<_>>>()?;
                Definition::data(ty, cs)
            }
            SBody::Layout { fields } => Definition {
                kind: Kind::Layout,
                ty: None,
                body: Body::Fields(
                    fields.iter().map(|(n, t)| Ok(Field { name: n.clone(), ty: self.expr(t)? })).collect::<R<_>>()?,
                ),
            },
            SBody::Relation { ty } => {
                let signature = Definition { kind: Kind::Relation, ty: Some(self.expr(ty)?), body: Body::None };
                return Ok(Item::Relation { name, signature });
            }
            SBody::Postulate { ty } => Definition::postulate(self.expr(ty)?),
            SBody::Test { body } => Definition::test(self.expr(body)?),
            SBody::Rule { vars, weight, head, premises } => {
                let weight = match weight {
                    None => None,
                    Some(w) => match &self.expr(w)? {
                        Term::Lit(l) => Some(l.clone()),
                        _ => return Err(Diagnostic::error(w.span, "a rule weight must be a literal")),
                    },
                };
                for v in vars {
                    self.scope.push(v.name.clone());
                }
                let head = self.expr(head)?;
                let premises = premises.iter().map(|p| self.expr(p)).collect::<R<Vec<_>>>()?;
                let vars = vars.iter().map(|v| Binder::named(v.name.clone())).collect();
                Definition { kind: Kind::Rule, ty: None, body: Body::Rule { vars, head, premises, weight } }
            }
        };
        Ok(Item::Definition { name, def })
    }

    fn local(&self, name: &str) -> Option<u32> {
        self.scope.iter().rev().position(|n| n == name).map(|i| i as u32)
    }

    fn item_index(&self, name: &str) -> Option<u32> {
        self.items.iter().position(|it| it.name == name).map(|i| i as u32)
    }

    fn found(f: Found) -> Term {
        match f {
            Found::Content(d) => Term::Ref(d),
            Found::Nominal(n) => Term::Nominal(n),
        }
    }

    /// Constructor names of a data type given as a term.
    fn ctor_names(&self, ty: &Term) -> Option<Vec<String>> {
        match ty {
            Term::Draft(i) => match &self.items.get(*i as usize)?.body {
                SBody::Data { constructors, .. } => Some(constructors.iter().map(|c| c.0.clone()).collect()),
                _ => None,
            },
            Term::Ref(d) => self.env.constructors(d),
            _ => None,
        }
    }

    /// Resolves a non-local path to a term, or `None`.
    fn path(&self, path: &[String]) -> Option<Term> {
        let joined = path.join(".");
        if path.len() == 1 {
            if let Some(i) = self.item_index(&path[0]) {
                return Some(Term::Draft(i));
            }
        }
        if let Some(f) = resolve_global(self.env, self.ns, &joined) {
            return Some(Self::found(f));
        }
        // `T.c`: constructor `c` of data type `T`.
        if path.len() >= 2 {
            let (last, init) = path.split_last().unwrap();
            if let Some(ty) = self.path(init) {
                if let Some(j) = self.ctor_names(&ty).and_then(|cs| cs.iter().position(|c| c == last)) {
                    return Some(Term::Con { ty: Box::new(ty), index: j as u32 });
                }
            }
        }
        None
    }

    fn binder(&mut self, b: &SBinder) -> R<(Binder, Option<Term>)> {
        let ty = b.ty.as_ref().map(|t| self.expr(t)).transpose()?;
        Ok((Binder { name: b.name.clone(), usage: b.usage, implicit: b.implicit }, ty))
    }

    /// One level per call, through `deep::grow` (SP-015): any depth.
    pub fn expr(&mut self, e: &SExpr) -> R<Term> {
        sophia_hash::deep::grow(|| self.expr_step(e))
    }

    fn expr_step(&mut self, e: &SExpr) -> R<Term> {
        Ok(match &e.kind {
            SKind::Path(p) => {
                if p.len() == 1 {
                    if let Some(i) = self.local(&p[0]) {
                        return Ok(Term::Var(i));
                    }
                }
                match self.path(p) {
                    Some(t) => t,
                    None => {
                        self.warnings.push(Diagnostic::warning(e.span, format!("`{}` is not defined; stored as unresolved", p.join("."))));
                        Term::Unresolved(p.clone())
                    }
                }
            }
            SKind::HashRef(h) => Term::Ref(h.parse::<Digest>().map_err(|_| Diagnostic::error(e.span, "bad content id"))?),
            SKind::NominalRef(h) => {
                Term::Nominal(h.parse::<Digest>().map_err(|_| Diagnostic::error(e.span, "bad nominal id"))?)
            }
            SKind::Int(v) => Term::Lit(Lit::Int64(*v)),
            SKind::UInt(v) => Term::Lit(Lit::UInt64(*v)),
            SKind::Float(b) => Term::Lit(Lit::Float64(*b)),
            SKind::Str(s) => Term::Lit(Lit::String(s.clone())),
            SKind::Char(c) => Term::Lit(Lit::Char(*c)),
            SKind::Bool(b) => Term::Lit(Lit::Bool(*b)),
            SKind::Type(l) => Term::Type(*l),
            SKind::Prop => Term::Prop,
            SKind::Prim(n) => Term::Prim(n.clone()),
            SKind::Hole(t) => Term::Hole(Box::new(self.expr(t)?)),
            SKind::Unsafe(a, t) => Term::Unsafe(*a, Box::new(self.expr(t)?)),
            SKind::ConIndex(t, j) => Term::Con { ty: Box::new(self.expr(t)?), index: *j },
            SKind::Member(k) => Term::Member(*k),
            SKind::Call(h, args) => {
                let h = self.expr(h)?;
                Term::App(Box::new(h), args.iter().map(|a| self.expr(a)).collect::<R<_>>()?)
            }
            SKind::BinOp(op, l, r) => {
                let l = self.expr(l)?;
                let r = self.expr(r)?;
                Term::App(Box::new(Term::Nominal(builtin(op))), vec![l, r])
            }
            SKind::Lam(b, body) => {
                let (binder, domain) = self.binder(b)?;
                self.scope.push(b.name.clone());
                let body = self.expr(body);
                self.scope.pop();
                Term::Lam { binder, domain: domain.map(Box::new), body: Box::new(body?) }
            }
            SKind::Pi(b, cod) => {
                let (binder, domain) = self.binder(b)?;
                let domain = domain.unwrap_or_else(any);
                self.scope.push(b.name.clone());
                let cod = self.expr(cod);
                self.scope.pop();
                Term::Pi { binder, domain: Box::new(domain), codomain: Box::new(cod?) }
            }
            SKind::Let(b, value, body) => {
                let (binder, ty) = self.binder(b)?;
                let value = self.expr(value)?;
                self.scope.push(b.name.clone());
                let body = self.expr(body);
                self.scope.pop();
                Term::Let { binder, ty: ty.map(Box::new), value: Box::new(value), body: Box::new(body?) }
            }
            SKind::Match { scrutinee, data, cases } => {
                let scrutinee = self.expr(scrutinee)?;
                let data = data.as_ref().map(|d| self.expr(d)).transpose()?;
                let names = data.as_ref().and_then(|d| self.ctor_names(d));
                let mut out = Vec::new();
                for c in cases {
                    let constructor = match &c.pattern {
                        Pattern::Index(j) => *j,
                        Pattern::Name(n) => names
                            .as_ref()
                            .and_then(|ns| ns.iter().position(|x| x == n))
                            .ok_or_else(|| Diagnostic::error(c.span, format!("`{n}` is not a constructor of the matched type")))?
                            as u32,
                    };
                    let n0 = self.scope.len();
                    for b in &c.binders {
                        self.scope.push(b.name.clone());
                    }
                    let body = self.expr(&c.body);
                    self.scope.truncate(n0);
                    out.push(Case {
                        constructor,
                        binders: c.binders.iter().map(|b| Binder { name: b.name.clone(), usage: Usage::Many, implicit: false }).collect(),
                        body: body?,
                    });
                }
                Term::Match { scrutinee: Box::new(scrutinee), data: data.map(Box::new), motive: None, cases: out }
            }
            SKind::Annot(..) | SKind::Implicit(_) | SKind::Usage(..) => {
                return Err(Diagnostic::error(e.span, "a binder form is only allowed before `->` or `-->`"));
            }
        })
    }
}
