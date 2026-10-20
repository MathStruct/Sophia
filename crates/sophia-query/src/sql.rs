//! Translation of a checked query into SQL (SPEC-007 §7): one recursive
//! common table expression per local predicate, over SQLite's base tables.
//! SQL is a translation target here, not the definition of the query: what
//! SQL cannot express (mutual or non-linear recursion) is refused with
//! [`Error::Unsupported`], and the native executor still runs it. See
//! `sql.md`.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::Error;
use crate::ir::*;

fn lit(v: &Val) -> String {
    match v {
        Val::Id(d) => format!("X'{}'", d.to_hex()),
        Val::Text(s) => format!("'{}'", s.replace('\'', "''")),
        Val::Int(i) => i.to_string(),
    }
}

/// A relation's SQL source and column names.
fn source(p: Pred) -> (String, Vec<String>) {
    let cols = |cs: &[&str]| cs.iter().map(|s| s.to_string()).collect();
    match p {
        Pred::Base(Base::Def) => ("t_definition".into(), cols(&["id", "kind"])),
        Pred::Base(Base::Refs) => ("t_ref".into(), cols(&["src", "dst"])),
        Pred::Base(Base::Name) => ("t_name".into(), cols(&["path", "target"])),
        Pred::Base(Base::Member) => (
            "(SELECT nominal AS c0, member AS c1 FROM t_relation WHERE removed_seq IS NULL)".into(),
            cols(&["c0", "c1"]),
        ),
        Pred::Base(Base::Nominal) => ("t_nominal".into(), cols(&["id", "kind"])),
        Pred::Base(Base::Uses) => ("t_nominal_ref".into(), cols(&["src", "dst"])),
        Pred::Base(Base::Annotation) => (
            "(SELECT a.id AS c0, a.kind AS c1, s.subject AS c2 FROM t_annotation a JOIN t_annotation_subject s ON s.id = a.id)"
                .into(),
            cols(&["c0", "c1", "c2"]),
        ),
        Pred::Base(Base::Fact) => ("t_fact".into(), cols(&["id", "kind", "subject"])),
        Pred::Local(i) => (format!("p{i}"), Vec::new()),
    }
}

struct Body<'a> {
    params: &'a [Val],
    arity: &'a [u32],
    from: Vec<String>,
    wh: Vec<String>,
    var: HashMap<u32, String>,
}

impl Body<'_> {
    fn cols(&self, p: Pred) -> (String, Vec<String>) {
        match p {
            Pred::Local(i) => (format!("p{i}"), (0..self.arity[i as usize]).map(|c| format!("c{c}")).collect()),
            b => source(b),
        }
    }

    fn term(&self, t: &Term) -> Option<String> {
        match t {
            Term::Var(v) => self.var.get(v).cloned(),
            Term::Param(p) => self.params.get(*p as usize).map(lit),
            Term::Const(c) => Some(lit(c)),
        }
    }

    fn build(mut self, head: &[Term], body: &[Literal], distinct: bool, group: Option<&[Output]>) -> Result<String, Error> {
        for (i, l) in body.iter().enumerate() {
            if let Literal::Pos(a) = l {
                let (src, cols) = self.cols(a.pred);
                let alias = format!("t{i}");
                self.from.push(format!("{src} AS {alias}"));
                for (t, c) in a.args.iter().zip(cols) {
                    let col = format!("{alias}.{c}");
                    match t {
                        Term::Var(v) if !self.var.contains_key(v) => {
                            self.var.insert(*v, col);
                        }
                        t => {
                            let e = self.term(t).unwrap();
                            self.wh.push(format!("{col} = {e}"));
                        }
                    }
                }
            }
        }
        // Equations bind what positive literals did not.
        let mut eqs: Vec<(&Term, &Term)> = body.iter().filter_map(|l| if let Literal::Eq(x, y) = l { Some((x, y)) } else { None }).collect();
        while !eqs.is_empty() {
            let before = eqs.len();
            eqs.retain(|(x, y)| match (self.term(x), self.term(y)) {
                (Some(a), Some(b)) => {
                    self.wh.push(format!("{a} = {b}"));
                    false
                }
                (Some(a), None) | (None, Some(a)) => {
                    let Term::Var(v) = (if self.term(x).is_none() { x } else { y }) else { unreachable!() };
                    self.var.insert(*v, a);
                    false
                }
                _ => true,
            });
            if eqs.len() == before {
                return Err(Error::Check("an equation between two unbound variables".into()));
            }
        }
        for (i, l) in body.iter().enumerate() {
            match l {
                Literal::Ne(x, y) => self.wh.push(format!("{} <> {}", self.term(x).unwrap(), self.term(y).unwrap())),
                Literal::StartsWith(x, y) => {
                    let (a, b) = (self.term(x).unwrap(), self.term(y).unwrap());
                    self.wh.push(format!("substr({a}, 1, length({b})) = {b}"));
                }
                Literal::Neg(a) => {
                    let (src, cols) = self.cols(a.pred);
                    let alias = format!("n{i}");
                    let mut conds = Vec::new();
                    let mut local: HashMap<u32, String> = HashMap::new();
                    for (t, c) in a.args.iter().zip(cols) {
                        let col = format!("{alias}.{c}");
                        match (t, self.term(t)) {
                            (_, Some(e)) => conds.push(format!("{col} = {e}")),
                            (Term::Var(v), None) => match local.get(v) {
                                Some(prev) => conds.push(format!("{col} = {prev}")),
                                None => {
                                    local.insert(*v, col);
                                }
                            },
                            _ => unreachable!(),
                        }
                    }
                    let wh = if conds.is_empty() { String::new() } else { format!(" WHERE {}", conds.join(" AND ")) };
                    self.wh.push(format!("NOT EXISTS (SELECT 1 FROM {src} AS {alias}{wh})"));
                }
                _ => {}
            }
        }
        let mut sel = Vec::new();
        let mut groups = Vec::new();
        for (k, t) in head.iter().enumerate() {
            let e = self.term(t).ok_or_else(|| Error::Check("an unbound output".into()))?;
            match group.map(|g| g[k]) {
                Some(Output::Count(_)) => sel.push(format!("count(DISTINCT {e}) AS c{k}")),
                Some(Output::Var(_)) => {
                    groups.push(e.clone());
                    sel.push(format!("{e} AS c{k}"))
                }
                None => sel.push(format!("{e} AS c{k}")),
            }
        }
        let mut s = format!("SELECT {}{}", if distinct { "DISTINCT " } else { "" }, sel.join(", "));
        if !self.from.is_empty() {
            write!(s, " FROM {}", self.from.join(", ")).unwrap();
        }
        if !self.wh.is_empty() {
            write!(s, " WHERE {}", self.wh.join(" AND ")).unwrap();
        }
        if group.is_some_and(|g| g.iter().any(|o| matches!(o, Output::Count(_)))) && !groups.is_empty() {
            write!(s, " GROUP BY {}", groups.join(", ")).unwrap();
        }
        Ok(s)
    }
}

/// The SQL for a checked query (or its magic rewrite) with parameter values
/// inlined as literals. `strata` comes from [`crate::check::stratify`].
pub fn to_sql(q: &Query, strata: &[Vec<u32>], params: &[Val]) -> Result<String, Error> {
    let body = || Body { params, arity: &q.preds, from: Vec::new(), wh: Vec::new(), var: HashMap::new() };
    let mut ctes = Vec::new();
    for stratum in strata {
        let [p] = stratum[..] else {
            return Err(Error::Unsupported("mutual recursion between local predicates has no SQL translation".into()));
        };
        let mentions = |r: &Rule| {
            r.body.iter().filter(|l| matches!(l, Literal::Pos(Atom { pred: Pred::Local(x), .. }) if *x == p)).count()
        };
        let mut base = Vec::new();
        let mut rec = Vec::new();
        for r in q.rules_of(p) {
            let s = body().build(&r.args, &r.body, false, None)?;
            match mentions(r) {
                0 => base.push(s),
                1 => rec.push(s),
                _ => return Err(Error::Unsupported("non-linear recursion has no SQL translation in SQLite".into())),
            }
        }
        let arity = q.preds[p as usize];
        if base.is_empty() {
            let nulls: Vec<String> = (0..arity).map(|c| format!("NULL AS c{c}")).collect();
            base.push(format!("SELECT {} WHERE 0", nulls.join(", ")));
        }
        let cols: Vec<String> = (0..arity).map(|c| format!("c{c}")).collect();
        let parts: Vec<String> = base.into_iter().chain(rec).collect();
        ctes.push(format!("p{p}({}) AS ({})", cols.join(", "), parts.join(" UNION ")));
    }
    let s = &q.select;
    let heads: Vec<Term> = s.outputs.iter().map(|o| match o { Output::Var(v) | Output::Count(v) => Term::Var(*v) }).collect();
    let has_count = s.outputs.iter().any(|o| matches!(o, Output::Count(_)));
    let main = body().build(&heads, &s.body, !has_count, Some(&s.outputs))?;
    Ok(if ctes.is_empty() { main } else { format!("WITH RECURSIVE {} {main}", ctes.join(", ")) })
}
