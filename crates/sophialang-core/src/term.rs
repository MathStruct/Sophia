//! Core terms of sophialang (SPEC-003 §2). See `term.md`.

use sophia_hash::{Digest, NominalId};

/// How often a bound variable may be used at run time (quantitative type
/// theory). `Zero` is erased, so types and proofs cost nothing (SPEC-002 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Usage {
    Zero,
    One,
    #[default]
    Many,
}

/// A binder. The name is display data and is not hashed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Binder {
    pub name: String,
    pub usage: Usage,
    pub implicit: bool,
}

impl Binder {
    pub fn named(name: impl Into<String>) -> Self {
        Binder { name: name.into(), ..Default::default() }
    }

    pub fn erased(name: impl Into<String>) -> Self {
        Binder { name: name.into(), usage: Usage::Zero, implicit: false }
    }

    pub fn implicit(name: impl Into<String>) -> Self {
        Binder { name: name.into(), usage: Usage::Zero, implicit: true }
    }
}

/// Literals. Floats are kept as their bits, so equality and hashing are exact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Lit {
    Int64(i64),
    UInt64(u64),
    Float64(u64),
    String(String),
    Bool(bool),
    Char(char),
}

impl Lit {
    pub fn float(x: f64) -> Self {
        Lit::Float64(x.to_bits())
    }
}

/// What an `Unsafe` section takes explicit control of (SPEC-002 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Aspect {
    Memory,
    Concurrency,
    Compilation,
}

impl Aspect {
    pub fn as_str(self) -> &'static str {
        match self {
            Aspect::Memory => "memory",
            Aspect::Concurrency => "concurrency",
            Aspect::Compilation => "compilation",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "memory" => Aspect::Memory,
            "concurrency" => Aspect::Concurrency,
            "compilation" => Aspect::Compilation,
            _ => return None,
        })
    }
}

/// One case of a `Match`: a constructor index, its fields' binders, a body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Case {
    pub constructor: u32,
    pub binders: Vec<Binder>,
    pub body: Term,
}

/// A core term. `Clone`, `PartialEq`, `Hash`, `Debug` and `Drop` are
/// written by hand below, to work at any depth (SP-015).
pub enum Term {
    /// A bound variable, as a de Bruijn index (0 = innermost binder).
    Var(u32),
    /// A content-addressed definition outside this definition's group.
    Ref(Digest),
    /// Member `k` of this definition's recursive group (canonical index).
    Member(u32),
    /// Before hashing only: definition `i` of the batch being normalised.
    Draft(u32),
    /// A nominal entity: a generic function, struct, abstract or primitive type.
    Nominal(NominalId),
    /// Constructor `index` of the data type or struct `ty`.
    Con { ty: Box<Term>, index: u32 },
    Lam { binder: Binder, domain: Option<Box<Term>>, body: Box<Term> },
    /// n-ary application: dispatch sees all arguments together.
    App(Box<Term>, Vec<Term>),
    Pi { binder: Binder, domain: Box<Term>, codomain: Box<Term> },
    Let { binder: Binder, ty: Option<Box<Term>>, value: Box<Term>, body: Box<Term> },
    /// `data` is the data type being matched, when known: it names the
    /// constructors in printed patterns, and it is hashed.
    Match { scrutinee: Box<Term>, data: Option<Box<Term>>, motive: Option<Box<Term>>, cases: Vec<Case> },
    Type(u32),
    Prop,
    Lit(Lit),
    /// A built-in operation, by name, from the normaliser's versioned table.
    Prim(String),
    /// A hole with its expected type.
    Hole(Box<Term>),
    /// A name that did not resolve, as its path segments.
    Unresolved(Vec<String>),
    Unsafe(Aspect, Box<Term>),
}

impl Term {
    pub fn app(f: Term, args: impl IntoIterator<Item = Term>) -> Term {
        Term::App(Box::new(f), args.into_iter().collect())
    }

    pub fn lam(name: &str, body: Term) -> Term {
        Term::Lam { binder: Binder::named(name), domain: None, body: Box::new(body) }
    }

    pub fn lam_typed(name: &str, domain: Term, body: Term) -> Term {
        Term::Lam { binder: Binder::named(name), domain: Some(Box::new(domain)), body: Box::new(body) }
    }

    pub fn pi(name: &str, domain: Term, codomain: Term) -> Term {
        Term::Pi { binder: Binder::named(name), domain: Box::new(domain), codomain: Box::new(codomain) }
    }

    pub fn arrow(domain: Term, codomain: Term) -> Term {
        Term::pi("_", domain, codomain)
    }

    pub fn prim(name: &str) -> Term {
        Term::Prim(name.into())
    }

    pub fn hole(ty: Term) -> Term {
        Term::Hole(Box::new(ty))
    }

    /// Visits every subterm in pre-order (the canonical stream order), with
    /// an explicit stack: any depth on every target (SP-015).
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a Term)) {
        let mut stack = vec![self];
        while let Some(t) = stack.pop() {
            f(t);
            let kids = t.children();
            stack.extend(kids.into_iter().rev());
        }
    }

    /// The direct subterms, in canonical stream order.
    pub fn children(&self) -> Vec<&Term> {
        match self {
            Term::Con { ty, .. } | Term::Hole(ty) | Term::Unsafe(_, ty) => vec![ty],
            Term::Lam { domain, body, .. } => domain.iter().map(|d| d.as_ref()).chain([body.as_ref()]).collect(),
            Term::App(h, args) => std::iter::once(h.as_ref()).chain(args.iter()).collect(),
            Term::Pi { domain, codomain, .. } => vec![domain, codomain],
            Term::Let { ty, value, body, .. } => ty.iter().map(|t| t.as_ref()).chain([value.as_ref(), body.as_ref()]).collect(),
            Term::Match { scrutinee, data, motive, cases } => std::iter::once(scrutinee.as_ref())
                .chain(data.iter().map(|d| d.as_ref()))
                .chain(motive.iter().map(|m| m.as_ref()))
                .chain(cases.iter().map(|c| &c.body))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Rewrites every subterm bottom-up with `f` (used to resolve drafts),
    /// with an explicit stack: any depth on every target (SP-015). A node's
    /// subterms are taken out, mapped, put back, then `f` sees the node.
    pub fn map(self, f: &mut impl FnMut(Term) -> Term) -> Term {
        enum W {
            Visit(Term),
            /// A node whose `n` subterms are being mapped.
            Build(Term, usize),
        }
        let mut done: Vec<Term> = Vec::new();
        let mut work = vec![W::Visit(self)];
        while let Some(w) = work.pop() {
            match w {
                W::Visit(mut t) => {
                    let kids: Vec<Term> = slots(&mut t).into_iter().map(std::mem::take).collect();
                    work.push(W::Build(t, kids.len()));
                    work.extend(kids.into_iter().rev().map(W::Visit));
                }
                W::Build(mut t, n) => {
                    let kids = done.split_off(done.len() - n);
                    for (slot, k) in slots(&mut t).into_iter().zip(kids) {
                        *slot = k;
                    }
                    done.push(f(t));
                }
            }
        }
        done.pop().unwrap_or_default()
    }

    /// Moves every direct subterm into `out`, leaving cheap leaves: the
    /// step of the iterative `Drop`.
    fn take_children(&mut self, out: &mut Vec<Term>) {
        let mut take = |b: &mut Term| {
            if !b.is_leaf() {
                out.push(std::mem::take(b));
            }
        };
        match self {
            Term::Con { ty, .. } => take(ty),
            Term::Lam { domain, body, .. } => {
                if let Some(d) = domain {
                    take(d);
                }
                take(body);
            }
            Term::App(h, args) => {
                take(h);
                args.iter_mut().for_each(&mut take);
            }
            Term::Pi { domain, codomain, .. } => {
                take(domain);
                take(codomain);
            }
            Term::Let { ty, value, body, .. } => {
                if let Some(t) = ty {
                    take(t);
                }
                take(value);
                take(body);
            }
            Term::Match { scrutinee, data, motive, cases } => {
                take(scrutinee);
                if let Some(d) = data {
                    take(d);
                }
                if let Some(m) = motive {
                    take(m);
                }
                cases.iter_mut().for_each(|c| take(&mut c.body));
            }
            Term::Hole(t) | Term::Unsafe(_, t) => take(t),
            _ => {}
        }
    }

    fn is_leaf(&self) -> bool {
        !matches!(
            self,
            Term::Con { .. }
                | Term::Lam { .. }
                | Term::App(..)
                | Term::Pi { .. }
                | Term::Let { .. }
                | Term::Match { .. }
                | Term::Hole(_)
                | Term::Unsafe(..)
        )
    }
}

/// `Prop`: the cheap leaf left behind when a subterm is taken out.
impl Default for Term {
    fn default() -> Self {
        Term::Prop
    }
}

/// Dropping a term of any depth uses a heap stack, not the call stack:
/// the default drop of nested `Box`es recurses once per level (SP-015).
impl Drop for Term {
    fn drop(&mut self) {
        if self.is_leaf() {
            return;
        }
        let mut stack = Vec::new();
        self.take_children(&mut stack);
        while let Some(mut t) = stack.pop() {
            t.take_children(&mut stack);
        }
    }
}


/// The node itself with its subterms replaced by `Prop` placeholders: what
/// `Clone` copies at each step, and what `PartialEq` and `Hash` compare.
fn shallow(t: &Term) -> Term {
    let p = || Box::new(Term::Prop);
    match t {
        Term::Var(x) => Term::Var(*x),
        Term::Ref(x) => Term::Ref(*x),
        Term::Member(x) => Term::Member(*x),
        Term::Draft(x) => Term::Draft(*x),
        Term::Nominal(x) => Term::Nominal(*x),
        Term::Con { index, .. } => Term::Con { ty: p(), index: *index },
        Term::Lam { binder, domain, .. } => Term::Lam { binder: binder.clone(), domain: domain.as_ref().map(|_| p()), body: p() },
        Term::App(_, args) => Term::App(p(), args.iter().map(|_| Term::Prop).collect()),
        Term::Pi { binder, .. } => Term::Pi { binder: binder.clone(), domain: p(), codomain: p() },
        Term::Let { binder, ty, .. } => Term::Let { binder: binder.clone(), ty: ty.as_ref().map(|_| p()), value: p(), body: p() },
        Term::Match { data, motive, cases, .. } => Term::Match {
            scrutinee: p(),
            data: data.as_ref().map(|_| p()),
            motive: motive.as_ref().map(|_| p()),
            cases: cases.iter().map(|c| Case { constructor: c.constructor, binders: c.binders.clone(), body: Term::Prop }).collect(),
        },
        Term::Type(x) => Term::Type(*x),
        Term::Prop => Term::Prop,
        Term::Lit(x) => Term::Lit(x.clone()),
        Term::Prim(x) => Term::Prim(x.clone()),
        Term::Hole(_) => Term::Hole(p()),
        Term::Unresolved(x) => Term::Unresolved(x.clone()),
        Term::Unsafe(a, _) => Term::Unsafe(*a, p()),
    }
}

/// The subterm slots of a node, mutable, in canonical order.
fn slots(t: &mut Term) -> Vec<&mut Term> {
    match t {
        Term::Con { ty, .. } | Term::Hole(ty) | Term::Unsafe(_, ty) => vec![ty],
        Term::Lam { domain, body, .. } => domain.iter_mut().map(|d| d.as_mut()).chain([body.as_mut()]).collect(),
        Term::App(h, args) => std::iter::once(h.as_mut()).chain(args.iter_mut()).collect(),
        Term::Pi { domain, codomain, .. } => vec![domain, codomain],
        Term::Let { ty, value, body, .. } => ty.iter_mut().map(|t| t.as_mut()).chain([value.as_mut(), body.as_mut()]).collect(),
        Term::Match { scrutinee, data, motive, cases } => std::iter::once(scrutinee.as_mut())
            .chain(data.iter_mut().map(|d| d.as_mut()))
            .chain(motive.iter_mut().map(|m| m.as_mut()))
            .chain(cases.iter_mut().map(|c| &mut c.body))
            .collect(),
        _ => Vec::new(),
    }
}

// Clone, PartialEq and Hash by hand, with explicit stacks: copying,
// comparing or hashing a term of any depth works on every target, wasm
// included (SP-015). They agree with what the derived impls did.

impl Clone for Term {
    fn clone(&self) -> Term {
        // Post-order: a node is built once its subterms are.
        enum W<'t> {
            Visit(&'t Term),
            Build(&'t Term),
        }
        let mut done: Vec<Term> = Vec::new();
        let mut work = vec![W::Visit(self)];
        while let Some(w) = work.pop() {
            match w {
                W::Visit(t) => {
                    work.push(W::Build(t));
                    work.extend(t.children().into_iter().rev().map(W::Visit));
                }
                W::Build(t) => {
                    let mut node = shallow(t);
                    let n = t.children().len();
                    let kids = done.split_off(done.len() - n);
                    for (slot, k) in slots(&mut node).into_iter().zip(kids) {
                        *slot = k;
                    }
                    done.push(node);
                }
            }
        }
        done.pop().unwrap_or_default()
    }
}

/// Everything about a node but its subterms, for `PartialEq` and `Hash`.
fn same_node(a: &Term, b: &Term) -> bool {
    match (a, b) {
        (Term::Var(x), Term::Var(y)) | (Term::Member(x), Term::Member(y)) | (Term::Draft(x), Term::Draft(y)) | (Term::Type(x), Term::Type(y)) => x == y,
        (Term::Ref(x), Term::Ref(y)) | (Term::Nominal(x), Term::Nominal(y)) => x == y,
        (Term::Con { index: x, .. }, Term::Con { index: y, .. }) => x == y,
        (Term::Lam { binder: x, domain: dx, .. }, Term::Lam { binder: y, domain: dy, .. }) => x == y && dx.is_some() == dy.is_some(),
        (Term::App(_, x), Term::App(_, y)) => x.len() == y.len(),
        (Term::Pi { binder: x, .. }, Term::Pi { binder: y, .. }) => x == y,
        (Term::Let { binder: x, ty: tx, .. }, Term::Let { binder: y, ty: ty_, .. }) => x == y && tx.is_some() == ty_.is_some(),
        (Term::Match { data: dx, motive: mx, cases: cx, .. }, Term::Match { data: dy, motive: my, cases: cy, .. }) => {
            dx.is_some() == dy.is_some()
                && mx.is_some() == my.is_some()
                && cx.len() == cy.len()
                && cx.iter().zip(cy).all(|(a, b)| a.constructor == b.constructor && a.binders == b.binders)
        }
        (Term::Prop, Term::Prop) => true,
        (Term::Lit(x), Term::Lit(y)) => x == y,
        (Term::Prim(x), Term::Prim(y)) => x == y,
        (Term::Hole(_), Term::Hole(_)) => true,
        (Term::Unresolved(x), Term::Unresolved(y)) => x == y,
        (Term::Unsafe(x, _), Term::Unsafe(y, _)) => x == y,
        _ => false,
    }
}

impl PartialEq for Term {
    fn eq(&self, other: &Term) -> bool {
        let mut stack = vec![(self, other)];
        while let Some((a, b)) = stack.pop() {
            if !same_node(a, b) {
                return false;
            }
            stack.extend(a.children().into_iter().zip(b.children()));
        }
        true
    }
}

impl Eq for Term {}

impl std::hash::Hash for Term {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut stack = vec![self];
        while let Some(t) = stack.pop() {
            // The node without its subterms, then its subterms in order.
            std::mem::discriminant(t).hash(state);
            match t {
                Term::Var(x) | Term::Member(x) | Term::Draft(x) | Term::Type(x) => x.hash(state),
                Term::Ref(x) | Term::Nominal(x) => x.hash(state),
                Term::Con { index, .. } => index.hash(state),
                Term::Lam { binder, domain, .. } => (binder, domain.is_some()).hash(state),
                Term::App(_, args) => args.len().hash(state),
                Term::Pi { binder, .. } => binder.hash(state),
                Term::Let { binder, ty, .. } => (binder, ty.is_some()).hash(state),
                Term::Match { data, motive, cases, .. } => {
                    (data.is_some(), motive.is_some(), cases.len()).hash(state);
                    for c in cases {
                        (c.constructor, &c.binders).hash(state);
                    }
                }
                Term::Lit(x) => x.hash(state),
                Term::Prim(x) => x.hash(state),
                Term::Unresolved(x) => x.hash(state),
                Term::Unsafe(a, _) => a.hash(state),
                Term::Prop | Term::Hole(_) => {}
            }
            stack.extend(t.children().into_iter().rev());
        }
    }
}

/// `Debug` as derive printed it, without recursion (SP-015, [`crate::debug`]).
impl std::fmt::Debug for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::debug::{DNode, opt, render};
        fn ob(o: &Option<Box<Term>>) -> DNode<'_, Term> {
            opt(o.as_deref())
        }
        fn one(t: &Term, alt: bool) -> DNode<'_, Term> {
            let l = |v: &dyn std::fmt::Debug| DNode::Leaf(if alt { format!("{v:#?}") } else { format!("{v:?}") });
            let c = DNode::Child;
            match t {
                Term::Var(x) => DNode::Tuple("Var", vec![l(x)]),
                Term::Ref(x) => DNode::Tuple("Ref", vec![l(x)]),
                Term::Member(x) => DNode::Tuple("Member", vec![l(x)]),
                Term::Draft(x) => DNode::Tuple("Draft", vec![l(x)]),
                Term::Nominal(x) => DNode::Tuple("Nominal", vec![l(x)]),
                Term::Con { ty, index } => DNode::Struct("Con", vec![("ty", c(ty)), ("index", l(index))]),
                Term::Lam { binder, domain, body } => {
                    DNode::Struct("Lam", vec![("binder", l(binder)), ("domain", ob(domain)), ("body", c(body))])
                }
                Term::App(h, args) => DNode::Tuple("App", vec![c(h), DNode::List(args.iter().map(c).collect())]),
                Term::Pi { binder, domain, codomain } => {
                    DNode::Struct("Pi", vec![("binder", l(binder)), ("domain", c(domain)), ("codomain", c(codomain))])
                }
                Term::Let { binder, ty, value, body } => DNode::Struct(
                    "Let",
                    vec![("binder", l(binder)), ("ty", ob(ty)), ("value", c(value)), ("body", c(body))],
                ),
                Term::Match { scrutinee, data, motive, cases } => DNode::Struct(
                    "Match",
                    vec![
                        ("scrutinee", c(scrutinee)),
                        ("data", ob(data)),
                        ("motive", ob(motive)),
                        (
                            "cases",
                            DNode::List(
                                cases
                                    .iter()
                                    .map(|k| {
                                        DNode::Struct(
                                            "Case",
                                            vec![("constructor", l(&k.constructor)), ("binders", l(&k.binders)), ("body", c(&k.body))],
                                        )
                                    })
                                    .collect(),
                            ),
                        ),
                    ],
                ),
                Term::Type(x) => DNode::Tuple("Type", vec![l(x)]),
                Term::Prop => DNode::Tuple("Prop", vec![]),
                Term::Lit(x) => DNode::Tuple("Lit", vec![l(x)]),
                Term::Prim(x) => DNode::Tuple("Prim", vec![l(x)]),
                Term::Hole(x) => DNode::Tuple("Hole", vec![c(x)]),
                Term::Unresolved(x) => DNode::Tuple("Unresolved", vec![l(x)]),
                Term::Unsafe(a, x) => DNode::Tuple("Unsafe", vec![l(a), c(x)]),
            }
        }
        render(f, self, one)
    }
}
