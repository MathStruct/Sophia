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

#[derive(Debug, Clone, PartialEq)]
pub struct SExpr {
    pub kind: SKind,
    pub span: Span,
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
