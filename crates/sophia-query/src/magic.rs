//! The magic-set rewrite (SPEC-007 §6.2): a query-to-query transformation
//! that makes bottom-up evaluation goal-directed. "Transitive dependants of
//! X" then explores only what X reaches, not the closure of the whole store.
//! It is the first optimisation expressed as IR → IR, so its result is again
//! a query that both executors (native and SQL) run. See `magic.md`.
//!
//! Sources: Bancilhon, Maier, Sagiv and Ullman, *Magic Sets and Other
//! Strange Ways to Implement Logic Programs*, PODS 1986; Beeri and
//! Ramakrishnan, *On the Power of Magic*, JLP 1991.

use std::collections::HashMap;

use crate::check::stratify;
use crate::ir::*;
use crate::plan;

struct Rewriter<'a> {
    q: &'a Query,
    preds: Vec<u32>,
    adorned: HashMap<(u32, Vec<bool>), u32>,
    magic: HashMap<(u32, Vec<bool>), u32>,
    work: Vec<(u32, Vec<bool>)>,
    rules: Vec<Rule>,
    /// For display and debugging: the name of each new predicate.
    pub names: Vec<String>,
    old_names: &'a [String],
}

fn mask_str(m: &[bool]) -> String {
    m.iter().map(|&b| if b { 'b' } else { 'f' }).collect()
}

impl Rewriter<'_> {
    fn pname(&self, p: u32) -> String {
        self.old_names.get(p as usize).cloned().unwrap_or_else(|| format!("p{p}"))
    }

    fn adorned(&mut self, p: u32, mask: Vec<bool>) -> u32 {
        if let Some(&a) = self.adorned.get(&(p, mask.clone())) {
            return a;
        }
        let a = self.preds.len() as u32;
        self.preds.push(self.q.preds[p as usize]);
        self.names.push(format!("{}_{}", self.pname(p), mask_str(&mask)));
        self.adorned.insert((p, mask.clone()), a);
        self.work.push((p, mask));
        a
    }

    fn magic(&mut self, p: u32, mask: &[bool]) -> u32 {
        if let Some(&m) = self.magic.get(&(p, mask.to_vec())) {
            return m;
        }
        let m = self.preds.len() as u32;
        self.preds.push(mask.iter().filter(|b| **b).count() as u32);
        self.names.push(format!("magic_{}_{}", self.pname(p), mask_str(mask)));
        self.magic.insert((p, mask.to_vec()), m);
        m
    }

    /// Rewrites one body. `magic_head` is the magic literal that guards it.
    fn body(&mut self, head: &[Term], body: &[Literal], vars: u32, magic_head: Option<Atom>, mut bound: Vec<bool>) -> Option<Vec<Literal>> {
        let order = plan::order(head, body, &mut bound.clone(), None).ok()?;
        let mut out: Vec<Literal> = magic_head.into_iter().map(Literal::Pos).collect();
        for i in order {
            let new = match &body[i] {
                Literal::Pos(Atom { pred: Pred::Local(p), args }) => {
                    let mask: Vec<bool> = args.iter().map(|t| plan::is_bound(t, &bound)).collect();
                    let ap = self.adorned(*p, mask.clone());
                    if mask.iter().any(|b| *b) {
                        let mp = self.magic(*p, &mask);
                        let margs: Vec<Term> = args.iter().zip(&mask).filter(|(_, b)| **b).map(|(t, _)| t.clone()).collect();
                        let guard = Atom { pred: Pred::Local(mp), args: margs.clone() };
                        // Negations are dropped from magic rules: the magic set
                        // only gets larger, which is sound, and stratification
                        // is preserved.
                        let mbody: Vec<Literal> = out.iter().filter(|l| !matches!(l, Literal::Neg(_))).cloned().collect();
                        if mbody != [Literal::Pos(guard)] {
                            self.rules.push(Rule { head: mp, args: margs, body: mbody, vars });
                        }
                    }
                    Literal::Pos(Atom { pred: Pred::Local(ap), args: args.clone() })
                }
                Literal::Neg(Atom { pred: Pred::Local(p), args }) => {
                    let ap = self.adorned(*p, vec![false; args.len()]);
                    Literal::Neg(Atom { pred: Pred::Local(ap), args: args.clone() })
                }
                l => l.clone(),
            };
            plan::bind(&new, &mut bound);
            out.push(new);
        }
        Some(out)
    }
}

/// The rewritten query, with names for its predicates; `None` when the
/// rewrite does not apply (no local predicate is called with a bound
/// argument) or would not be stratified.
pub fn rewrite(q: &Query, names: &Names) -> Option<(Query, Vec<String>)> {
    let mut w = Rewriter {
        q,
        preds: Vec::new(),
        adorned: HashMap::new(),
        magic: HashMap::new(),
        work: Vec::new(),
        rules: Vec::new(),
        names: Vec::new(),
        old_names: &names.preds,
    };
    let out_terms: Vec<Term> = q
        .select
        .outputs
        .iter()
        .map(|o| match o {
            Output::Var(v) | Output::Count(v) => Term::Var(*v),
        })
        .collect();
    let sel_body = w.body(&out_terms, &q.select.body, q.select.vars, None, vec![false; q.select.vars as usize])?;
    while let Some((p, mask)) = w.work.pop() {
        let ap = w.adorned[&(p, mask.clone())];
        for r in q.rules_of(p) {
            let mut bound = vec![false; r.vars as usize];
            for (t, b) in r.args.iter().zip(&mask) {
                if let (Term::Var(v), true) = (t, b) {
                    bound[*v as usize] = true;
                }
            }
            let guard = mask.iter().any(|b| *b).then(|| Atom {
                pred: Pred::Local(w.magic(p, &mask)),
                args: r.args.iter().zip(&mask).filter(|(_, b)| **b).map(|(t, _)| t.clone()).collect(),
            });
            let body = w.body(&r.args, &r.body, r.vars, guard, bound)?;
            w.rules.push(Rule { head: ap, args: r.args.clone(), body, vars: r.vars });
        }
    }
    if w.magic.is_empty() {
        return None;
    }
    let out = Query {
        params: q.params.clone(),
        preds: w.preds,
        rules: w.rules,
        select: Select { outputs: q.select.outputs.clone(), body: sel_body, vars: q.select.vars },
    };
    stratify(&out).ok()?;
    Some((out, w.names))
}
