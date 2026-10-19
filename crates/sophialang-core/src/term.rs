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

/// A core term.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// Visits every subterm in pre-order (the canonical stream order).
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a Term)) {
        f(self);
        match self {
            Term::Var(_)
            | Term::Ref(_)
            | Term::Member(_)
            | Term::Draft(_)
            | Term::Nominal(_)
            | Term::Type(_)
            | Term::Prop
            | Term::Lit(_)
            | Term::Prim(_)
            | Term::Unresolved(_) => {}
            Term::Con { ty, .. } => ty.walk(f),
            Term::Lam { domain, body, .. } => {
                if let Some(d) = domain {
                    d.walk(f);
                }
                body.walk(f);
            }
            Term::App(h, args) => {
                h.walk(f);
                for a in args {
                    a.walk(f);
                }
            }
            Term::Pi { domain, codomain, .. } => {
                domain.walk(f);
                codomain.walk(f);
            }
            Term::Let { ty, value, body, .. } => {
                if let Some(t) = ty {
                    t.walk(f);
                }
                value.walk(f);
                body.walk(f);
            }
            Term::Match { scrutinee, data, motive, cases } => {
                scrutinee.walk(f);
                if let Some(d) = data {
                    d.walk(f);
                }
                if let Some(m) = motive {
                    m.walk(f);
                }
                for c in cases {
                    c.body.walk(f);
                }
            }
            Term::Hole(t) | Term::Unsafe(_, t) => t.walk(f),
        }
    }

    /// Rewrites every subterm bottom-up with `f` (used to resolve drafts).
    pub fn map(self, f: &mut impl FnMut(Term) -> Term) -> Term {
        let t = match self {
            Term::Con { ty, index } => Term::Con { ty: Box::new(ty.map(f)), index },
            Term::Lam { binder, domain, body } => Term::Lam {
                binder,
                domain: domain.map(|d| Box::new(d.map(f))),
                body: Box::new(body.map(f)),
            },
            Term::App(h, args) => Term::App(Box::new(h.map(f)), args.into_iter().map(|a| a.map(f)).collect()),
            Term::Pi { binder, domain, codomain } => {
                Term::Pi { binder, domain: Box::new(domain.map(f)), codomain: Box::new(codomain.map(f)) }
            }
            Term::Let { binder, ty, value, body } => Term::Let {
                binder,
                ty: ty.map(|t| Box::new(t.map(f))),
                value: Box::new(value.map(f)),
                body: Box::new(body.map(f)),
            },
            Term::Match { scrutinee, data, motive, cases } => Term::Match {
                scrutinee: Box::new(scrutinee.map(f)),
                data: data.map(|d| Box::new(d.map(f))),
                motive: motive.map(|m| Box::new(m.map(f))),
                cases: cases.into_iter().map(|c| Case { body: c.body.map(f), ..c }).collect(),
            },
            Term::Hole(t) => Term::Hole(Box::new(t.map(f))),
            Term::Unsafe(a, t) => Term::Unsafe(a, Box::new(t.map(f))),
            leaf => leaf,
        };
        f(t)
    }
}
