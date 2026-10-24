//! Elaboration: surface items → core definitions. Resolves names (locals,
//! the items of the same text, the store, built-ins, constructors), turns
//! binders into de Bruijn indices, and reports what did not resolve as
//! warnings (unresolved names are allowed: SPEC-003 §4). See `elab.md`.

use sophia_hash::Digest;
use sophialang_core::samples::builtin;
use sophialang_core::{Aspect, Binder, Body, Case, Constructor, Definition, Field, Kind, Lit, Term, Usage};

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
    scope: Scope,
    pub warnings: Vec<Diagnostic>,
}

/// Names in scope, innermost last, indexed by name, so a variable's de
/// Bruijn index costs O(1) instead of a scan of the scope (SP-015: deep
/// chains of binders).
#[derive(Default)]
struct Scope {
    names: Vec<String>,
    at: std::collections::HashMap<String, Vec<usize>>,
}

impl Scope {
    fn push(&mut self, n: String) {
        self.at.entry(n.clone()).or_default().push(self.names.len());
        self.names.push(n);
    }
    fn pop(&mut self) {
        if let Some(n) = self.names.pop() {
            if let Some(v) = self.at.get_mut(&n) {
                v.pop();
            }
        }
    }
    fn truncate(&mut self, len: usize) {
        while self.names.len() > len {
            self.pop();
        }
    }
    fn clear(&mut self) {
        self.truncate(0);
    }
    fn len(&self) -> usize {
        self.names.len()
    }
    /// The de Bruijn index of the innermost binder named `name`.
    fn index(&self, name: &str) -> Option<u32> {
        let i = *self.at.get(name)?.last()?;
        Some((self.names.len() - 1 - i) as u32)
    }
}

type R<T> = Result<T, Diagnostic>;

pub fn elaborate(env: &dyn Env, ns: &str, items: &[SItem]) -> R<(Vec<Item>, Vec<Diagnostic>)> {
    for (i, a) in items.iter().enumerate() {
        if items[..i].iter().any(|b| b.name == a.name) {
            return Err(Diagnostic::error(a.name_span, format!("`{}` is defined twice in this text", a.name)));
        }
    }
    let mut e = Elab { env, ns, items, scope: Scope::default(), warnings: Vec::new() };
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
        self.scope.index(name)
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

    fn binder(b: &SBinder) -> Binder {
        Binder { name: b.name.clone(), usage: b.usage, implicit: b.implicit }
    }

    /// Elaborates an expression with an explicit stack of steps instead of
    /// recursion (SP-015): any depth on every target, wasm included. Steps
    /// run in order; `Eval` pushes a term, `Build` pops its subterms.
    pub fn expr(&mut self, e: &SExpr) -> R<Term> {
        let mut steps: Vec<Step> = vec![Step::Eval(e)];
        let mut vals: Vec<Term> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Eval(e) => self.plan(e, &mut steps, &mut vals)?,
                Step::Push(n) => self.scope.push(n),
                Step::Pop => self.scope.pop(),
                Step::Truncate(n) => self.scope.truncate(n),
                Step::MatchCases(e) => self.plan_cases(e, &mut steps, &vals)?,
                Step::Build(b) => {
                    let t = self.build(b, &mut vals);
                    vals.push(t);
                }
            }
        }
        Ok(vals.pop().unwrap_or_default())
    }

    /// Plans one syntax node: a leaf's term goes to `vals`; an inner node
    /// pushes its steps.
    fn plan<'e>(&mut self, e: &'e SExpr, steps: &mut Vec<Step<'e>>, vals: &mut Vec<Term>) -> R<()> {
        let mut seq: Vec<Step<'e>> = Vec::new();
        let leaf = |vals: &mut Vec<Term>, t: Term| {
            vals.push(t);
            Ok(())
        };
        match &e.kind {
            SKind::Path(p) => {
                if p.len() == 1 {
                    if let Some(i) = self.local(&p[0]) {
                        return leaf(vals, Term::Var(i));
                    }
                }
                let t = match self.path(p) {
                    Some(t) => t,
                    None => {
                        self.warnings.push(Diagnostic::warning(e.span, format!("`{}` is not defined; stored as unresolved", p.join("."))));
                        Term::Unresolved(p.clone())
                    }
                };
                return leaf(vals, t);
            }
            SKind::HashRef(h) => return leaf(vals, Term::Ref(h.parse::<Digest>().map_err(|_| Diagnostic::error(e.span, "bad content id"))?)),
            SKind::NominalRef(h) => {
                return leaf(vals, Term::Nominal(h.parse::<Digest>().map_err(|_| Diagnostic::error(e.span, "bad nominal id"))?));
            }
            SKind::Int(v) => return leaf(vals, Term::Lit(Lit::Int64(*v))),
            SKind::UInt(v) => return leaf(vals, Term::Lit(Lit::UInt64(*v))),
            SKind::Float(b) => return leaf(vals, Term::Lit(Lit::Float64(*b))),
            SKind::Str(s) => return leaf(vals, Term::Lit(Lit::String(s.clone()))),
            SKind::Char(c) => return leaf(vals, Term::Lit(Lit::Char(*c))),
            SKind::Bool(b) => return leaf(vals, Term::Lit(Lit::Bool(*b))),
            SKind::Type(l) => return leaf(vals, Term::Type(*l)),
            SKind::Prop => return leaf(vals, Term::Prop),
            SKind::Prim(n) => return leaf(vals, Term::Prim(n.clone())),
            SKind::Member(k) => return leaf(vals, Term::Member(*k)),
            SKind::Hole(t) => seq.extend([Step::Eval(t), Step::Build(Build::Hole)]),
            SKind::Unsafe(a, t) => seq.extend([Step::Eval(t), Step::Build(Build::Unsafe(*a))]),
            SKind::ConIndex(t, j) => seq.extend([Step::Eval(t), Step::Build(Build::Con(*j))]),
            SKind::Call(h, args) => {
                seq.push(Step::Eval(h));
                seq.extend(args.iter().map(Step::Eval));
                seq.push(Step::Build(Build::App(args.len())));
            }
            SKind::BinOp(op, l, r) => seq.extend([Step::Eval(l), Step::Eval(r), Step::Build(Build::BinOp(op))]),
            SKind::Lam(b, body) => {
                seq.extend(b.ty.iter().map(|t| Step::Eval(t)));
                seq.extend([Step::Push(b.name.clone()), Step::Eval(body), Step::Pop]);
                seq.push(Step::Build(Build::Lam(Self::binder(b), b.ty.is_some())));
            }
            SKind::Pi(b, cod) => {
                seq.extend(b.ty.iter().map(|t| Step::Eval(t)));
                seq.extend([Step::Push(b.name.clone()), Step::Eval(cod), Step::Pop]);
                seq.push(Step::Build(Build::Pi(Self::binder(b), b.ty.is_some())));
            }
            SKind::Let(b, value, body) => {
                seq.extend(b.ty.iter().map(|t| Step::Eval(t)));
                seq.extend([Step::Eval(value), Step::Push(b.name.clone()), Step::Eval(body), Step::Pop]);
                seq.push(Step::Build(Build::Let(Self::binder(b), b.ty.is_some())));
            }
            SKind::Match { scrutinee, data, .. } => {
                seq.push(Step::Eval(scrutinee));
                seq.extend(data.iter().map(|d| Step::Eval(d)));
                // Constructor names come from the elaborated data type.
                seq.push(Step::MatchCases(e));
            }
            SKind::Annot(..) | SKind::Implicit(_) | SKind::Usage(..) => {
                return Err(Diagnostic::error(e.span, "a binder form is only allowed before `->` or `-->`"));
            }
        }
        steps.extend(seq.into_iter().rev());
        Ok(())
    }

    /// A match's cases, once its scrutinee and data type are elaborated (on
    /// top of `vals`): constructors resolved by name, bodies scheduled.
    fn plan_cases<'e>(&mut self, e: &'e SExpr, steps: &mut Vec<Step<'e>>, vals: &[Term]) -> R<()> {
        let SKind::Match { data, cases, .. } = &e.kind else { return Ok(()) };
        let names = if data.is_some() { vals.last().and_then(|d| self.ctor_names(d)) } else { None };
        let mut seq: Vec<Step<'e>> = Vec::new();
        let mut meta = Vec::new();
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
            seq.extend(c.binders.iter().map(|b| Step::Push(b.name.clone())));
            seq.extend([Step::Eval(&c.body), Step::Truncate(n0)]);
            let binders = c.binders.iter().map(|b| Binder { name: b.name.clone(), usage: Usage::Many, implicit: false }).collect();
            meta.push((constructor, binders));
        }
        seq.push(Step::Build(Build::Match { has_data: data.is_some(), cases: meta }));
        steps.extend(seq.into_iter().rev());
        Ok(())
    }

    /// Pops a node's subterms (in evaluation order) and builds it.
    fn build(&mut self, b: Build, vals: &mut Vec<Term>) -> Term {
        let mut pop = || vals.pop().unwrap_or_default();
        match b {
            Build::Hole => Term::Hole(Box::new(pop())),
            Build::Unsafe(a) => Term::Unsafe(a, Box::new(pop())),
            Build::Con(j) => Term::Con { ty: Box::new(pop()), index: j },
            Build::App(n) => {
                let mut args: Vec<Term> = (0..n).map(|_| pop()).collect();
                args.reverse();
                let h = pop();
                Term::App(Box::new(h), args)
            }
            Build::BinOp(op) => {
                let r = pop();
                let l = pop();
                Term::App(Box::new(Term::Nominal(builtin(op))), vec![l, r])
            }
            Build::Lam(binder, has_domain) => {
                let body = pop();
                let domain = if has_domain { Some(Box::new(pop())) } else { None };
                Term::Lam { binder, domain, body: Box::new(body) }
            }
            Build::Pi(binder, has_domain) => {
                let codomain = pop();
                let domain = if has_domain { pop() } else { any() };
                Term::Pi { binder, domain: Box::new(domain), codomain: Box::new(codomain) }
            }
            Build::Let(binder, has_ty) => {
                let body = pop();
                let value = pop();
                let ty = if has_ty { Some(Box::new(pop())) } else { None };
                Term::Let { binder, ty, value: Box::new(value), body: Box::new(body) }
            }
            Build::Match { has_data, cases } => {
                let mut bodies: Vec<Term> = (0..cases.len()).map(|_| pop()).collect();
                bodies.reverse();
                let data = if has_data { Some(Box::new(pop())) } else { None };
                let scrutinee = pop();
                let cases = cases
                    .into_iter()
                    .zip(bodies)
                    .map(|((constructor, binders), body)| Case { constructor, binders, body })
                    .collect();
                Term::Match { scrutinee: Box::new(scrutinee), data, motive: None, cases }
            }
        }
    }
}

/// One step of the elaborator's machine ([`Elab::expr`]).
enum Step<'e> {
    Eval(&'e SExpr),
    Push(String),
    Pop,
    Truncate(usize),
    /// Resolve a match's constructors and schedule its case bodies.
    MatchCases(&'e SExpr),
    Build(Build<'e>),
}

/// How to build a node from the subterms on top of the value stack.
enum Build<'e> {
    Hole,
    Unsafe(Aspect),
    Con(u32),
    App(usize),
    BinOp(&'e str),
    Lam(Binder, bool),
    Pi(Binder, bool),
    Let(Binder, bool),
    Match { has_data: bool, cases: Vec<(u32, Vec<Binder>)> },
}
