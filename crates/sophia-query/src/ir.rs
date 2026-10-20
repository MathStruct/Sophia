//! The query IR (SPEC-007 §3): stratified Datalog over the store's base
//! relations, with constants, parameters, negation, comparisons and a final
//! grouped `count`. Names are display data, kept beside the IR in
//! [`Names`], exactly as for sophialang definitions. See `ir.md`.

use std::fmt;

use sophia_hash::Digest;

/// The sort of a column or variable. A query's result type is the list of
/// its output sorts (SPEC-007 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sort {
    /// A 32-byte id: a definition, a nominal entity, an annotation, a fact.
    Id,
    Text,
    Int,
}

impl Sort {
    pub fn as_str(self) -> &'static str {
        match self {
            Sort::Id => "Id",
            Sort::Text => "Text",
            Sort::Int => "Int",
        }
    }

    pub fn parse(s: &str) -> Option<Sort> {
        match s {
            "Id" => Some(Sort::Id),
            "Text" => Some(Sort::Text),
            "Int" => Some(Sort::Int),
            _ => None,
        }
    }

    pub(crate) fn code(self) -> u32 {
        self as u32
    }

    pub(crate) fn from_code(c: u32) -> Option<Sort> {
        [Sort::Id, Sort::Text, Sort::Int].get(c as usize).copied()
    }
}

/// A value: a constant in a query, a parameter, a cell of a result.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Val {
    Id(Digest),
    Text(String),
    Int(i64),
}

impl Val {
    pub fn sort(&self) -> Sort {
        match self {
            Val::Id(_) => Sort::Id,
            Val::Text(_) => Sort::Text,
            Val::Int(_) => Sort::Int,
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Id(d) => write!(f, "{}", d.to_hex()),
            Val::Text(s) => write!(f, "{s}"),
            Val::Int(i) => write!(f, "{i}"),
        }
    }
}

/// A term: a variable of the enclosing rule, a parameter of the query, or a
/// constant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    Var(u32),
    Param(u32),
    Const(Val),
}

/// The store's base relations (SPEC-007 §2): the extensional database every
/// backend answers through its access paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Base {
    /// `def(d, kind)`: a definition and its kind.
    Def,
    /// `refs(a, b)`: definition `a` references definition `b` directly.
    Refs,
    /// `name(path, target)`: a name and what it points at.
    Name,
    /// `member(n, d)`: `d` is a current member of nominal entity `n`'s relation.
    Member,
    /// `nominal(n, kind)`: a nominal entity and its kind.
    Nominal,
    /// `uses(d, n)`: definition `d` references nominal entity `n`.
    Uses,
    /// `annotation(a, kind, subject)`.
    Annotation,
    /// `fact(f, kind, subject)`.
    Fact,
}

impl Base {
    pub const ALL: [Base; 8] =
        [Base::Def, Base::Refs, Base::Name, Base::Member, Base::Nominal, Base::Uses, Base::Annotation, Base::Fact];

    /// The schema version: part of every query's identity, so a change to
    /// the base relations gives new query ids.
    pub const SCHEMA: &'static str = "store-base-v0";

    pub fn name(self) -> &'static str {
        match self {
            Base::Def => "def",
            Base::Refs => "refs",
            Base::Name => "name",
            Base::Member => "member",
            Base::Nominal => "nominal",
            Base::Uses => "uses",
            Base::Annotation => "annotation",
            Base::Fact => "fact",
        }
    }

    pub fn parse(s: &str) -> Option<Base> {
        Base::ALL.into_iter().find(|b| b.name() == s)
    }

    pub fn sorts(self) -> &'static [Sort] {
        use Sort::*;
        match self {
            Base::Def => &[Id, Text],
            Base::Refs => &[Id, Id],
            Base::Name => &[Text, Id],
            Base::Member => &[Id, Id],
            Base::Nominal => &[Id, Text],
            Base::Uses => &[Id, Id],
            Base::Annotation => &[Id, Text, Id],
            Base::Fact => &[Id, Text, Id],
        }
    }

    pub fn arity(self) -> usize {
        self.sorts().len()
    }
}

/// A predicate: a base relation, or one the query defines by rules
/// (numbered within the query).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pred {
    Base(Base),
    Local(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Atom {
    pub pred: Pred,
    pub args: Vec<Term>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    Pos(Atom),
    /// Negation as failure, stratified (SPEC-007 §3.3).
    Neg(Atom),
    Eq(Term, Term),
    Ne(Term, Term),
    /// `startswith(text, prefix)`, Julia's spelling.
    StartsWith(Term, Term),
}

impl Literal {
    /// The variables the literal mentions, in order of appearance.
    pub fn vars(&self) -> Vec<u32> {
        let terms: Vec<&Term> = match self {
            Literal::Pos(a) | Literal::Neg(a) => a.args.iter().collect(),
            Literal::Eq(x, y) | Literal::Ne(x, y) | Literal::StartsWith(x, y) => vec![x, y],
        };
        terms
            .into_iter()
            .filter_map(|t| match t {
                Term::Var(v) => Some(*v),
                _ => None,
            })
            .collect()
    }
}

/// `head(args) = body`. Several rules for one predicate are a union.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule {
    pub head: u32,
    pub args: Vec<Term>,
    pub body: Vec<Literal>,
    /// Variables are `0..vars`, numbered by first occurrence (head first).
    pub vars: u32,
}

/// One output column of `select`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Output {
    Var(u32),
    /// `count(v)`: the number of distinct values of `v` per group, where the
    /// group is the tuple of the plain `Var` outputs.
    Count(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Select {
    pub outputs: Vec<Output>,
    pub body: Vec<Literal>,
    pub vars: u32,
}

/// A query: parameters, local predicates defined by rules, and the select.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Query {
    pub params: Vec<Sort>,
    /// The arity of each local predicate.
    pub preds: Vec<u32>,
    pub rules: Vec<Rule>,
    pub select: Select,
}

impl Query {
    pub fn rules_of(&self, p: u32) -> impl Iterator<Item = &Rule> {
        self.rules.iter().filter(move |r| r.head == p)
    }

    /// The local predicates each local predicate's rules mention, with
    /// whether the mention is negated.
    pub fn pred_edges(&self) -> Vec<Vec<(u32, bool)>> {
        let mut out = vec![Vec::new(); self.preds.len()];
        for r in &self.rules {
            for l in &r.body {
                match l {
                    Literal::Pos(Atom { pred: Pred::Local(q), .. }) => out[r.head as usize].push((*q, false)),
                    Literal::Neg(Atom { pred: Pred::Local(q), .. }) => out[r.head as usize].push((*q, true)),
                    _ => {}
                }
            }
        }
        out
    }
}

/// The names a reader sees: display data, not part of a query's identity.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Names {
    pub query: String,
    pub params: Vec<String>,
    pub preds: Vec<String>,
    /// Variable names of each rule, in the IR's rule order.
    pub rule_vars: Vec<Vec<String>>,
    pub select_vars: Vec<String>,
}

impl Names {
    /// The column names of the result.
    pub fn columns(&self, q: &Query) -> Vec<String> {
        q.select
            .outputs
            .iter()
            .map(|o| match o {
                Output::Var(v) => self.select_vars.get(*v as usize).cloned().unwrap_or_else(|| format!("x{v}")),
                Output::Count(v) => {
                    format!("count_{}", self.select_vars.get(*v as usize).cloned().unwrap_or_else(|| format!("x{v}")))
                }
            })
            .collect()
    }
}
