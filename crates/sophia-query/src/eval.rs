//! The native executor (SPEC-007 §6): semi-naive bottom-up evaluation of a
//! checked query over any backend's `ReadTx`. Base relations are read
//! through access paths, memoised per run; local relations are hash-indexed
//! on the columns the plan looks them up by. See `eval.md`.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sophia_hash::Digest;
use sophia_store::{ReadTx, Target};

use crate::Error;
use crate::ir::*;
use crate::plan::{self, access_columns};

type Tuple = Vec<Val>;

/// Memoised lookups of one access path: key value → tuples.
type Memo = HashMap<Val, Rc<Vec<Tuple>>>;

/// Receives each solution of a body; an error stops the join.
type Emit<'e> = dyn FnMut(&[Option<Val>]) -> Result<(), Error> + 'e;

/// The resource budget of one run (SPEC-007 §6.0). Every limit aborts the
/// run with [`Error::Limit`], except `max_rows`, which truncates.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_rows: usize,
    /// Join steps, in thousands, before the run is aborted.
    pub max_steps_k: u64,
    /// Tuples derived for local predicates (and select rows before
    /// grouping) before the run is aborted. A logical limit, independent of
    /// tuple sizes.
    pub max_derived: u64,
    /// **Estimated** bytes held by the run: derived tuples (their list, set
    /// and index copies), memoised and scanned base tuples, and select rows.
    /// An estimate from value sizes, not an allocator count: it bounds what
    /// the executor keeps, not every temporary.
    pub max_bytes: u64,
    /// Wall-clock time before the run is aborted.
    pub deadline: Option<Duration>,
}

/// How the executor chooses between tuple-at-a-time and set-at-a-time
/// access (B-014). Results never depend on it; only the work does. Tests
/// and benchmarks set it to compare both ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tuning {
    /// Probes of a negated base literal, on one column set, after which the
    /// executor scans the relation once and probes a hash set of its keys
    /// instead (an anti-join). `0` builds the set on the first probe;
    /// `u64::MAX` never does.
    pub anti_join_after: u64,
    /// Whether `name(p, _)` with a bound `startswith(p, s)` reads only the
    /// names that start with `s` (a range scan) instead of every name.
    pub prefix_paths: bool,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning { anti_join_after: 256, prefix_paths: true }
    }
}

impl Tuning {
    /// Tuple at a time throughout: the executor before B-014.
    pub const TUPLE_AT_A_TIME: Tuning = Tuning { anti_join_after: u64::MAX, prefix_paths: false };
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_rows: 10_000,
            max_steps_k: 1_000_000,
            max_derived: 5_000_000,
            max_bytes: 1 << 30,
            deadline: Some(Duration::from_secs(30)),
        }
    }
}

/// What a run did, for benchmarks and `explain`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Access-path lookups that reached the backend (memoised ones excluded).
    pub lookups: u64,
    /// Full scans of a base relation.
    pub scans: u64,
    /// Name prefixes read through the prefix access path.
    pub prefix_reads: u64,
    /// Hash sets built for negated base literals (anti-joins).
    pub anti_joins: u64,
    /// Tuples derived for local predicates.
    pub derived: u64,
    /// Semi-naive iterations, summed over strata.
    pub iterations: u64,
    pub steps: u64,
    /// The estimated bytes held at the end (see [`Limits::max_bytes`]).
    pub bytes: u64,
}

/// Estimated heap and inline bytes of one tuple.
pub(crate) fn tuple_bytes(t: &[Val]) -> u64 {
    let heap: usize = t.iter().map(|v| if let Val::Text(s) = v { s.len() } else { 0 }).sum();
    (std::mem::size_of::<Vec<Val>>() + std::mem::size_of_val(t) + heap) as u64
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

    /// Builds an index on `cols` if missing; returns its estimated bytes.
    fn ensure_index(&mut self, cols: &[usize]) -> u64 {
        if cols.is_empty() || self.index.contains_key(cols) {
            return 0;
        }
        let mut idx: HashMap<Tuple, Vec<usize>> = HashMap::new();
        let mut bytes = 0;
        for (i, t) in self.tuples.iter().enumerate() {
            let key: Tuple = cols.iter().map(|&c| t[c].clone()).collect();
            bytes += tuple_bytes(&key) + 8;
            idx.entry(key).or_default().push(i);
        }
        self.index.insert(cols.to_vec(), idx);
        bytes
    }
}

/// A body literal with the columns bound when it runs.
#[derive(Clone)]
struct Step {
    lit: usize,
    bound_cols: Vec<usize>,
    /// For `name(p, _)`: the bound prefix `s` of a `startswith(p, s)`, read
    /// as a range ([`plan::name_prefix`]).
    prefix: Option<Term>,
    /// For a negated literal: its unbound arguments are distinct variables,
    /// so "some tuple matches" is "the bound columns' key is in the
    /// relation's projection", and an anti-join may answer it.
    anti: bool,
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
            let anti = match &body[i] {
                Literal::Neg(a) => {
                    let mut free: Vec<&Term> = a.args.iter().filter(|t| !plan::is_bound(t, &bound)).collect();
                    let n = free.len();
                    free.sort_by_key(|t| match t {
                        Term::Var(v) => *v,
                        _ => u32::MAX,
                    });
                    free.dedup();
                    free.len() == n
                }
                _ => false,
            };
            let prefix = plan::name_prefix(body, i, &bound).cloned();
            plan::bind(&body[i], &mut bound);
            Step { lit: i, bound_cols: cols, prefix, anti }
        })
        .collect()
}

/// A negated literal's anti-join state: probes counted so far, or the
/// projection of the relation on the bound columns.
enum Anti {
    Counting(u64),
    Built(Rc<HashSet<Tuple>>),
}

pub struct Exec<'a> {
    tx: &'a dyn ReadTx,
    params: &'a [Val],
    limits: Limits,
    tuning: Tuning,
    memo: HashMap<(Base, usize), Memo>,
    full: HashMap<Base, Rc<Vec<Tuple>>>,
    /// Names read by prefix, memoised per prefix.
    prefixed: HashMap<String, Rc<Vec<Tuple>>>,
    /// Anti-join state per (relation, bound columns); a short list.
    anti: Vec<(Base, Vec<usize>, Anti)>,
    /// Variables bound by `unify`, undone by `undo` (no allocation per tuple).
    trail: Vec<u32>,
    /// The estimated bytes held, shared with the closures that receive a
    /// join's solutions: base reads in the middle of a join and the tuples
    /// the solutions add are counted against one budget.
    held: Rc<Cell<u64>>,
    /// When the run started, if a deadline is to be enforced; see [`clock`].
    started: Option<Instant>,
    cancel: Option<&'a AtomicBool>,
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

/// Adds `n` to the estimated bytes held, failing over `max`.
fn charge(held: &Cell<u64>, n: u64, max: u64) -> Result<(), Error> {
    held.set(held.get().saturating_add(n));
    if held.get() > max {
        return Err(Error::Limit(format!("more than {max} estimated bytes")));
    }
    Ok(())
}

/// A term's value, borrowed: from the environment, the parameters, or the
/// term itself. `None` for an unbound variable.
fn val<'v>(params: &'v [Val], t: &'v Term, env: &'v [Option<Val>]) -> Option<&'v Val> {
    match t {
        Term::Var(v) => env[*v as usize].as_ref(),
        Term::Param(p) => params.get(*p as usize),
        Term::Const(c) => Some(c),
    }
}

/// The start of a run, for the deadline. `wasm32-unknown-unknown` has no
/// clock in `std` (`Instant::now` panics), so there the deadline is not
/// enforced and the step and row budgets bound the work alone. A browser
/// clock (`performance.now()`) can replace this placeholder (SPEC-009 §4).
#[cfg(not(target_arch = "wasm32"))]
fn clock() -> Option<Instant> {
    Some(Instant::now())
}

#[cfg(target_arch = "wasm32")]
fn clock() -> Option<Instant> {
    None
}

impl<'a> Exec<'a> {
    pub fn new(tx: &'a dyn ReadTx, params: &'a [Val], limits: Limits) -> Self {
        Exec {
            tx,
            params,
            limits,
            tuning: Tuning::default(),
            memo: HashMap::new(),
            full: HashMap::new(),
            prefixed: HashMap::new(),
            anti: Vec::new(),
            trail: Vec::new(),
            held: Rc::new(Cell::new(0)),
            started: clock(),
            cancel: None,
            stats: Stats::default(),
        }
    }

    /// A run with other set-at-a-time choices ([`Tuning`]).
    pub fn with_tuning(mut self, tuning: Tuning) -> Self {
        self.tuning = tuning;
        self
    }

    /// A run that stops with [`Error::Limit`] once `flag` is set, from any
    /// thread.
    pub fn with_cancel(mut self, flag: &'a AtomicBool) -> Self {
        self.cancel = Some(flag);
        self
    }

    /// Adds to the estimated bytes held, failing over budget.
    fn charge(&mut self, n: u64) -> Result<(), Error> {
        charge(&self.held, n, self.limits.max_bytes)?;
        self.stats.bytes = self.held.get();
        Ok(())
    }

    fn all_subjects(&self) -> Result<Vec<Digest>, Error> {
        let mut out = self.tx.records_of_kind(None, usize::MAX)?;
        out.extend(self.tx.nominal_ids()?);
        Ok(out)
    }

    /// Tuples of a base relation whose column `col` equals `key`.
    fn lookup(&mut self, b: Base, col: usize, key: &Val) -> Result<Rc<Vec<Tuple>>, Error> {
        if let Some(r) = self.memo.get(&(b, col)).and_then(|m| m.get(key)) {
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
            (Base::Member, 1) => {
                if let Some(d) = id(key) {
                    out.extend(tx.memberships_of(d)?.into_iter().map(|n| vec![Val::Id(n), key.clone()]));
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
            (Base::Annotation, 0) => {
                if let Some(d) = id(key) {
                    if let Some(a) = tx.annotation(d)? {
                        out.extend(a.subjects.iter().map(|s| vec![key.clone(), Val::Text(a.kind.clone()), Val::Id(*s)]));
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
        self.charge(out.iter().map(|t| tuple_bytes(t)).sum::<u64>() + 64)?;
        self.memo.entry((b, col)).or_default().insert(key.clone(), out.clone());
        Ok(out)
    }

    /// The `name` tuples whose path starts with `prefix` (any string, not
    /// only a namespace), read as a range: pages of
    /// [`ReadTx::names_from`] from `prefix` under its namespace, stopping at
    /// the first path past the range. Memoised per prefix.
    fn names_with_prefix(&mut self, prefix: &Val) -> Result<Rc<Vec<Tuple>>, Error> {
        let Some(p) = text(prefix) else { return Ok(Rc::new(Vec::new())) };
        if let Some(r) = self.prefixed.get(p) {
            return Ok(r.clone());
        }
        self.stats.prefix_reads += 1;
        let base = p.rfind('.').map_or("", |i| &p[..=i]);
        let mut out: Vec<Tuple> = Vec::new();
        let mut from = p.to_string();
        let mut page = 64;
        'pages: loop {
            let names = self.tx.names_from(base, &from, page)?;
            let more = names.len() == page;
            for n in names {
                if !n.path.starts_with(p) {
                    break 'pages;
                }
                let t = match n.target {
                    Target::Content(d) | Target::Nominal(d) => d,
                };
                from.clear();
                from.push_str(&n.path);
                out.push(vec![Val::Text(n.path), Val::Id(t)]);
            }
            if !more {
                break;
            }
            // `from` is inclusive: the next path after the last one read.
            from.push('\0');
            page = (page * 2).min(4096);
        }
        let out = Rc::new(out);
        self.charge(out.iter().map(|t| tuple_bytes(t)).sum::<u64>() + 64)?;
        self.prefixed.insert(p.to_string(), out.clone());
        Ok(out)
    }

    /// The anti-join set for a negated base literal on `cols`, once
    /// [`Tuning::anti_join_after`] probes have been made; `None` before.
    fn anti_set(&mut self, b: Base, cols: &[usize]) -> Result<Option<Rc<HashSet<Tuple>>>, Error> {
        let i = match self.anti.iter().position(|(x, c, _)| *x == b && c == cols) {
            Some(i) => i,
            None => {
                self.anti.push((b, cols.to_vec(), Anti::Counting(0)));
                self.anti.len() - 1
            }
        };
        let probes = match &mut self.anti[i].2 {
            Anti::Built(s) => return Ok(Some(s.clone())),
            Anti::Counting(n) => {
                *n += 1;
                *n
            }
        };
        if probes <= self.tuning.anti_join_after {
            return Ok(None);
        }
        let all = self.scan(b)?;
        let set: HashSet<Tuple> = all.iter().map(|t| cols.iter().map(|&c| t[c].clone()).collect()).collect();
        self.charge(set.iter().map(|k| tuple_bytes(k)).sum::<u64>())?;
        self.stats.anti_joins += 1;
        let set = Rc::new(set);
        self.anti[i].2 = Anti::Built(set.clone());
        Ok(Some(set))
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
        self.charge(out.iter().map(|t| tuple_bytes(t)).sum())?;
        self.full.insert(b, out.clone());
        Ok(out)
    }

    /// Matches `args` against a tuple, binding unbound variables and
    /// recording them on the trail. On a mismatch it undoes its own
    /// bindings and returns `false`; on a match the caller undoes them with
    /// [`Exec::undo`] from the trail length before the call.
    fn unify(&mut self, args: &[Term], t: &[Val], env: &mut [Option<Val>]) -> bool {
        let mark = self.trail.len();
        for (a, v) in args.iter().zip(t) {
            let ok = match a {
                Term::Var(x) => match &env[*x as usize] {
                    Some(cur) => cur == v,
                    None => {
                        env[*x as usize] = Some(v.clone());
                        self.trail.push(*x);
                        true
                    }
                },
                other => val(self.params, other, env) == Some(v),
            };
            if !ok {
                self.undo(mark, env);
                return false;
            }
        }
        true
    }

    fn undo(&mut self, mark: usize, env: &mut [Option<Val>]) {
        for x in self.trail.drain(mark..) {
            env[x as usize] = None;
        }
    }

    fn tick(&mut self) -> Result<(), Error> {
        self.stats.steps += 1;
        if self.stats.steps > self.limits.max_steps_k.saturating_mul(1000) {
            return Err(Error::Limit(format!("more than {}k join steps", self.limits.max_steps_k)));
        }
        if self.stats.steps % 4096 == 0 {
            if self.cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                return Err(Error::Limit("cancelled".into()));
            }
            if self.limits.deadline.is_some_and(|d| self.started.is_some_and(|s| s.elapsed() > d)) {
                return Err(Error::Limit(format!("over the deadline of {:?}", self.limits.deadline.unwrap())));
            }
        }
        Ok(())
    }

    /// Candidate tuples for an atom given the bound columns.
    fn candidates(&mut self, a: &Atom, step: &Step, env: &[Option<Val>], rels: &Rels, delta_lit: bool) -> Result<Cand, Error> {
        let cols = &step.bound_cols;
        let params = self.params;
        match a.pred {
            Pred::Base(b) => {
                for &c in access_columns(b) {
                    if cols.contains(&c) {
                        let key = val(params, &a.args[c], env).unwrap();
                        return Ok(Cand::Owned(self.lookup(b, c, key)?));
                    }
                }
                if let (Some(s), true) = (&step.prefix, self.tuning.prefix_paths) {
                    return Ok(Cand::Owned(self.names_with_prefix(val(params, s, env).unwrap())?));
                }
                Ok(Cand::Owned(self.scan(b)?))
            }
            Pred::Local(p) => {
                let rel = if delta_lit { &rels.delta[p as usize] } else { &rels.full[p as usize] };
                if cols.is_empty() {
                    return Ok(Cand::All(rel.tuples.len()));
                }
                let ix = &rel.index[cols];
                let hit = if let [c] = cols[..] {
                    ix.get(std::slice::from_ref(val(params, &a.args[c], env).unwrap()))
                } else {
                    let key: Tuple = cols.iter().map(|&c| val(params, &a.args[c], env).unwrap().clone()).collect();
                    ix.get(&key)
                };
                Ok(Cand::Some(hit.cloned().unwrap_or_default()))
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
                let cands = self.candidates(a, step, env, rels, is_delta)?;
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
                    let mark = self.trail.len();
                    if self.unify(&a.args, t, env) {
                        self.join(body, steps, at + 1, delta, env, rels, emit)?;
                        self.undo(mark, env);
                    }
                }
            }
            Literal::Neg(a) => {
                let anti = match a.pred {
                    Pred::Base(b) if step.anti => self.anti_set(b, &step.bound_cols)?,
                    _ => None,
                };
                let found = if let Some(set) = anti {
                    let params = self.params;
                    if let [c] = step.bound_cols[..] {
                        set.contains(std::slice::from_ref(val(params, &a.args[c], env).unwrap()))
                    } else {
                        let key: Tuple = step.bound_cols.iter().map(|&c| val(params, &a.args[c], env).unwrap().clone()).collect();
                        set.contains(&key)
                    }
                } else {
                    let cands = self.candidates(a, step, env, rels, false)?;
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
                        let mark = self.trail.len();
                        if self.unify(&a.args, t, env) {
                            self.undo(mark, env);
                            found = true;
                            break;
                        }
                    }
                    found
                };
                if !found {
                    self.join(body, steps, at + 1, delta, env, rels, emit)?;
                }
            }
            Literal::Eq(x, y) => {
                let params = self.params;
                match (val(params, x, env), val(params, y, env)) {
                    (Some(a), Some(b)) => {
                        if a == b {
                            self.join(body, steps, at + 1, delta, env, rels, emit)?;
                        }
                    }
                    (Some(a), None) | (None, Some(a)) => {
                        let a = a.clone();
                        let Term::Var(v) = (if val(params, x, env).is_none() { x } else { y }) else { unreachable!() };
                        env[*v as usize] = Some(a);
                        self.join(body, steps, at + 1, delta, env, rels, emit)?;
                        env[*v as usize] = None;
                    }
                    _ => unreachable!("the planner orders an equation after one side is bound"),
                }
            }
            Literal::Ne(x, y) => {
                if val(self.params, x, env) != val(self.params, y, env) {
                    self.join(body, steps, at + 1, delta, env, rels, emit)?;
                }
            }
            Literal::StartsWith(x, y) => {
                let ok = matches!((val(self.params, x, env), val(self.params, y, env)), (Some(Val::Text(a)), Some(Val::Text(b))) if a.starts_with(b.as_str()));
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
            r.body
                .iter()
                .enumerate()
                .filter_map(|(i, l)| match l {
                    Literal::Pos(Atom { pred: Pred::Local(p), .. }) if in_s(*p) => Some(i),
                    _ => None,
                })
                .collect::<Vec<_>>()
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
                    if let Literal::Pos(Atom { pred: Pred::Local(p), .. }) | Literal::Neg(Atom { pred: Pred::Local(p), .. }) =
                        &r.body[s.lit]
                    {
                        let b = rels.full[*p as usize].ensure_index(&s.bound_cols);
                        exec.charge(b)?;
                        if v.delta == Some(s.lit) {
                            let b = rels.delta[*p as usize].ensure_index(&s.bound_cols);
                            exec.charge(b)?;
                        }
                    }
                }
            }
            // New tuples of this round, deduplicated as they are produced
            // against everything known, so duplicates never accumulate.
            let mut fresh: Vec<Rel> = (0..n).map(|_| Rel::default()).collect();
            let max_derived = exec.limits.max_derived;
            let max_bytes = exec.limits.max_bytes;
            let mut derived = exec.stats.derived;
            let held = exec.held.clone();
            for v in variants {
                let r = &q.rules[v.rule];
                let mut env = vec![None; r.vars as usize];
                let full = &rels.full[r.head as usize];
                let fresh_p = &mut fresh[r.head as usize];
                let mut emit = |env: &[Option<Val>]| -> Result<(), Error> {
                    let t: Tuple = r.args.iter().map(|t| ground(t, env, params)).collect();
                    let b = tuple_bytes(&t);
                    if !full.set.contains(&t) && fresh_p.insert(t) {
                        derived += 1;
                        // The fresh list and set now, the full list and set
                        // after the round: four copies.
                        if derived > max_derived {
                            return Err(Error::Limit(format!("more than {max_derived} derived tuples")));
                        }
                        charge(&held, 4 * b, max_bytes)?;
                    }
                    Ok(())
                };
                exec.join(&r.body, &v.steps, 0, v.delta, &mut env, &rels, &mut emit)?;
            }
            exec.stats.derived = derived;
            exec.stats.bytes = exec.held.get();
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
    let out_terms: Vec<Term> = s
        .outputs
        .iter()
        .map(|o| match o {
            Output::Var(v) | Output::Count(v) => Term::Var(*v),
        })
        .collect();
    let mut bound = vec![false; s.vars as usize];
    let order = plan::order(&out_terms, &s.body, &mut bound, None).expect("checked");
    let st = steps(&s.body, &order, s.vars, &[]);
    for step in &st {
        if let Literal::Pos(Atom { pred: Pred::Local(p), .. }) | Literal::Neg(Atom { pred: Pred::Local(p), .. }) = &s.body[step.lit] {
            let b = rels.full[*p as usize].ensure_index(&step.bound_cols);
            exec.charge(b)?;
        }
    }
    let has_count = s.outputs.iter().any(|o| matches!(o, Output::Count(_)));
    let mut env = vec![None; s.vars as usize];
    let max_bytes = exec.limits.max_bytes;
    let held = exec.held.clone();
    let mut rows: Vec<Tuple> = Vec::new();
    if has_count {
        // Grouped as solutions arrive: the plain outputs are the group key,
        // and each counted output keeps the set of its distinct values in
        // the group. Counting distinct values per group does not need the
        // distinct rows first. Groups plus distinct counted values are
        // bounded like derived tuples.
        let group_cols: Vec<usize> = (0..s.outputs.len()).filter(|&i| matches!(s.outputs[i], Output::Var(_))).collect();
        let count_cols: Vec<usize> = (0..s.outputs.len()).filter(|&i| matches!(s.outputs[i], Output::Count(_))).collect();
        let cap = exec.limits.max_derived;
        let mut entries: u64 = 0;
        let mut groups: Vec<(Tuple, Vec<HashSet<Val>>)> = Vec::new();
        let mut gi: HashMap<Tuple, usize> = HashMap::new();
        {
            let mut emit = |env: &[Option<Val>]| -> Result<(), Error> {
                let get = |c: usize| val(params, &out_terms[c], env).expect("safe select");
                let hit = if let [c] = group_cols[..] {
                    gi.get(std::slice::from_ref(get(c))).copied()
                } else {
                    gi.get(&group_cols.iter().map(|&c| get(c).clone()).collect::<Tuple>()).copied()
                };
                let g = match hit {
                    Some(g) => g,
                    None => {
                        let key: Tuple = group_cols.iter().map(|&c| get(c).clone()).collect();
                        charge(&held, 2 * tuple_bytes(&key), max_bytes)?;
                        entries += 1;
                        gi.insert(key.clone(), groups.len());
                        groups.push((key, vec![HashSet::new(); count_cols.len()]));
                        groups.len() - 1
                    }
                };
                for (j, &c) in count_cols.iter().enumerate() {
                    let v = get(c);
                    if !groups[g].1[j].contains(v) {
                        charge(&held, tuple_bytes(std::slice::from_ref(v)), max_bytes)?;
                        entries += 1;
                        groups[g].1[j].insert(v.clone());
                    }
                }
                if entries > cap {
                    return Err(Error::Limit(format!("more than {cap} groups and distinct values while counting")));
                }
                Ok(())
            };
            exec.join(&s.body, &st, 0, None, &mut env, &rels, &mut emit)?;
        }
        rows = groups
            .into_iter()
            .map(|(key, counts)| {
                let mut k = key.into_iter();
                let mut n = counts.into_iter();
                s.outputs
                    .iter()
                    .map(|o| match o {
                        Output::Var(_) => k.next().unwrap(),
                        Output::Count(_) => Val::Int(n.next().unwrap().len() as i64),
                    })
                    .collect()
            })
            .collect();
        if rows.is_empty() && group_cols.is_empty() {
            // Counting over no solutions gives one row of zeros, as in SQL.
            rows.push(vec![Val::Int(0); s.outputs.len()]);
        }
    } else {
        // The select stops once it has one row more than the cap, to know
        // it truncated.
        let cap = exec.limits.max_rows.saturating_add(1);
        let mut seen: HashSet<Tuple> = HashSet::new();
        let mut full_stop = false;
        {
            let mut emit = |env: &[Option<Val>]| -> Result<(), Error> {
                let t: Tuple = out_terms.iter().map(|t| ground(t, env, params)).collect();
                if !seen.contains(&t) {
                    charge(&held, 2 * tuple_bytes(&t), max_bytes)?;
                    seen.insert(t.clone());
                    rows.push(t);
                    if rows.len() >= cap {
                        full_stop = true;
                        return Err(Error::Limit("rows".into()));
                    }
                }
                Ok(())
            };
            match exec.join(&s.body, &st, 0, None, &mut env, &rels, &mut emit) {
                Err(Error::Limit(_)) if full_stop => {}
                other => other?,
            }
        }
    }
    exec.stats.bytes = exec.held.get();
    let truncated = rows.len() > exec.limits.max_rows;
    rows.truncate(exec.limits.max_rows);
    Ok(Evaluated { rows, truncated })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neg(args: Vec<Term>) -> Literal {
        Literal::Neg(Atom { pred: Pred::Base(Base::Refs), args })
    }

    /// An anti-join answers a negation only when its unbound arguments are
    /// distinct variables: `!refs(x, x)` with `x` unbound asks for a
    /// self-reference, which the projection on no columns cannot see.
    #[test]
    fn anti_join_needs_distinct_free_variables() {
        let def = Literal::Pos(Atom { pred: Pred::Base(Base::Def), args: vec![Term::Var(0), Term::Var(1)] });
        for (lit, anti) in [
            (neg(vec![Term::Var(2), Term::Var(2)]), false),
            (neg(vec![Term::Var(2), Term::Var(3)]), true),
            (neg(vec![Term::Var(0), Term::Var(0)]), true),
            (neg(vec![Term::Var(2), Term::Var(0)]), true),
            (neg(vec![Term::Const(Val::Id(Digest::of(b"x"))), Term::Var(2)]), true),
        ] {
            let body = vec![def.clone(), lit];
            let st = steps(&body, &[0, 1], 4, &[]);
            assert_eq!(st[1].anti, anti, "{:?}", body[1]);
        }
    }
}
