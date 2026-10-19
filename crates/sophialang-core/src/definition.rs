//! sophialang definitions and their kinds (SPEC-003 §1). See `definition.md`.

use crate::term::{Binder, Lit, Term};

/// The kinds of content-addressed sophialang definitions. Nominal entities
/// (generic functions, structs, abstract and primitive types) are not
/// definitions: the store creates them with fresh ids ([`NominalKind`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// One method of a generic function: a signature (the type) and a body.
    Method,
    Constant,
    /// A structural inductive type: equal declarations are one type.
    Data,
    /// The field layout of a nominal struct.
    Layout,
    /// The declared type of a relation. The relation itself is nominal
    /// (S-ADR-0009); its signature and its rules join it through the
    /// store's versioned relation, like methods join a generic function.
    Relation,
    Rule,
    Test,
    /// A declared type with no body.
    Postulate,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::Method,
        Kind::Constant,
        Kind::Data,
        Kind::Layout,
        Kind::Relation,
        Kind::Rule,
        Kind::Test,
        Kind::Postulate,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Method => "method",
            Kind::Constant => "constant",
            Kind::Data => "data",
            Kind::Layout => "layout",
            Kind::Relation => "relation",
            Kind::Rule => "rule",
            Kind::Test => "test",
            Kind::Postulate => "postulate",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Kind::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

/// Nominal entity kinds sophialang uses (SPEC-001 §4.3, SPEC-003 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NominalKind {
    Generic,
    /// A logic relation: its rules attach like methods (S-ADR-0009).
    Relation,
    Struct,
    Abstract,
    Primitive,
}

impl NominalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NominalKind::Generic => "generic",
            NominalKind::Relation => "relation",
            NominalKind::Struct => "struct",
            NominalKind::Abstract => "abstract",
            NominalKind::Primitive => "primitive",
        }
    }
}

/// A constructor of a `data` type. Its name is display data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Constructor {
    pub name: String,
    pub ty: Term,
}

/// A struct field. Field names are observable (`x.r`), so they are hashed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    pub name: String,
    pub ty: Term,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Body {
    None,
    Term(Term),
    Constructors(Vec<Constructor>),
    Fields(Vec<Field>),
    Rule { vars: Vec<Binder>, head: Term, premises: Vec<Term>, weight: Option<Lit> },
}

/// A sophialang definition: a kind, an optional declared type, a body.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Definition {
    pub kind: Kind,
    pub ty: Option<Term>,
    pub body: Body,
}

impl Definition {
    pub fn method(signature: Term, body: Term) -> Self {
        Definition { kind: Kind::Method, ty: Some(signature), body: Body::Term(body) }
    }

    pub fn constant(ty: Term, value: Term) -> Self {
        Definition { kind: Kind::Constant, ty: Some(ty), body: Body::Term(value) }
    }

    pub fn postulate(ty: Term) -> Self {
        Definition { kind: Kind::Postulate, ty: Some(ty), body: Body::None }
    }

    pub fn data(ty: Term, constructors: Vec<Constructor>) -> Self {
        Definition { kind: Kind::Data, ty: Some(ty), body: Body::Constructors(constructors) }
    }

    pub fn test(body: Term) -> Self {
        Definition { kind: Kind::Test, ty: None, body: Body::Term(body) }
    }

    /// Every top-level term of the definition, in canonical stream order.
    pub fn terms(&self) -> Vec<&Term> {
        let mut out: Vec<&Term> = self.ty.iter().collect();
        match &self.body {
            Body::None => {}
            Body::Term(t) => out.push(t),
            Body::Constructors(cs) => out.extend(cs.iter().map(|c| &c.ty)),
            Body::Fields(fs) => out.extend(fs.iter().map(|f| &f.ty)),
            Body::Rule { head, premises, .. } => {
                out.push(head);
                out.extend(premises.iter());
            }
        }
        out
    }

    /// Visits every subterm in canonical stream order.
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a Term)) {
        for t in self.terms() {
            t.walk(f);
        }
    }

    /// Rewrites every term of the definition with `f` (bottom-up).
    pub fn map_terms(self, f: &mut impl FnMut(Term) -> Term) -> Self {
        let ty = self.ty.map(|t| t.map(f));
        let body = match self.body {
            Body::None => Body::None,
            Body::Term(t) => Body::Term(t.map(f)),
            Body::Constructors(cs) => {
                Body::Constructors(cs.into_iter().map(|c| Constructor { ty: c.ty.map(f), ..c }).collect())
            }
            Body::Fields(fs) => Body::Fields(fs.into_iter().map(|x| Field { ty: x.ty.map(f), ..x }).collect()),
            Body::Rule { vars, head, premises, weight } => Body::Rule {
                vars,
                head: head.map(f),
                premises: premises.into_iter().map(|p| p.map(f)).collect(),
                weight,
            },
        };
        Definition { kind: self.kind, ty, body }
    }

    /// Whether the definition contains holes or unresolved names.
    pub fn is_incomplete(&self) -> bool {
        let mut inc = false;
        self.walk(&mut |t| inc |= matches!(t, Term::Hole(_) | Term::Unresolved(_)));
        inc
    }
}
