//! Dispatch facts recorded from Julia and the facts a new method makes stale
//! (E-3, B-024). See `dispatch_facts.md` and [[Dispatch Invalidation]].
//!
//! `julia-semantics/dispatch_facts.jl` prints, for one change (one or more
//! methods added after a workload was compiled), every **dispatch fact** (a
//! call in a compiled method instance: its argument types, the methods it can
//! reach before and after), the call edges between instances, the type
//! hierarchy those types need, and the instances Julia's own invalidation log
//! names. This module computes Sophia's stale set from the facts and the new
//! methods' signatures alone, by signature overlap, and scores it against what
//! actually changed and against what Julia invalidated.
//!
//! The type check is deliberately small. `overlap` may say "yes" when Julia's
//! `typeintersect` says empty (a stale set too large, never too small);
//! `subtype` says "yes" only when it is sure (so the specificity rule can only
//! drop a fact that truly cannot change). Type parameters are compared by
//! name, except that two concrete types overlap only when they are equal.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// A Julia type as `dispatch_facts.jl` encodes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Any,
    Bottom,
    /// Something the encoder could not express: overlaps everything, a subtype of nothing.
    Unknown,
    /// A named type; `exact` is the full type, `None` when it has free type variables.
    Named {
        name: String,
        exact: Option<String>,
    },
    Union(Vec<Ty>),
    /// `Type{X}`.
    Type(Box<Ty>),
    Vararg(Box<Ty>),
    /// A type variable with this upper bound.
    Var(Box<Ty>),
    Tuple(Vec<Ty>),
}

#[derive(Debug, thiserror::Error)]
pub enum InvalidationError {
    #[error("bad type encoding at token {at}: {text}")]
    Type { at: usize, text: String },
    #[error("line {line}: {msg}")]
    Row { line: usize, msg: String },
}

impl Ty {
    pub fn parse(text: &str) -> Result<Ty, InvalidationError> {
        let toks: Vec<&str> = text.split(' ').collect();
        let mut at = 0;
        let ty = parse_tokens(&toks, &mut at).ok_or_else(|| InvalidationError::Type { at, text: text.to_owned() })?;
        if at != toks.len() {
            return Err(InvalidationError::Type { at, text: text.to_owned() });
        }
        Ok(ty)
    }

    /// True when the type contains no variable, no unknown and no partly known name,
    /// so that structural equality is type equality.
    fn ground(&self) -> bool {
        match self {
            Ty::Any | Ty::Bottom => true,
            Ty::Unknown | Ty::Var(_) => false,
            Ty::Named { exact, .. } => exact.is_some(),
            Ty::Union(xs) | Ty::Tuple(xs) => xs.iter().all(Ty::ground),
            Ty::Type(x) | Ty::Vararg(x) => x.ground(),
        }
    }
}

/// A named type whose spelling is the only one its type has.
fn canonical(t: &Ty) -> bool {
    match t {
        Ty::Named { exact: Some(e), .. } => !(e.contains("Tuple{") || e.contains("Union{") || e.contains("where")),
        _ => false,
    }
}

fn parse_tokens(t: &[&str], at: &mut usize) -> Option<Ty> {
    let tok = *t.get(*at)?;
    *at += 1;
    let many = |at: &mut usize| -> Option<Vec<Ty>> {
        let k: usize = t.get(*at)?.parse().ok()?;
        *at += 1;
        (0..k).map(|_| parse_tokens(t, at)).collect()
    };
    Some(match tok {
        "any" => Ty::Any,
        "bot" => Ty::Bottom,
        "unk" => Ty::Unknown,
        "n" => {
            let name = (*t.get(*at)?).to_owned();
            let exact = *t.get(*at + 1)?;
            *at += 2;
            Ty::Named { name, exact: (exact != "~").then(|| exact.to_owned()) }
        }
        "u" => Ty::Union(many(at)?),
        "tup" => Ty::Tuple(many(at)?),
        "t" => Ty::Type(Box::new(parse_tokens(t, at)?)),
        "v" => Ty::Vararg(Box::new(parse_tokens(t, at)?)),
        "tv" => Ty::Var(Box::new(parse_tokens(t, at)?)),
        _ => return None,
    })
}

/// The nominal hierarchy of the named types in a run: abstract or not, and
/// the names of the supertypes up to `Any`.
#[derive(Debug, Default)]
pub struct Lattice {
    types: HashMap<String, (bool, Vec<String>)>,
}

/// The kinds, the types whose instances are types: what `Type{X}` can overlap.
const KINDS: [&str; 5] = ["Core.Type", "Core.DataType", "Core.UnionAll", "Core.Union", "Core.TypeofBottom"];

impl Lattice {
    pub fn insert(&mut self, name: &str, abstract_: bool, supers: Vec<String>) {
        self.types.insert(name.to_owned(), (abstract_, supers));
    }

    fn is_super(&self, sub: &str, sup: &str) -> Option<bool> {
        self.types.get(sub).map(|(_, s)| s.iter().any(|x| x == sup))
    }

    /// May some value have both types? Never false when Julia's answer is non-empty.
    pub fn overlap(&self, a: &Ty, b: &Ty) -> bool {
        use Ty::*;
        match (a, b) {
            (Any | Unknown, _) | (_, Any | Unknown) => true,
            (Bottom, _) | (_, Bottom) => false,
            (Var(ub), x) | (x, Var(ub)) => self.overlap(ub, x),
            (Union(xs), y) | (y, Union(xs)) => xs.iter().any(|x| self.overlap(x, y)),
            (Vararg(x), y) | (y, Vararg(x)) => self.overlap(x, y),
            (Tuple(xs), Tuple(ys)) => self.tuple_overlap(xs, ys),
            // A tuple type spelled as a closed `UnionAll` inside `Type{…}`.
            (Tuple(_), Named { name, .. }) | (Named { name, .. }, Tuple(_)) if name == "Core.Tuple" => true,
            (Tuple(_), _) | (_, Tuple(_)) => false,
            // `Type{X}` holds one value, `X`: two fully known named types overlap
            // only when equal. Only spellings without a tuple, a union or a `where`
            // are canonical: tuples are covariant, so `Tuple{T} where T<:Real` is
            // `Tuple{Real}`, and `Tuple{Union{A,B}}` is `Union{Tuple{A},Tuple{B}}`.
            // `Type{Union{}}` is a singleton too; a variable or anything else may be
            // whatever it overlaps.
            (Type(x), Type(y)) if canonical(x) && canonical(y) => x == y,
            (Type(x), Type(y)) => matches!(**x, Bottom | Var(_)) || matches!(**y, Bottom | Var(_)) || self.overlap(x, y),
            (Type(_), Named { name, .. }) | (Named { name, .. }, Type(_)) => KINDS.contains(&name.as_str()),
            (Named { name: na, exact: ea }, Named { name: nb, exact: eb }) => {
                if na == nb {
                    // Two concrete types overlap only when equal.
                    match (ea, eb, self.types.get(na)) {
                        (Some(x), Some(y), Some((false, _))) => x == y,
                        _ => true,
                    }
                } else {
                    match (self.is_super(na, nb), self.is_super(nb, na)) {
                        (Some(x), Some(y)) => x || y,
                        _ => true,
                    }
                }
            }
        }
    }

    fn tuple_overlap(&self, xs: &[Ty], ys: &[Ty]) -> bool {
        let tail = |v: &[Ty]| match v.last() {
            Some(Ty::Vararg(t)) => Some((**t).clone()),
            _ => None,
        };
        let (tx, ty) = (tail(xs), tail(ys));
        let fx = xs.len() - usize::from(tx.is_some());
        let fy = ys.len() - usize::from(ty.is_some());
        let n = fx.max(fy);
        match (&tx, &ty) {
            (None, None) if fx != fy => return false,
            (None, Some(_)) if fx < fy => return false,
            (Some(_), None) if fy < fx => return false,
            _ => {}
        }
        let at = |v: &[Ty], f: usize, t: &Option<Ty>, i: usize| if i < f { Some(v[i].clone()) } else { t.clone() };
        // Positions past both fixed parts may be empty, so the two tails need not overlap.
        (0..n).all(|i| match (at(xs, fx, &tx, i), at(ys, fy, &ty, i)) {
            (Some(a), Some(b)) => self.overlap(&a, &b),
            _ => true,
        })
    }

    /// Is every value of `a` a value of `b`? True only when sure.
    pub fn subtype(&self, a: &Ty, b: &Ty) -> bool {
        use Ty::*;
        match (a, b) {
            (_, Any) | (Bottom, _) => true,
            (Unknown, _) | (_, Unknown) => false,
            (Union(xs), _) => xs.iter().all(|x| self.subtype(x, b)),
            (_, Union(ys)) => ys.iter().any(|y| self.subtype(a, y)),
            (Var(ub), _) => self.subtype(ub, b),
            (_, Var(_)) => false,
            (Tuple(xs), Tuple(ys)) => {
                xs.len() == ys.len()
                    && xs.iter().zip(ys).all(|(x, y)| match (x, y) {
                        (Vararg(_), _) | (_, Vararg(_)) => x == y && x.ground(),
                        _ => self.subtype(x, y),
                    })
            }
            (Type(_), Type(y)) if matches!(**y, Var(ref ub) if **ub == Any) => true,
            (Type(x), Type(y)) => x == y && x.ground(),
            (Named { name: na, exact: ea }, Named { name: nb, exact: eb }) => {
                if ea.is_some() && ea == eb {
                    return true;
                }
                // Only a supertype without parameters is safe to compare by name.
                match eb {
                    Some(e) if !e.contains('{') => na == nb || self.is_super(na, nb) == Some(true),
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

/// What a fact records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactKind {
    /// A call in a compiled body: every method a call with these types can reach.
    Call,
    /// `invoke(f, T, …)`: the one method chosen for the signature `T`.
    Invoke,
    /// The instance's own argument types (what reached it), not a call in its body.
    Own,
}

/// One dispatch fact: a call in a compiled instance.
#[derive(Debug, Clone)]
pub struct Fact {
    pub caller: String,
    pub sig: Ty,
    /// The methods the call could reach before the change; `None` when Julia could not list them.
    pub before: Option<BTreeSet<String>>,
    pub after: Option<BTreeSet<String>>,
    pub kind: FactKind,
}

impl Fact {
    /// Did the change alter what the call can reach? Unknown counts as changed.
    pub fn changed(&self) -> bool {
        match (&self.before, &self.after) {
            (Some(a), Some(b)) => a != b,
            _ => true,
        }
    }
}

/// One run of `dispatch_facts.jl`: one change, everything it printed.
#[derive(Debug, Default)]
pub struct Run {
    pub facts: Vec<Fact>,
    /// (caller, callee, call signature): a call with one concrete target.
    pub edges: Vec<(String, String, Ty)>,
    /// Every recorded instance, and whether Julia's log names it.
    pub instances: BTreeMap<String, bool>,
    /// Instances Julia's log names (inside and outside the recorded ones).
    pub logged: BTreeSet<String>,
    pub new_methods: Vec<(String, Ty)>,
    pub methods: HashMap<String, Ty>,
    pub lattice: Lattice,
    pub summary: BTreeMap<String, String>,
}

fn method_set(s: &str) -> Option<BTreeSet<String>> {
    match s {
        "?" => None,
        "-" => Some(BTreeSet::new()),
        _ => Some(s.split(' ').map(str::to_owned).collect()),
    }
}

impl Run {
    pub fn parse(text: &str) -> Result<Run, InvalidationError> {
        let mut run = Run::default();
        for (i, line) in text.lines().enumerate() {
            let f: Vec<&str> = line.split('\t').collect();
            let bad = |msg: &str| InvalidationError::Row { line: i + 1, msg: msg.to_owned() };
            let want = |n: usize| if f.len() == n { Ok(()) } else { Err(bad(&format!("expected {n} fields"))) };
            match f[0] {
                "fact" => {
                    want(6)?;
                    let kind = match f[5] {
                        "call" => FactKind::Call,
                        "invoke" => FactKind::Invoke,
                        "self" => FactKind::Own,
                        _ => return Err(bad("unknown fact kind")),
                    };
                    run.facts.push(Fact {
                        caller: f[1].to_owned(),
                        sig: Ty::parse(f[2])?,
                        before: method_set(f[3]),
                        after: method_set(f[4]),
                        kind,
                    });
                }
                "edge" => {
                    want(4)?;
                    run.edges.push((f[1].to_owned(), f[2].to_owned(), Ty::parse(f[3])?));
                }
                "instance" => {
                    want(3)?;
                    run.instances.insert(f[1].to_owned(), f[2] == "1");
                }
                "logged" => {
                    want(2)?;
                    run.logged.insert(f[1].to_owned());
                }
                "new" => {
                    want(3)?;
                    run.new_methods.push((f[1].to_owned(), Ty::parse(f[2])?));
                }
                "method" => {
                    want(3)?;
                    run.methods.insert(f[1].to_owned(), Ty::parse(f[2])?);
                }
                "type" => {
                    want(4)?;
                    let supers = f[3].split(' ').filter(|s| !s.is_empty()).map(str::to_owned).collect();
                    run.lattice.insert(f[1], f[2] == "1", supers);
                }
                "summary" => {
                    want(3)?;
                    run.summary.insert(f[1].to_owned(), f[2].to_owned());
                }
                _ => return Err(bad("unknown row kind")),
            }
        }
        Ok(run)
    }
}

/// How Sophia decides that a fact is stale when a method is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// The call's signature overlaps the new method's (SP-006's proposal).
    Overlap,
    /// Overlap, unless a method the call already reaches covers the whole call
    /// and is strictly more specific than the new one (so the new one never wins).
    /// For `invoke(f, T, …)` also unless the new method is strictly smaller than
    /// `T` (`invoke` only chooses a method that covers all of `T`).
    Specific,
}

impl Run {
    /// Is `fact` stale under `rule`, from the recorded facts and new signatures alone?
    pub fn stale(&self, fact: &Fact, rule: Rule) -> bool {
        self.new_methods.iter().any(|(_, new)| {
            if !self.lattice.overlap(&fact.sig, new) {
                return false;
            }
            if rule == Rule::Overlap {
                return true;
            }
            let Some(before) = &fact.before else { return true };
            let l = &self.lattice;
            let strict = |a: &Ty, b: &Ty| l.subtype(a, b) && !(a == b && a.ground());
            if fact.kind == FactKind::Invoke && strict(new, &fact.sig) {
                return false;
            }
            !before.iter().any(|m| self.methods.get(m).is_some_and(|ms| l.subtype(&fact.sig, ms) && strict(ms, new)))
        })
    }

    /// The instances whose compiled code relies on a fact `bad` marks: the
    /// fact's caller, the instance a marked call reached, and every caller of
    /// one of these through the recorded edges.
    pub fn affected(&self, bad: impl Fn(&Fact) -> bool) -> BTreeSet<String> {
        let marked: HashSet<(&str, &Ty)> = self.facts.iter().filter(|f| bad(f)).map(|f| (f.caller.as_str(), &f.sig)).collect();
        let mut out: BTreeSet<String> = marked.iter().map(|(c, _)| (*c).to_owned()).collect();
        for (a, b, sig) in &self.edges {
            if marked.contains(&(a.as_str(), sig)) {
                out.insert(b.clone());
            }
        }
        let mut callers: HashMap<&str, Vec<&str>> = HashMap::new();
        for (a, b, _) in &self.edges {
            callers.entry(b.as_str()).or_default().push(a.as_str());
        }
        let mut work: Vec<String> = out.iter().cloned().collect();
        while let Some(x) = work.pop() {
            for c in callers.get(x.as_str()).into_iter().flatten() {
                if out.insert((*c).to_owned()) {
                    work.push((*c).to_owned());
                }
            }
        }
        out
    }
}

/// Precision and recall of a predicted set against a true one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Score {
    pub tp: usize,
    pub fp: usize,
    pub fn_: usize,
}

impl Score {
    pub fn of<T: Ord>(predicted: &BTreeSet<T>, truth: &BTreeSet<T>) -> Score {
        Score {
            tp: predicted.intersection(truth).count(),
            fp: predicted.difference(truth).count(),
            fn_: truth.difference(predicted).count(),
        }
    }
    pub fn precision(&self) -> f64 {
        if self.tp + self.fp == 0 { 1.0 } else { self.tp as f64 / (self.tp + self.fp) as f64 }
    }
    pub fn recall(&self) -> f64 {
        if self.tp + self.fn_ == 0 { 1.0 } else { self.tp as f64 / (self.tp + self.fn_) as f64 }
    }
}

/// The measurements of one run.
#[derive(Debug, Clone)]
pub struct Report {
    /// Calls and invokes in compiled bodies (`Own` facts count only at the instance level).
    pub facts: usize,
    /// Calls whose reachable methods changed.
    pub changed: usize,
    /// Fact level, against the facts that changed.
    pub overlap: Score,
    pub specific: Score,
    pub instances: usize,
    /// Recorded instances Julia invalidated.
    pub julia: usize,
    /// Instances Julia's log names that were not recorded.
    pub julia_unrecorded: usize,
    /// Instance level, against Julia's invalidations.
    pub overlap_vs_julia: Score,
    pub specific_vs_julia: Score,
    /// The instances relying on a fact that truly changed, against Julia's: how
    /// much Julia itself invalidates beyond what changed.
    pub changed_vs_julia: Score,
}

impl Run {
    pub fn report(&self) -> Report {
        let idx: Vec<usize> = (0..self.facts.len()).filter(|&i| self.facts[i].kind != FactKind::Own).collect();
        let set = |p: &dyn Fn(&Fact) -> bool| -> BTreeSet<usize> { idx.iter().copied().filter(|&i| p(&self.facts[i])).collect() };
        let changed = set(&|f| f.changed());
        let over = set(&|f| self.stale(f, Rule::Overlap));
        let spec = set(&|f| self.stale(f, Rule::Specific));
        let julia: BTreeSet<String> = self.instances.iter().filter(|(_, l)| **l).map(|(k, _)| k.clone()).collect();
        Report {
            facts: self.facts.len(),
            changed: changed.len(),
            overlap: Score::of(&over, &changed),
            specific: Score::of(&spec, &changed),
            instances: self.instances.len(),
            julia: julia.len(),
            julia_unrecorded: self.logged.iter().filter(|m| !self.instances.contains_key(*m)).count(),
            overlap_vs_julia: Score::of(&self.affected(|f| self.stale(f, Rule::Overlap)), &julia),
            specific_vs_julia: Score::of(&self.affected(|f| self.stale(f, Rule::Specific)), &julia),
            changed_vs_julia: Score::of(&self.affected(Fact::changed), &julia),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lattice() -> Lattice {
        let mut l = Lattice::default();
        l.insert("M.Shape", true, vec!["Any".into()]);
        l.insert("M.Circle", false, vec!["M.Shape".into(), "Any".into()]);
        l.insert("M.Sq", false, vec!["M.Shape".into(), "Any".into()]);
        l.insert("Core.Array", false, vec!["Core.DenseArray".into(), "Core.AbstractArray".into(), "Any".into()]);
        l
    }
    fn t(s: &str) -> Ty {
        Ty::parse(s).unwrap()
    }

    #[test]
    fn overlap_follows_the_hierarchy() {
        let l = lattice();
        assert!(l.overlap(&t("n M.Circle M.Circle"), &t("n M.Shape M.Shape")));
        assert!(!l.overlap(&t("n M.Circle M.Circle"), &t("n M.Sq M.Sq")));
        assert!(l.overlap(&t("any"), &t("n M.Sq M.Sq")));
        assert!(l.overlap(&t("u 2 n M.Circle M.Circle n M.Sq M.Sq"), &t("n M.Sq M.Sq")));
        // Concrete parametric types: equal or disjoint.
        assert!(!l.overlap(&t("n Core.Array Vector{Int64}"), &t("n Core.Array Vector{Float64}")));
        assert!(l.overlap(&t("n Core.Array Vector{Int64}"), &t("n Core.Array ~")));
        assert!(!l.overlap(&t("tup 2 any any"), &t("tup 1 any")));
        assert!(l.overlap(&t("tup 3 any any any"), &t("tup 2 any v any")));
        assert!(!l.overlap(&t("tup 1 any"), &t("tup 3 any any v any")));
    }

    #[test]
    fn subtype_is_sure_or_false() {
        let l = lattice();
        assert!(l.subtype(&t("n M.Circle M.Circle"), &t("n M.Shape M.Shape")));
        assert!(!l.subtype(&t("n M.Shape M.Shape"), &t("n M.Circle M.Circle")));
        assert!(l.subtype(&t("n M.Shape M.Shape"), &t("any")));
        // A parametric supertype is never assumed.
        assert!(!l.subtype(&t("n Core.Array Vector{Int64}"), &t("n Core.AbstractArray ~")));
        assert!(!l.subtype(&t("tv any"), &t("n M.Shape M.Shape")));
        assert!(l.subtype(&t("tup 2 n M.Sq M.Sq n M.Circle M.Circle"), &t("tup 2 n M.Shape M.Shape any")));
    }

    #[test]
    fn type_of_bottom_overlaps_itself() {
        // Codex's review: `Type{Union{}}` is a singleton, not empty.
        let l = lattice();
        // A variable with a lower bound is encoded `unk`: never a supertype.
        assert!(!l.subtype(&t("t n M.Sq M.Sq"), &t("t unk")));
        assert!(l.overlap(&t("t bot"), &t("t bot")));
        assert!(l.overlap(&t("t bot"), &t("t tv n M.Shape M.Shape")));
    }

    #[test]
    fn type_of_two_known_types_overlaps_only_when_equal() {
        // `Type{Circle}` and `Type{Shape}` are singletons with different values;
        // found on LinearAlgebra (a struct's constructor against `AbstractArray{Float64}(x)`).
        let mut l = lattice();
        assert!(!l.overlap(&t("t n M.Circle M.Circle"), &t("t n M.Shape M.Shape")));
        assert!(l.overlap(&t("t n M.Circle M.Circle"), &t("t n M.Circle M.Circle")));
        assert!(l.overlap(&t("t n M.Circle M.Circle"), &t("t tv n M.Shape M.Shape")));
        assert!(l.overlap(&t("t n M.Circle M.Circle"), &t("t n Core.AbstractArray ~")));
        // Codex's second review: equal types with two spellings (tuples are covariant).
        l.insert("Core.Real", true, vec!["Core.Number".into(), "Any".into()]);
        assert!(l.overlap(&t("t n Core.Tuple Tuple{T}whereT<:Real"), &t("t tup 1 n Core.Real Real")));
        assert!(l.overlap(&t("t tup 1 u 2 n M.Circle M.Circle n M.Sq M.Sq"), &t("t u 2 tup 1 n M.Circle M.Circle tup 1 n M.Sq M.Sq")));
    }

    #[test]
    fn parse_rejects_trailing_tokens() {
        assert!(Ty::parse("any any").is_err());
        assert!(Ty::parse("u 2 any").is_err());
    }
}
