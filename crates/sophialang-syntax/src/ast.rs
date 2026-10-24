//! The surface syntax tree: what the parser produces before names are
//! resolved and binders become de Bruijn indices. See `ast.md`.

use sophialang_core::{Aspect, Usage};

use crate::diagnostic::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct SBinder {
    pub name: String,
    pub ty: Option<Box<SExpr>>,
    pub usage: Usage,
    pub implicit: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SKind {
    /// A dotted path: `x`, `sample.add`, `Nat.succ`.
    Path(Vec<String>),
    HashRef(String),
    NominalRef(String),
    Int(i64),
    UInt(u64),
    Float(u64),
    Str(String),
    Char(char),
    Bool(bool),
    Type(u32),
    Prop,
    Prim(String),
    Hole(Box<SExpr>),
    Unsafe(Aspect, Box<SExpr>),
    /// `@con(T, j)`: a constructor by index, when it has no printable name.
    ConIndex(Box<SExpr>, u32),
    /// `@member(k)`: a member of the printed recursive group without a name.
    Member(u32),
    Call(Box<SExpr>, Vec<SExpr>),
    BinOp(String, Box<SExpr>, Box<SExpr>),
    Lam(SBinder, Box<SExpr>),
    Pi(SBinder, Box<SExpr>),
    Let(SBinder, Box<SExpr>, Box<SExpr>),
    Match { scrutinee: Box<SExpr>, data: Option<Box<SExpr>>, cases: Vec<SCase> },
    /// `(x::T)` and friends: only meaningful as a binder before `->`/`-->`.
    Annot(Box<SExpr>, Box<SExpr>),
    /// `{…}`: implicit binder.
    Implicit(Box<SExpr>),
    /// `@erased x`, `@linear x`: usage marker on a binder.
    Usage(Usage, Box<SExpr>),
}

/// `Clone`, `PartialEq`, `Debug` and `Drop` by hand, with explicit stacks:
/// a syntax tree of any depth can be copied, compared, printed and dropped
/// on every target (SP-015). They agree with what the derived impls did.
pub struct SExpr {
    pub kind: SKind,
    pub span: Span,
}

impl Default for SExpr {
    fn default() -> Self {
        SExpr { kind: SKind::Prop, span: Span::default() }
    }
}

impl SExpr {
    /// The direct subexpressions, in a fixed order (binder types included).
    pub fn children(&self) -> Vec<&SExpr> {
        fn bty(b: &SBinder) -> Option<&SExpr> {
            b.ty.as_deref()
        }
        match &self.kind {
            SKind::Hole(e) | SKind::Unsafe(_, e) | SKind::ConIndex(e, _) | SKind::Implicit(e) | SKind::Usage(_, e) => vec![e],
            SKind::Call(h, args) => std::iter::once(h.as_ref()).chain(args.iter()).collect(),
            SKind::BinOp(_, l, r) | SKind::Annot(l, r) => vec![l, r],
            SKind::Lam(b, body) | SKind::Pi(b, body) => bty(b).into_iter().chain([body.as_ref()]).collect(),
            SKind::Let(b, v, body) => bty(b).into_iter().chain([v.as_ref(), body.as_ref()]).collect(),
            SKind::Match { scrutinee, data, cases } => {
                let mut v: Vec<&SExpr> = vec![scrutinee];
                v.extend(data.as_deref());
                for c in cases {
                    v.extend(c.binders.iter().filter_map(bty));
                    v.push(&c.body);
                }
                v
            }
            _ => Vec::new(),
        }
    }

    /// The same subexpressions, mutable, in the same order.
    fn slots(&mut self) -> Vec<&mut SExpr> {
        match &mut self.kind {
            SKind::Hole(e) | SKind::Unsafe(_, e) | SKind::ConIndex(e, _) | SKind::Implicit(e) | SKind::Usage(_, e) => vec![e],
            SKind::Call(h, args) => std::iter::once(h.as_mut()).chain(args.iter_mut()).collect(),
            SKind::BinOp(_, l, r) | SKind::Annot(l, r) => vec![l, r],
            SKind::Lam(b, body) | SKind::Pi(b, body) => b.ty.as_deref_mut().into_iter().chain([body.as_mut()]).collect(),
            SKind::Let(b, v, body) => b.ty.as_deref_mut().into_iter().chain([v.as_mut(), body.as_mut()]).collect(),
            SKind::Match { scrutinee, data, cases } => {
                let mut v: Vec<&mut SExpr> = vec![scrutinee];
                v.extend(data.as_deref_mut());
                for c in cases {
                    v.extend(c.binders.iter_mut().filter_map(|b| b.ty.as_deref_mut()));
                    v.push(&mut c.body);
                }
                v
            }
            _ => Vec::new(),
        }
    }

    /// This node with every subexpression replaced by a placeholder.
    fn shallow(&self) -> SExpr {
        let p = || Box::new(SExpr::default());
        let b = |b: &SBinder| SBinder { ty: b.ty.as_ref().map(|_| p()), ..b.clone_shallow() };
        let kind = match &self.kind {
            SKind::Path(x) => SKind::Path(x.clone()),
            SKind::HashRef(x) => SKind::HashRef(x.clone()),
            SKind::NominalRef(x) => SKind::NominalRef(x.clone()),
            SKind::Int(x) => SKind::Int(*x),
            SKind::UInt(x) => SKind::UInt(*x),
            SKind::Float(x) => SKind::Float(*x),
            SKind::Str(x) => SKind::Str(x.clone()),
            SKind::Char(x) => SKind::Char(*x),
            SKind::Bool(x) => SKind::Bool(*x),
            SKind::Type(x) => SKind::Type(*x),
            SKind::Prop => SKind::Prop,
            SKind::Prim(x) => SKind::Prim(x.clone()),
            SKind::Hole(_) => SKind::Hole(p()),
            SKind::Unsafe(a, _) => SKind::Unsafe(*a, p()),
            SKind::ConIndex(_, j) => SKind::ConIndex(p(), *j),
            SKind::Member(k) => SKind::Member(*k),
            SKind::Call(_, args) => SKind::Call(p(), args.iter().map(|_| SExpr::default()).collect()),
            SKind::BinOp(op, _, _) => SKind::BinOp(op.clone(), p(), p()),
            SKind::Lam(x, _) => SKind::Lam(b(x), p()),
            SKind::Pi(x, _) => SKind::Pi(b(x), p()),
            SKind::Let(x, _, _) => SKind::Let(b(x), p(), p()),
            SKind::Match { data, cases, .. } => SKind::Match {
                scrutinee: p(),
                data: data.as_ref().map(|_| p()),
                cases: cases
                    .iter()
                    .map(|c| SCase { pattern: c.pattern.clone(), binders: c.binders.iter().map(b).collect(), body: SExpr::default(), span: c.span })
                    .collect(),
            },
            SKind::Annot(_, _) => SKind::Annot(p(), p()),
            SKind::Implicit(_) => SKind::Implicit(p()),
            SKind::Usage(u, _) => SKind::Usage(*u, p()),
        };
        SExpr { kind, span: self.span }
    }

    /// Everything but the subexpressions is equal.
    fn same_node(&self, o: &SExpr) -> bool {
        let sb = |a: &SBinder, b: &SBinder| {
            a.name == b.name && a.usage == b.usage && a.implicit == b.implicit && a.span == b.span && a.ty.is_some() == b.ty.is_some()
        };
        self.span == o.span
            && match (&self.kind, &o.kind) {
                (SKind::Hole(_), SKind::Hole(_)) | (SKind::Annot(..), SKind::Annot(..)) | (SKind::Implicit(_), SKind::Implicit(_)) => true,
                (SKind::Unsafe(a, _), SKind::Unsafe(b, _)) => a == b,
                (SKind::ConIndex(_, a), SKind::ConIndex(_, b)) => a == b,
                (SKind::Usage(a, _), SKind::Usage(b, _)) => a == b,
                (SKind::Call(_, a), SKind::Call(_, b)) => a.len() == b.len(),
                (SKind::BinOp(a, ..), SKind::BinOp(b, ..)) => a == b,
                (SKind::Lam(a, _), SKind::Lam(b, _)) | (SKind::Pi(a, _), SKind::Pi(b, _)) | (SKind::Let(a, ..), SKind::Let(b, ..)) => sb(a, b),
                (SKind::Match { data: da, cases: ca, .. }, SKind::Match { data: db, cases: cb, .. }) => {
                    da.is_some() == db.is_some()
                        && ca.len() == cb.len()
                        && ca.iter().zip(cb).all(|(x, y)| {
                            x.pattern == y.pattern
                                && x.span == y.span
                                && x.binders.len() == y.binders.len()
                                && x.binders.iter().zip(&y.binders).all(|(p, q)| sb(p, q))
                        })
                }
                (a, b) if a.children_free() && b.children_free() => a.leaf_eq(b),
                _ => false,
            }
    }
}

impl SKind {
    fn children_free(&self) -> bool {
        matches!(
            self,
            SKind::Path(_)
                | SKind::HashRef(_)
                | SKind::NominalRef(_)
                | SKind::Int(_)
                | SKind::UInt(_)
                | SKind::Float(_)
                | SKind::Str(_)
                | SKind::Char(_)
                | SKind::Bool(_)
                | SKind::Type(_)
                | SKind::Prop
                | SKind::Prim(_)
                | SKind::Member(_)
        )
    }

    /// Equality of two leaves (no subexpressions, so the derived-style
    /// comparison does not recurse).
    fn leaf_eq(&self, o: &SKind) -> bool {
        match (self, o) {
            (SKind::Path(a), SKind::Path(b)) => a == b,
            (SKind::HashRef(a), SKind::HashRef(b)) | (SKind::NominalRef(a), SKind::NominalRef(b)) => a == b,
            (SKind::Str(a), SKind::Str(b)) | (SKind::Prim(a), SKind::Prim(b)) => a == b,
            (SKind::Int(a), SKind::Int(b)) => a == b,
            (SKind::UInt(a), SKind::UInt(b)) | (SKind::Float(a), SKind::Float(b)) => a == b,
            (SKind::Char(a), SKind::Char(b)) => a == b,
            (SKind::Bool(a), SKind::Bool(b)) => a == b,
            (SKind::Type(a), SKind::Type(b)) | (SKind::Member(a), SKind::Member(b)) => a == b,
            (SKind::Prop, SKind::Prop) => true,
            _ => false,
        }
    }
}

impl SBinder {
    /// The binder without its type.
    fn clone_shallow(&self) -> SBinder {
        SBinder { name: self.name.clone(), ty: None, usage: self.usage, implicit: self.implicit, span: self.span }
    }
}

impl Clone for SExpr {
    fn clone(&self) -> SExpr {
        enum W<'e> {
            Visit(&'e SExpr),
            Build(&'e SExpr),
        }
        let mut done: Vec<SExpr> = Vec::new();
        let mut work = vec![W::Visit(self)];
        while let Some(w) = work.pop() {
            match w {
                W::Visit(e) => {
                    work.push(W::Build(e));
                    work.extend(e.children().into_iter().rev().map(W::Visit));
                }
                W::Build(e) => {
                    let mut node = e.shallow();
                    let kids = done.split_off(done.len() - e.children().len());
                    for (slot, k) in node.slots().into_iter().zip(kids) {
                        *slot = k;
                    }
                    done.push(node);
                }
            }
        }
        done.pop().unwrap_or_default()
    }
}

impl PartialEq for SExpr {
    fn eq(&self, other: &SExpr) -> bool {
        let mut stack = vec![(self, other)];
        while let Some((a, b)) = stack.pop() {
            if !a.same_node(b) {
                return false;
            }
            stack.extend(a.children().into_iter().zip(b.children()));
        }
        true
    }
}

impl Drop for SExpr {
    fn drop(&mut self) {
        let mut stack: Vec<SExpr> = self.slots().into_iter().map(std::mem::take).collect();
        while let Some(mut e) = stack.pop() {
            stack.extend(e.slots().into_iter().map(std::mem::take));
        }
    }
}

/// `Debug` as derive printed it, without recursion
/// (`sophialang_core::debug`).
impl std::fmt::Debug for SExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use sophialang_core::debug::{DNode, opt, render};
        fn binder(b: &SBinder, alt: bool) -> DNode<'_, SExpr> {
            let l = |v: &dyn std::fmt::Debug| DNode::Leaf(if alt { format!("{v:#?}") } else { format!("{v:?}") });
            DNode::Struct(
                "SBinder",
                vec![("name", l(&b.name)), ("ty", opt(b.ty.as_deref())), ("usage", l(&b.usage)), ("implicit", l(&b.implicit)), ("span", l(&b.span))],
            )
        }
        fn one(e: &SExpr, alt: bool) -> DNode<'_, SExpr> {
            let l = |v: &dyn std::fmt::Debug| DNode::Leaf(if alt { format!("{v:#?}") } else { format!("{v:?}") });
            fn c(x: &SExpr) -> DNode<'_, SExpr> {
                DNode::Child(x)
            }
            let kind = match &e.kind {
                SKind::Path(x) => DNode::Tuple("Path", vec![l(x)]),
                SKind::HashRef(x) => DNode::Tuple("HashRef", vec![l(x)]),
                SKind::NominalRef(x) => DNode::Tuple("NominalRef", vec![l(x)]),
                SKind::Int(x) => DNode::Tuple("Int", vec![l(x)]),
                SKind::UInt(x) => DNode::Tuple("UInt", vec![l(x)]),
                SKind::Float(x) => DNode::Tuple("Float", vec![l(x)]),
                SKind::Str(x) => DNode::Tuple("Str", vec![l(x)]),
                SKind::Char(x) => DNode::Tuple("Char", vec![l(x)]),
                SKind::Bool(x) => DNode::Tuple("Bool", vec![l(x)]),
                SKind::Type(x) => DNode::Tuple("Type", vec![l(x)]),
                SKind::Prop => DNode::Tuple("Prop", vec![]),
                SKind::Prim(x) => DNode::Tuple("Prim", vec![l(x)]),
                SKind::Hole(x) => DNode::Tuple("Hole", vec![c(x)]),
                SKind::Unsafe(a, x) => DNode::Tuple("Unsafe", vec![l(a), c(x)]),
                SKind::ConIndex(x, j) => DNode::Tuple("ConIndex", vec![c(x), l(j)]),
                SKind::Member(k) => DNode::Tuple("Member", vec![l(k)]),
                SKind::Call(h, args) => DNode::Tuple("Call", vec![c(h), DNode::List(args.iter().map(c).collect())]),
                SKind::BinOp(op, a, b) => DNode::Tuple("BinOp", vec![l(op), c(a), c(b)]),
                SKind::Lam(b, x) => DNode::Tuple("Lam", vec![binder(b, alt), c(x)]),
                SKind::Pi(b, x) => DNode::Tuple("Pi", vec![binder(b, alt), c(x)]),
                SKind::Let(b, v, x) => DNode::Tuple("Let", vec![binder(b, alt), c(v), c(x)]),
                SKind::Match { scrutinee, data, cases } => DNode::Struct(
                    "Match",
                    vec![
                        ("scrutinee", c(scrutinee)),
                        ("data", opt(data.as_deref())),
                        (
                            "cases",
                            DNode::List(
                                cases
                                    .iter()
                                    .map(|k| {
                                        DNode::Struct(
                                            "SCase",
                                            vec![
                                                ("pattern", l(&k.pattern)),
                                                ("binders", DNode::List(k.binders.iter().map(|b| binder(b, alt)).collect())),
                                                ("body", c(&k.body)),
                                                ("span", l(&k.span)),
                                            ],
                                        )
                                    })
                                    .collect(),
                            ),
                        ),
                    ],
                ),
                SKind::Annot(a, b) => DNode::Tuple("Annot", vec![c(a), c(b)]),
                SKind::Implicit(x) => DNode::Tuple("Implicit", vec![c(x)]),
                SKind::Usage(u, x) => DNode::Tuple("Usage", vec![l(u), c(x)]),
            };
            DNode::Struct("SExpr", vec![("kind", kind), ("span", l(&e.span))])
        }
        render(f, self, one)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Name(String),
    Index(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SCase {
    pub pattern: Pattern,
    pub binders: Vec<SBinder>,
    pub body: SExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SBody {
    /// `function f(params)::R body end`
    Sugar { params: Vec<SBinder>, ret: SExpr, body: SExpr },
    /// `function f :: T body end`
    Typed { ty: SExpr, body: SExpr },
    Constant { ty: SExpr, value: SExpr },
    Data { ty: SExpr, constructors: Vec<(String, SExpr)> },
    Layout { fields: Vec<(String, SExpr)> },
    Relation { ty: SExpr },
    Rule { vars: Vec<SBinder>, weight: Option<SExpr>, head: SExpr, premises: Vec<SExpr> },
    Test { body: SExpr },
    Postulate { ty: SExpr },
}

/// One top-level definition in a text.
#[derive(Debug, Clone, PartialEq)]
pub struct SItem {
    pub name: String,
    pub name_span: Span,
    pub body: SBody,
    pub span: Span,
}
