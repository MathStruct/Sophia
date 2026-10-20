//! The native executor (SPEC-007 §6): semi-naive bottom-up evaluation of a
//! checked query over any backend's `ReadTx`. Base relations are read
//! through access paths, memoised per run; local relations are hash-indexed
//! on the columns the plan looks them up by. See `eval.md`.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use sophia_hash::Digest;
use sophia_store::{ReadTx, Target};

use crate::Error;
use crate::ir::*;
use crate::plan::{self, access_columns};

type Tuple = Vec<Val>;

/// Receives each solution of a body; an error stops the join.
type Emit<'e> = dyn FnMut(&[Option<Val>]) -> Result<(), Error> + 'e;

/// Limits for one run.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_rows: usize,
    /// Join steps, in thousands, before the run is aborted.
    pub max_steps_k: u64,
    /// Tuples derived for local predicates (and select rows before
    /// grouping) before the run is aborted: the memory bound. The step
    /// budget alone does not bound memory.
    pub max_derived: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_rows: 10_000, max_steps_k: 1_000_000, max_derived: 5_000_000 }
    }
}

/// What a run did, for benchmarks and `explain`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Access-path lookups that reached the backend (memoised ones excluded).
    pub lookups: u64,
    /// Full scans of a base relation.
    pub scans: u64,
    /// Tuples derived for local predicates.
    pub derived: u64,
    /// Semi-naive iterations, summed over strata.
    pub iterations: u64,
    pub steps: u64,
}

#[derive(Default)]
struct Rel {
    tuples: Vec<Tuple>,
    set: HashSet<Tuple>,
    index: HashMap<Vec<usize>, HashMap<Tuple, Vec<usize>>>,
}

impl Rel {
    fn insert(&mut self, t: Tuple) -> bool {
        if self.set.contains(&t) {
            return false;
        }
        let i = self.tuples.len();
        for (cols, idx) in self.index.iter_mut() {
            idx.entry(cols.iter().map(|&c| t[c].clone()).collect()).or_default().push(i);
        }
        self.set.insert(t.clone());
        self.tuples.push(t);
        true
    }

    fn ensure_index(&mut self, cols: &[usize]) {
        if cols.is_empty() || self.index.contains_key(cols) {
            return;
        }
        let mut idx: HashMap<Tuple, Vec<usize>> = HashMap::new();
        for (i, t) in self.tuples.iter().enumerate() {
            idx.entry(cols.iter().map(|&c| t[c].clone()).collect()).or_default().push(i);
        }
        self.index.insert(cols.to_vec(), idx);
    }
}

/// A body literal with the columns bound when it runs.
#[derive(Clone)]
struct Step {
    lit: usize,
    bound_cols: Vec<usize>,
}

fn steps(body: &[Literal], order: &[usize], vars: u32, initially: &[bool]) -> Vec<Step> {
    let mut bound = initially.to_vec();
    bound.resize(vars as usize, false);
    order
        .iter()
        .map(|&i| {
            let cols = match &body[i] {
                Literal::Pos(a) | Literal::Neg(a) => {
                    a.args.iter().enumerate().filter(|(_, t)| plan::is_bound(t, &bound)).map(|(c, _)| c).collect()
                }
                _ => Vec::new(),
            };
            plan::bind(&body[i], &mut bound);
            Step { lit: i, bound_cols: cols }
        })
        .collect()
}

pub struct Exec<'a> {
    tx: &'a dyn ReadTx,
    params: &'a [Val],
    limits: Limits,
    memo: HashMap<(Base, usize, Val), Rc<Vec<Tuple>>>,
    full: HashMap<Base, Rc<Vec<Tuple>>>,
    pub stats: Stats,
}

fn id(v: &Val) -> Option<&Digest> {
    match v {
        Val::Id(d) => Some(d),
        _ => None,
    }
}

fn text(v: &Val) -> Option<&str> {
    match v {
        Val::Text(s) => Some(s),
        _ => None,
    }
}

impl<'a> Exec<'a> {
    pub fn new(tx: &'a dyn ReadTx, params: &'a [Val], limits: Limits) -> Self {
        Exec { tx, params, limits, memo: HashMap::new(), full: HashMap::new(), stats: Stats::default() }
    }

    fn all_subjects(&self) -> Result<Vec<Digest>, Error> {
        let mut out = self.tx.records_of_kind(None, usize::MAX)?;
        out.extend(self.tx.nominal_ids()?);
        Ok(out)
    }

    /// Tuples of a base relation whose column `col` equals `key`.
    fn lookup(&mut self, b: Base, col: usize, key: &Val) -> Result<Rc<Vec<Tuple>>, Error> {
        if let Some(r) = self.memo.get(&(b, col, key.clone())) {
            return Ok(r.clone());
        }
        self.stats.lookups += 1;
        let tx = self.tx;
        let mut out: Vec<Tuple> = Vec::new();
        match (b, col) {
            (Base::Def, 0) => {
                if let Some(d) = id(key) {
                    out.extend(tx.kind_of(d)?.map(|k| vec![key.clone(), Val::Text(k)]));
                }
            }
            (Base::Def, 1) => {
                if let Some(k) = text(key) {
                    out.extend(tx.records_of_kind(Some(k), usize::MAX)?.into_iter().map(|d| vec![Val::Id(d), key.clone()]));
                }
            }
            (Base::Refs, 0) => {
                if let Some(d) = id(key) {
                    out.extend(tx.refs_of(d)?.into_iter().map(|r| vec![key.clone(), Val::Id(r)]));
                }
            }
            (Base::Refs, 1) => {
                if let Some(d) = id(key) {
                    out.extend(tx.dependants(d)?.into_iter().map(|r| vec![Val::Id(r), key.clone()]));
                }
            }
            (Base::Name, 0) => {
                if let Some(p) = text(key) {
                    out.extend(tx.name(p)?.map(|n| vec![key.clone(), Val::Id(n.target.digest())]));
                }
            }
            (Base::Name, 1) => {
                if let Some(d) = id(key) {
                    out.extend(tx.names_of(d)?.into_iter().map(|n| vec![Val::Text(n.path), key.clone()]));
                }
            }
            (Base::Member, 0) => {
                if let Some(d) = id(key) {
                    out.extend(tx.relation(d)?.into_iter().map(|e| vec![key.clone(), Val::Id(e.member)]));
                }
            }
            (Base::Nominal, 0) => {
                if let Some(d) = id(key) {
                    out.extend(tx.nominal(d)?.map(|n| vec![key.clone(), Val::Text(n.kind)]));
                }
            }
            (Base::Uses, 0) => {
                if let Some(d) = id(key) {
                    if let Some(r) = tx.record(d)? {
                        out.extend(r.nominal_refs.into_iter().map(|n| vec![key.clone(), Val::Id(n)]));
                    }
                }
            }
            (Base::Annotation, 2) => {
                if let Some(d) = id(key) {
                    out.extend(tx.annotations_of(d)?.into_iter().map(|a| vec![Val::Id(a.id), Val::Text(a.kind), key.clone()]));
                }
            }
            (Base::Fact, 2) => {
                if let Some(d) = id(key) {
                    out.extend(tx.facts_of(d)?.into_iter().map(|f| vec![Val::Id(f.id), Val::Text(f.kind), key.clone()]));
                }
            }
            _ => unreachable!("no access path for column {col} of {}", b.name()),
        }
        let out = Rc::new(out);
        self.memo.insert((b, col, key.clone()), out.clone());
        Ok(out)
    }

    /// Every tuple of a base relation.
    fn scan(&mut self, b: Base) -> Result<Rc<Vec<Tuple>>, Error> {
        if let Some(r) = self.full.get(&b) {
            return Ok(r.clone());
        }
        self.stats.scans += 1;
        let tx = self.tx;
        let out: Vec<Tuple> = match b {
            Base::Def => tx.all_kinds()?.into_iter().map(|(d, k)| vec![Val::Id(d), Val::Text(k)]).collect(),
            Base::Refs => tx.all_refs()?.into_iter().map(|(a, b)| vec![Val::Id(a), Val::Id(b)]).collect(),
            Base::Name => tx
                .names_under("", usize::MAX)?
                .into_iter()
                .map(|n| {
                    let t = match n.target {
                        Target::Content(d) | Target::Nominal(d) => d,
                    };
                    vec![Val::Text(n.path), Val::Id(t)]
                })
                .collect(),
            Base::Member => {
                let mut out = Vec::new();
                for n in tx.nominal_ids()? {
                    out.extend(tx.relation(&n)?.into_iter().map(|e| vec![Val::Id(n), Val::Id(e.member)]));
                }
                out
            }
            Base::Nominal => {
                let mut out = Vec::new();
                for n in tx.nominal_ids()? {
                    out.extend(tx.nominal(&n)?.map(|x| vec![Val::Id(n), Val::Text(x.kind)]));
                }
                out
            }
            Base::Uses => {
                let mut out = Vec::new();
                for d in tx.records_of_kind(None, usize::MAX)? {
                    if let Some(r) = tx.record(&d)? {
                        out.extend(r.nominal_refs.into_iter().map(|n| vec![Val::Id(d), Val::Id(n)]));
                    }
                }
                out
            }
            Base::Annotation => {
                let mut out = Vec::new();
                for s in self.all_subjects()? {
                    out.extend(tx.annotations_of(&s)?.into_iter().map(|a| vec![Val::Id(a.id), Val::Text(a.kind), Val::Id(s)]));
                }
                out
            }
            Base::Fact => {
                let mut out = Vec::new();
                for s in self.all_subjects()? {
                    out.extend(tx.facts_of(&s)?.into_iter().map(|f| vec![Val::Id(f.id), Val::Text(f.kind), Val::Id(s)]));
                }
                out
            }
        };
        let out = Rc::new(out);
        self.full.insert(b, out.clone());
        Ok(out)
    }

    fn value(&self, t: &Term, env: &[Option<Val>]) -> Option<Val> {
        match t {
            Term::Var(v) => env[*v as usize].clone(),
            Term::Param(p) => self.params.get(*p as usize).cloned(),
            Term::Const(c) => Some(c.clone()),
        }
    }

    /// Matches `args` against a tuple, binding unbound variables. Returns
    /// the variables it bound (to undo), or `None` on a mismatch.
    fn unify(&self, args: &[Term], t: &[Val], env: &mut [Option<Val>]) -> Option<Vec<u32>> {
        let mut newly = Vec::new();
        for (a, v) in args.iter().zip(t) {
            let ok = match a {
                Term::Var(x) => match &env[*x as usize] {
                    Some(cur) => cur == v,
                    None => {
                        env[*x as usize] = Some(v.clone());
                        newly.push(*x);
                        true
                    }
                },
                other => self.value(other, env).as_ref() == Some(v),
            };
            if !ok {
                for x in newly {
                    env[x as usize] = None;
                }
                return None;
            }
        }
        Some(newly)
    }

    fn tick(&mut self) -> Result<(), Error> {
        self.stats.steps += 1;
        if self.stats.steps > self.limits.max_steps_k.saturating_mul(1000) {
            return Err(Error::Limit(format!("more than {}k join steps", self.limits.max_steps_k)));
        }
        Ok(())
    }

    /// Candidate tuples for an atom given the bound columns.
    fn candidates(
        &mut self,
        a: &Atom,
        cols: &[usize],
        env: &[Option<Val>],
        rels: &Rels,
        delta_lit: bool,
    ) -> Result<Cand, Error> {
        match a.pred {
            Pred::Base(b) => {
                for &c in access_columns(b) {
                    if cols.contains(&c) {
                        let key = self.value(&a.args[c], env).unwrap();
                        return Ok(Cand::Owned(self.lookup(b, c, &key)?));
                    }
                }
                Ok(Cand::Owned(self.scan(b)?))
            }
            Pred::Local(p) => {
                let rel = if delta_lit { &rels.delta[p as usize] } else { &rels.full[p as usize] };
                if cols.is_empty() {
                    return Ok(Cand::All(rel.tuples.len()));
                }
                let key: Tuple = cols.iter().map(|&c| self.value(&a.args[c], env).unwrap()).collect();
                Ok(Cand::Some(rel.index[cols].get(&key).cloned().unwrap_or_default()))
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn join(
        &mut self,
        body: &[Literal],
        steps: &[Step],
        at: usize,
        delta: Option<usize>,
        env: &mut Vec<Option<Val>>,
        rels: &Rels,
        emit: &mut Emit<'_>,
    ) -> Result<(), Error> {
        self.tick()?;
        let Some(step) = steps.get(at) else {
            return emit(env);
        };
        let lit = &body[step.lit];
        match lit {
            Literal::Pos(a) => {
                let is_delta = delta == Some(step.lit);
                let cands = self.candidates(a, &step.bound_cols, env, rels, is_delta)?;
                let rel = match a.pred {
                    Pred::Local(p) => Some(if is_delta { &rels.delta[p as usize] } else { &rels.full[p as usize] }),
                    _ => None,
                };
                let n = cands.len();
                for k in 0..n {
                    let t: &[Val] = match &cands {
                        Cand::Owned(v) => &v[k],
                        Cand::All(_) => &rel.unwrap().tuples[k],
                        Cand::Some(ix) => &rel.unwrap().tuples[ix[k]],
                    };
                    if let Some(newly) = self.unify(&a.args, t, env) {
                        self.join(body, steps, at + 1, delta, env, rels, emit)?;
                        for x in newly {
                            env[x as usize] = None;
                        }
                    }
                }
            }
            Literal::Neg(a) => {
                let cands = self.candidates(a, &step.bound_cols, env, rels, false)?;
                let rel = match a.pred {
                    Pred::Local(p) => Some(&rels.full[p as usize]),
                    _ => None,
                };
                let mut found = false;
                for k in 0..cands.len() {
                    let t: &[Val] = match &cands {
                        Cand::Owned(v) => &v[k],
                        Cand::All(_) => &rel.unwrap().tuples[k],
                        Cand::Some(ix) => &rel.unwrap().tuples[ix[k]],
                    };
                    if let Some(newly) = self.unify(&a.args, t, env) {
                        for x in newly {
                            env[x as usize] = None;
                        }
                        found = true;
                        break;
                    }
                }
                if !found {
                    self.join(body, steps, at + 1, delta, env, rels, emit)?;
                }
            }
            Literal::Eq(x, y) => match (self.value(x, env), self.value(y, env)) {
                (Some(a), Some(b)) => {
                    if a == b {
                        self.join(body, steps, at + 1, delta, env, rels, emit)?;
                    }
                }
                (Some(a), None) | (None, Some(a)) => {
                    let Term::Var(v) = (if self.value(x, env).is_none() { x } else { y }) else { unreachable!() };
                    env[*v as usize] = Some(a);
                    self.join(body, steps, at + 1, delta, env, rels, emit)?;
                    env[*v as usize] = None;
                }
                _ => unreachable!("the planner orders an equation after one side is bound"),
            },
            Literal::Ne(x, y) => {
                if self.value(x, env) != self.value(y, env) {
                    self.join(body, steps, at + 1, delta, env, rels, emit)?;
                }
            }
            Literal::StartsWith(x, y) => {
                let ok = matches!((self.value(x, env), self.value(y, env)), (Some(Val::Text(a)), Some(Val::Text(b))) if a.starts_with(&b));
                if ok {
                    self.join(body, steps, at + 1, delta, env, rels, emit)?;
                }
            }
        }
        Ok(())
    }
}

enum Cand {
    Owned(Rc<Vec<Tuple>>),
    All(usize),
    Some(Vec<usize>),
}

impl Cand {
    fn len(&self) -> usize {
        match self {
            Cand::Owned(v) => v.len(),
            Cand::All(n) => *n,
            Cand::Some(v) => v.len(),
        }
    }
}

struct Rels {
    full: Vec<Rel>,
    delta: Vec<Rel>,
}

/// A rule prepared for one semi-naive variant.
struct Variant {
    rule: usize,
    delta: Option<usize>,
    steps: Vec<Step>,
}

fn ground(t: &Term, env: &[Option<Val>], params: &[Val]) -> Val {
    match t {
        Term::Var(v) => env[*v as usize].clone().expect("safe rule"),
        Term::Param(p) => params[*p as usize].clone(),
        Term::Const(c) => c.clone(),
    }
}

/// The result of a run, before column names.
pub struct Evaluated {
    pub rows: Vec<Vec<Val>>,
    pub truncated: bool,
}

/// Evaluates a checked query. `strata` must be the strata of `q` (from
/// [`crate::check::stratify`]).
pub fn run(exec: &mut Exec, q: &Query, strata: &[Vec<u32>]) -> Result<Evaluated, Error> {
    if exec.params.len() != q.params.len() {
        return Err(Error::Check(format!("the query takes {} parameters, {} given", q.params.len(), exec.params.len())));
    }
    for (i, (v, s)) in exec.params.iter().zip(&q.params).enumerate() {
        if v.sort() != *s {
            return Err(Error::Check(format!("parameter {} must be {}, not {}", i + 1, s.as_str(), v.sort().as_str())));
        }
    }
    let n = q.preds.len();
    let mut rels = Rels { full: (0..n).map(|_| Rel::default()).collect(), delta: (0..n).map(|_| Rel::default()).collect() };
    let params = exec.params;

    for stratum in strata {
        let in_s = |p: u32| stratum.contains(&p);
        let rules: Vec<usize> = (0..q.rules.len()).filter(|&i| in_s(q.rules[i].head)).collect();
        let recursive = |r: &Rule| {
            r.body.iter().enumerate().filter_map(|(i, l)| match l {
                Literal::Pos(Atom { pred: Pred::Local(p), .. }) if in_s(*p) => Some(i),
                _ => None,
            }).collect::<Vec<_>>()
        };
        // Variants: each rule once in the first round; each recursive
        // literal as the delta in later rounds.
        let plan = |r: &Rule, first: Option<usize>| -> Vec<Step> {
            let mut bound = vec![false; r.vars as usize];
            let order = plan::order(&r.args, &r.body, &mut bound, first).expect("checked");
            steps(&r.body, &order, r.vars, &[])
        };
        let first_round: Vec<Variant> = rules.iter().map(|&i| Variant { rule: i, delta: None, steps: plan(&q.rules[i], None) }).collect();
        let later: Vec<Variant> = rules
            .iter()
            .flat_map(|&i| recursive(&q.rules[i]).into_iter().map(move |d| (i, d)))
            .map(|(i, d)| Variant { rule: i, delta: Some(d), steps: plan(&q.rules[i], Some(d)) })
            .collect();

        let mut round = 0;
        loop {
            let variants = if round == 0 { &first_round } else { &later };
            if round > 0 && variants.is_empty() {
                break;
            }
            // Indexes the variants need.
            for v in variants {
                let r = &q.rules[v.rule];
                for s in &v.steps {
                    if let Literal::Pos(Atom { pred: Pred::Local(p), .. }) | Literal::Neg(Atom { pred: Pred::Local(p), .. }) = &r.body[s.lit] {
                        rels.full[*p as usize].ensure_index(&s.bound_cols);
                        if v.delta == Some(s.lit) {
                            rels.delta[*p as usize].ensure_index(&s.bound_cols);
                        }
                    }
                }
            }
            // New tuples of this round, deduplicated as they are produced
            // against everything known, so duplicates never accumulate.
            let mut fresh: Vec<Rel> = (0..n).map(|_| Rel::default()).collect();
            let max_derived = exec.limits.max_derived;
            let mut derived = exec.stats.derived;
            for v in variants {
                let r = &q.rules[v.rule];
                let mut env = vec![None; r.vars as usize];
                let full = &rels.full[r.head as usize];
                let fresh_p = &mut fresh[r.head as usize];
                let mut emit = |env: &[Option<Val>]| -> Result<(), Error> {
                    let t: Tuple = r.args.iter().map(|t| ground(t, env, params)).collect();
                    if !full.set.contains(&t) && fresh_p.insert(t) {
                        derived += 1;
                        if derived > max_derived {
                            return Err(Error::Limit(format!("more than {max_derived} derived tuples")));
                        }
                    }
                    Ok(())
                };
                exec.join(&r.body, &v.steps, 0, v.delta, &mut env, &rels, &mut emit)?;
            }
            exec.stats.derived = derived;
            exec.stats.iterations += 1;
            let mut any = false;
            for &p in stratum {
                let new = std::mem::take(&mut fresh[p as usize]);
                for t in &new.tuples {
                    rels.full[p as usize].insert(t.clone());
                }
                any |= !new.tuples.is_empty();
                rels.delta[p as usize] = Rel { tuples: new.tuples, set: new.set, index: HashMap::new() };
            }
            round += 1;
            if !any || later.is_empty() {
                break;
            }
        }
    }

    // The select.
    let s = &q.select;
    let out_terms: Vec<Term> = s.outputs.iter().map(|o| match o { Output::Var(v) | Output::Count(v) => Term::Var(*v) }).collect();
    let mut bound = vec![false; s.vars as usize];
    let order = plan::order(&out_terms, &s.body, &mut bound, None).expect("checked");
    let st = steps(&s.body, &order, s.vars, &[]);
    for step in &st {
        if let Literal::Pos(Atom { pred: Pred::Local(p), .. }) | Literal::Neg(Atom { pred: Pred::Local(p), .. }) = &s.body[step.lit] {
            rels.full[*p as usize].ensure_index(&step.bound_cols);
        }
    }
    let has_count = s.outputs.iter().any(|o| matches!(o, Output::Count(_)));
    let mut seen: HashSet<Tuple> = HashSet::new();
    let mut rows: Vec<Tuple> = Vec::new();
    let mut env = vec![None; s.vars as usize];
    // Without counts, the select stops once it has one row more than the cap
    // (to know it truncated); with counts, rows before grouping are bounded
    // like derived tuples.
    let cap = if has_count { exec.limits.max_derived as usize } else { exec.limits.max_rows.saturating_add(1) };
    let mut full_stop = false;
    {
        let mut emit = |env: &[Option<Val>]| -> Result<(), Error> {
            let t: Tuple = out_terms.iter().map(|t| ground(t, env, params)).collect();
            if seen.insert(t.clone()) {
                rows.push(t);
                if rows.len() >= cap {
                    full_stop = true;
                    return Err(Error::Limit("rows".into()));
                }
            }
            Ok(())
        };
        match exec.join(&s.body, &st, 0, None, &mut env, &rels, &mut emit) {
            Err(Error::Limit(_)) if full_stop && !has_count => {}
            Err(Error::Limit(_)) if full_stop => {
                return Err(Error::Limit(format!("more than {cap} rows before grouping")));
            }
            other => other?,
        }
    }
    if has_count {
        // Group by the plain outputs; count distinct values per counted output.
        let groups_cols: Vec<usize> = (0..s.outputs.len()).filter(|&i| matches!(s.outputs[i], Output::Var(_))).collect();
        let mut groups: Vec<(Tuple, Vec<HashSet<Val>>)> = Vec::new();
        let mut gi: HashMap<Tuple, usize> = HashMap::new();
        for r in rows {
            let key: Tuple = groups_cols.iter().map(|&c| r[c].clone()).collect();
            let g = *gi.entry(key.clone()).or_insert_with(|| {
                groups.push((key, vec![HashSet::new(); s.outputs.len()]));
                groups.len() - 1
            });
            for (c, o) in s.outputs.iter().enumerate() {
                if matches!(o, Output::Count(_)) {
                    groups[g].1[c].insert(r[c].clone());
                }
            }
        }
        rows = groups
            .into_iter()
            .map(|(key, counts)| {
                let mut k = key.into_iter();
                s.outputs
                    .iter()
                    .enumerate()
                    .map(|(c, o)| match o {
                        Output::Var(_) => k.next().unwrap(),
                        Output::Count(_) => Val::Int(counts[c].len() as i64),
                    })
                    .collect()
            })
            .collect();
    }
    if has_count && rows.is_empty() && s.outputs.iter().all(|o| matches!(o, Output::Count(_))) {
        // Counting over no solutions gives one row of zeros, as in SQL.
        rows.push(vec![Val::Int(0); s.outputs.len()]);
    }
    let truncated = rows.len() > exec.limits.max_rows;
    rows.truncate(exec.limits.max_rows);
    Ok(Evaluated { rows, truncated })
}
