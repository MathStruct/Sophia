//! Static checks (SPEC-007 §3.3, §4.2): arity, sorts, safety (every variable
//! bound by a positive literal) and stratification (no recursion through
//! negation). A checked query has a type, `Query{S, A, E}`: the base schema,
//! the output sorts, and the effect `read`. See `check.md`.

use sophia_hash::component::strongly_connected;

use crate::Error;
use crate::ir::*;
use crate::plan;

/// What the checker learnt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// The sort of each output column: the result type `A`.
    pub columns: Vec<Sort>,
    /// The sort of each local predicate's columns.
    pub pred_sorts: Vec<Vec<Sort>>,
    /// Local predicates in strata, dependency first; each stratum is one
    /// strongly connected component.
    pub strata: Vec<Vec<u32>>,
}

/// Union-find over sort variables.
struct Sorts {
    parent: Vec<usize>,
    fixed: Vec<Option<Sort>>,
    what: Vec<String>,
}

impl Sorts {
    fn node(&mut self, what: String, s: Option<Sort>) -> usize {
        self.parent.push(self.parent.len());
        self.fixed.push(s);
        self.what.push(what);
        self.parent.len() - 1
    }

    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r {
            r = self.parent[r];
        }
        let mut y = x;
        while self.parent[y] != r {
            let n = self.parent[y];
            self.parent[y] = r;
            y = n;
        }
        r
    }

    fn unify(&mut self, a: usize, b: usize) -> Result<(), Error> {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return Ok(());
        }
        match (self.fixed[ra], self.fixed[rb]) {
            (Some(x), Some(y)) if x != y => Err(Error::Check(format!(
                "{} is {} but {} is {}",
                self.what[a],
                x.as_str(),
                self.what[b],
                y.as_str()
            ))),
            (x, y) => {
                self.parent[ra] = rb;
                self.fixed[rb] = y.or(x);
                Ok(())
            }
        }
    }

    fn sort(&mut self, x: usize) -> Option<Sort> {
        let r = self.find(x);
        self.fixed[r]
    }
}

pub fn check(q: &Query, names: &Names) -> Result<Checked, Error> {
    let pname = |p: u32| names.preds.get(p as usize).cloned().unwrap_or_else(|| format!("p{p}"));
    let mut s = Sorts { parent: Vec::new(), fixed: Vec::new(), what: Vec::new() };
    let params: Vec<usize> =
        q.params.iter().enumerate().map(|(i, &so)| s.node(format!("parameter {}", i + 1), Some(so))).collect();
    let cols: Vec<Vec<usize>> = q
        .preds
        .iter()
        .enumerate()
        .map(|(p, &a)| (0..a).map(|c| s.node(format!("column {} of `{}`", c + 1, pname(p as u32)), None)).collect())
        .collect();

    let term_node = |s: &mut Sorts, vars: &[usize], t: &Term| -> Result<usize, Error> {
        Ok(match t {
            Term::Var(v) => *vars.get(*v as usize).ok_or_else(|| Error::Check("variable out of range".into()))?,
            Term::Param(p) => *params.get(*p as usize).ok_or_else(|| Error::Check("parameter out of range".into()))?,
            Term::Const(c) => s.node(format!("constant {c}"), Some(c.sort())),
        })
    };

    let body = |s: &mut Sorts, vars: &[usize], body: &[Literal]| -> Result<(), Error> {
        for l in body {
            match l {
                Literal::Pos(a) | Literal::Neg(a) => {
                    let want: Vec<usize> = match a.pred {
                        Pred::Base(b) => b
                            .sorts()
                            .iter()
                            .enumerate()
                            .map(|(c, &so)| s.node(format!("column {} of `{}`", c + 1, b.name()), Some(so)))
                            .collect(),
                        Pred::Local(p) => {
                            cols.get(p as usize).cloned().ok_or_else(|| Error::Check("predicate out of range".into()))?
                        }
                    };
                    if want.len() != a.args.len() {
                        return Err(Error::Check(format!("wrong number of arguments in a literal on {:?}", a.pred)));
                    }
                    for (t, w) in a.args.iter().zip(want) {
                        let n = term_node(s, vars, t)?;
                        s.unify(n, w)?;
                    }
                }
                Literal::Eq(x, y) | Literal::Ne(x, y) => {
                    let (a, b) = (term_node(s, vars, x)?, term_node(s, vars, y)?);
                    s.unify(a, b)?;
                }
                Literal::StartsWith(x, y) => {
                    for t in [x, y] {
                        let a = term_node(s, vars, t)?;
                        let text = s.node("an argument of startswith".into(), Some(Sort::Text));
                        s.unify(a, text)?;
                    }
                }
            }
        }
        Ok(())
    };

    for (i, r) in q.rules.iter().enumerate() {
        let vn = names.rule_vars.get(i);
        let vars: Vec<usize> = (0..r.vars)
            .map(|v| {
                let n = vn.and_then(|n| n.get(v as usize)).cloned().unwrap_or_else(|| format!("x{v}"));
                s.node(format!("`{n}` in a rule of `{}`", pname(r.head)), None)
            })
            .collect();
        let head = cols.get(r.head as usize).cloned().ok_or_else(|| Error::Check("rule head out of range".into()))?;
        if head.len() != r.args.len() {
            return Err(Error::Check(format!("a rule of `{}` has the wrong number of arguments", pname(r.head))));
        }
        for (t, c) in r.args.iter().zip(head) {
            let n = term_node(&mut s, &vars, t)?;
            s.unify(n, c)?;
        }
        body(&mut s, &vars, &r.body)?;
        plan::check_safe(&r.args, &r.body, r.vars, vn.map(Vec::as_slice).unwrap_or_default())
            .map_err(|e| Error::Check(format!("in a rule of `{}`: {e}", pname(r.head))))?;
    }
    let svars: Vec<usize> = (0..q.select.vars)
        .map(|v| {
            let n = names.select_vars.get(v as usize).cloned().unwrap_or_else(|| format!("x{v}"));
            s.node(format!("`{n}` in the select"), None)
        })
        .collect();
    body(&mut s, &svars, &q.select.body)?;
    let out_terms: Vec<Term> = q
        .select
        .outputs
        .iter()
        .map(|o| match o {
            Output::Var(v) | Output::Count(v) => Term::Var(*v),
        })
        .collect();
    plan::check_safe(&out_terms, &q.select.body, q.select.vars, &names.select_vars).map_err(|e| Error::Check(format!("in the select: {e}")))?;

    let mut columns = Vec::new();
    for o in &q.select.outputs {
        columns.push(match o {
            Output::Var(v) => s.sort(svars[*v as usize]).ok_or_else(|| Error::Check(format!("cannot infer the sort of output {v}")))?,
            Output::Count(_) => Sort::Int,
        });
    }
    let mut pred_sorts = Vec::new();
    for (p, cs) in cols.iter().enumerate() {
        let mut v = Vec::new();
        for (c, &n) in cs.iter().enumerate() {
            v.push(s.sort(n).ok_or_else(|| {
                Error::Check(format!("cannot infer the sort of column {} of `{}`", c + 1, pname(p as u32)))
            })?);
        }
        pred_sorts.push(v);
    }

    let strata = stratify(q).map_err(|p| Error::Check(format!("`{}` depends on itself through negation", pname(p))))?;
    Ok(Checked { columns, pred_sorts, strata })
}

/// Strongly connected components of the predicate graph, dependency first.
/// Fails with a predicate that is recursive through negation.
pub fn stratify(q: &Query) -> Result<Vec<Vec<u32>>, u32> {
    let edges = q.pred_edges();
    let sccs = strongly_connected(q.preds.len(), |p| edges[p].iter().map(|(t, _)| *t as usize).collect::<Vec<_>>());
    for scc in &sccs {
        for &p in scc {
            if edges[p].iter().any(|(t, neg)| *neg && scc.contains(&(*t as usize))) {
                return Err(p as u32);
            }
        }
    }
    Ok(sccs.into_iter().map(|c| c.into_iter().map(|p| p as u32).collect()).collect())
}
