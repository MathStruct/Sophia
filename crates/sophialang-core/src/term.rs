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

    /// Visits every subterm in pre-order (the canonical stream order), at
    /// any depth (one `deep::grow` per level, SP-015).
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a Term)) {
        sophia_hash::deep::grow(|| self.walk_step(f))
    }

    fn walk_step<'a>(&'a self, f: &mut impl FnMut(&'a Term)) {
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

    /// Rewrites every subterm bottom-up with `f` (used to resolve drafts),
    /// at any depth (one `deep::grow` per level, SP-015).
    pub fn map(mut self, f: &mut impl FnMut(Term) -> Term) -> Term {
        sophia_hash::deep::grow(|| self.map_children(f));
        f(self)
    }

    /// Replaces each direct subterm `c` by `c.map(f)`, in place. (`Term`
    /// has a `Drop` impl, so subterms are taken out with `mem::take`, not
    /// moved out by a pattern.)
    fn map_children(&mut self, f: &mut impl FnMut(Term) -> Term) {
        let mut on = |b: &mut Term| *b = std::mem::take(b).map(f);
        match self {
            Term::Con { ty, .. } => on(ty),
            Term::Lam { domain, body, .. } => {
                if let Some(d) = domain {
                    on(d);
                }
                on(body);
            }
            Term::App(h, args) => {
                on(h);
                args.iter_mut().for_each(&mut on);
            }
            Term::Pi { domain, codomain, .. } => {
                on(domain);
                on(codomain);
            }
            Term::Let { ty, value, body, .. } => {
                if let Some(t) = ty {
                    on(t);
                }
                on(value);
                on(body);
            }
            Term::Match { scrutinee, data, motive, cases } => {
                on(scrutinee);
                if let Some(d) = data {
                    on(d);
                }
                if let Some(m) = motive {
                    on(m);
                }
                cases.iter_mut().for_each(|c| on(&mut c.body));
            }
            Term::Hole(t) | Term::Unsafe(_, t) => on(t),
            _ => {}
        }
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

// Clone, PartialEq, Hash and Debug by hand, one `deep::grow` per level, so
// that copying, comparing, hashing or printing a term of any depth cannot
// overflow the stack (SP-015). They do what the derived impls did.

impl Clone for Term {
    // Generated uniformly over all fields, `Copy` ones included.
    #[allow(clippy::clone_on_copy)]
    fn clone(&self) -> Term {
        sophia_hash::deep::grow(|| match self {
            Term::Var(a_0) => Term::Var(a_0.clone()),
            Term::Ref(a_0) => Term::Ref(a_0.clone()),
            Term::Member(a_0) => Term::Member(a_0.clone()),
            Term::Draft(a_0) => Term::Draft(a_0.clone()),
            Term::Nominal(a_0) => Term::Nominal(a_0.clone()),
            Term::Con { ty: a_ty, index: a_index } => Term::Con { ty: a_ty.clone(), index: a_index.clone() },
            Term::Lam { binder: a_binder, domain: a_domain, body: a_body } => Term::Lam { binder: a_binder.clone(), domain: a_domain.clone(), body: a_body.clone() },
            Term::App(a_0, a_1) => Term::App(a_0.clone(), a_1.clone()),
            Term::Pi { binder: a_binder, domain: a_domain, codomain: a_codomain } => Term::Pi { binder: a_binder.clone(), domain: a_domain.clone(), codomain: a_codomain.clone() },
            Term::Let { binder: a_binder, ty: a_ty, value: a_value, body: a_body } => Term::Let { binder: a_binder.clone(), ty: a_ty.clone(), value: a_value.clone(), body: a_body.clone() },
            Term::Match { scrutinee: a_scrutinee, data: a_data, motive: a_motive, cases: a_cases } => Term::Match { scrutinee: a_scrutinee.clone(), data: a_data.clone(), motive: a_motive.clone(), cases: a_cases.clone() },
            Term::Type(a_0) => Term::Type(a_0.clone()),
            Term::Prop => Term::Prop,
            Term::Lit(a_0) => Term::Lit(a_0.clone()),
            Term::Prim(a_0) => Term::Prim(a_0.clone()),
            Term::Hole(a_0) => Term::Hole(a_0.clone()),
            Term::Unresolved(a_0) => Term::Unresolved(a_0.clone()),
            Term::Unsafe(a_0, a_1) => Term::Unsafe(a_0.clone(), a_1.clone()),
        })
    }
}

impl PartialEq for Term {
    fn eq(&self, other: &Term) -> bool {
        sophia_hash::deep::grow(|| match (self, other) {
            (Term::Var(a_0), Term::Var(b_0)) => a_0 == b_0,
            (Term::Ref(a_0), Term::Ref(b_0)) => a_0 == b_0,
            (Term::Member(a_0), Term::Member(b_0)) => a_0 == b_0,
            (Term::Draft(a_0), Term::Draft(b_0)) => a_0 == b_0,
            (Term::Nominal(a_0), Term::Nominal(b_0)) => a_0 == b_0,
            (Term::Con { ty: a_ty, index: a_index }, Term::Con { ty: b_ty, index: b_index }) => a_ty == b_ty && a_index == b_index,
            (Term::Lam { binder: a_binder, domain: a_domain, body: a_body }, Term::Lam { binder: b_binder, domain: b_domain, body: b_body }) => a_binder == b_binder && a_domain == b_domain && a_body == b_body,
            (Term::App(a_0, a_1), Term::App(b_0, b_1)) => a_0 == b_0 && a_1 == b_1,
            (Term::Pi { binder: a_binder, domain: a_domain, codomain: a_codomain }, Term::Pi { binder: b_binder, domain: b_domain, codomain: b_codomain }) => a_binder == b_binder && a_domain == b_domain && a_codomain == b_codomain,
            (Term::Let { binder: a_binder, ty: a_ty, value: a_value, body: a_body }, Term::Let { binder: b_binder, ty: b_ty, value: b_value, body: b_body }) => a_binder == b_binder && a_ty == b_ty && a_value == b_value && a_body == b_body,
            (Term::Match { scrutinee: a_scrutinee, data: a_data, motive: a_motive, cases: a_cases }, Term::Match { scrutinee: b_scrutinee, data: b_data, motive: b_motive, cases: b_cases }) => a_scrutinee == b_scrutinee && a_data == b_data && a_motive == b_motive && a_cases == b_cases,
            (Term::Type(a_0), Term::Type(b_0)) => a_0 == b_0,
            (Term::Prop, Term::Prop) => true,
            (Term::Lit(a_0), Term::Lit(b_0)) => a_0 == b_0,
            (Term::Prim(a_0), Term::Prim(b_0)) => a_0 == b_0,
            (Term::Hole(a_0), Term::Hole(b_0)) => a_0 == b_0,
            (Term::Unresolved(a_0), Term::Unresolved(b_0)) => a_0 == b_0,
            (Term::Unsafe(a_0, a_1), Term::Unsafe(b_0, b_1)) => a_0 == b_0 && a_1 == b_1,
            _ => false,
        })
    }
}

impl Eq for Term {}

impl std::hash::Hash for Term {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        sophia_hash::deep::grow(|| {
            std::mem::discriminant(self).hash(state);
            match self {
                Term::Var(a_0) => { a_0.hash(state); }
                Term::Ref(a_0) => { a_0.hash(state); }
                Term::Member(a_0) => { a_0.hash(state); }
                Term::Draft(a_0) => { a_0.hash(state); }
                Term::Nominal(a_0) => { a_0.hash(state); }
                Term::Con { ty: a_ty, index: a_index } => { a_ty.hash(state); a_index.hash(state); }
                Term::Lam { binder: a_binder, domain: a_domain, body: a_body } => { a_binder.hash(state); a_domain.hash(state); a_body.hash(state); }
                Term::App(a_0, a_1) => { a_0.hash(state); a_1.hash(state); }
                Term::Pi { binder: a_binder, domain: a_domain, codomain: a_codomain } => { a_binder.hash(state); a_domain.hash(state); a_codomain.hash(state); }
                Term::Let { binder: a_binder, ty: a_ty, value: a_value, body: a_body } => { a_binder.hash(state); a_ty.hash(state); a_value.hash(state); a_body.hash(state); }
                Term::Match { scrutinee: a_scrutinee, data: a_data, motive: a_motive, cases: a_cases } => { a_scrutinee.hash(state); a_data.hash(state); a_motive.hash(state); a_cases.hash(state); }
                Term::Type(a_0) => { a_0.hash(state); }
                Term::Prop => {  }
                Term::Lit(a_0) => { a_0.hash(state); }
                Term::Prim(a_0) => { a_0.hash(state); }
                Term::Hole(a_0) => { a_0.hash(state); }
                Term::Unresolved(a_0) => { a_0.hash(state); }
                Term::Unsafe(a_0, a_1) => { a_0.hash(state); a_1.hash(state); }
            }
        })
    }
}

impl std::fmt::Debug for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        sophia_hash::deep::grow(|| match self {
            Term::Var(a_0) => f.debug_tuple("Var").field(a_0).finish(),
            Term::Ref(a_0) => f.debug_tuple("Ref").field(a_0).finish(),
            Term::Member(a_0) => f.debug_tuple("Member").field(a_0).finish(),
            Term::Draft(a_0) => f.debug_tuple("Draft").field(a_0).finish(),
            Term::Nominal(a_0) => f.debug_tuple("Nominal").field(a_0).finish(),
            Term::Con { ty: a_ty, index: a_index } => f.debug_struct("Con").field("ty", a_ty).field("index", a_index).finish(),
            Term::Lam { binder: a_binder, domain: a_domain, body: a_body } => f.debug_struct("Lam").field("binder", a_binder).field("domain", a_domain).field("body", a_body).finish(),
            Term::App(a_0, a_1) => f.debug_tuple("App").field(a_0).field(a_1).finish(),
            Term::Pi { binder: a_binder, domain: a_domain, codomain: a_codomain } => f.debug_struct("Pi").field("binder", a_binder).field("domain", a_domain).field("codomain", a_codomain).finish(),
            Term::Let { binder: a_binder, ty: a_ty, value: a_value, body: a_body } => f.debug_struct("Let").field("binder", a_binder).field("ty", a_ty).field("value", a_value).field("body", a_body).finish(),
            Term::Match { scrutinee: a_scrutinee, data: a_data, motive: a_motive, cases: a_cases } => f.debug_struct("Match").field("scrutinee", a_scrutinee).field("data", a_data).field("motive", a_motive).field("cases", a_cases).finish(),
            Term::Type(a_0) => f.debug_tuple("Type").field(a_0).finish(),
            Term::Prop => f.write_str("Prop"),
            Term::Lit(a_0) => f.debug_tuple("Lit").field(a_0).finish(),
            Term::Prim(a_0) => f.debug_tuple("Prim").field(a_0).finish(),
            Term::Hole(a_0) => f.debug_tuple("Hole").field(a_0).finish(),
            Term::Unresolved(a_0) => f.debug_tuple("Unresolved").field(a_0).finish(),
            Term::Unsafe(a_0, a_1) => f.debug_tuple("Unsafe").field(a_0).field(a_1).finish(),
        })
    }
}
