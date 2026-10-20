//! Literal ordering (SPEC-007 §6.1): a greedy planner that runs filters as
//! soon as their variables are bound, prefers literals with an access path
//! for a bound column, and reports a variable that nothing binds. The same
//! planner orders bodies for the magic-set rewrite and for evaluation. See
//! `plan.md`.

use crate::ir::*;

/// The columns of a base relation that an access path can look up directly
/// (SPEC-007 §6). Every backend provides these, through `ReadTx`.
pub fn access_columns(b: Base) -> &'static [usize] {
    match b {
        Base::Def | Base::Refs | Base::Name => &[0, 1],
        Base::Member => &[0, 1],
        Base::Nominal | Base::Uses => &[0],
        Base::Annotation => &[0, 2],
        Base::Fact => &[2],
    }
}

/// Lookups that can return a large share of a relation: every definition
/// of a kind, every member of a collection (a library's proofs, a big
/// method table). Other id- and path-keyed lookups are narrow. Without
/// statistics this is a static guess (SPEC-007 §6.1); cost-based planning
/// replaces it.
pub fn wide(b: Base, col: usize) -> bool {
    matches!((b, col), (Base::Def, 1) | (Base::Member, 0))
}

pub(crate) fn is_bound(t: &Term, bound: &[bool]) -> bool {
    match t {
        Term::Var(v) => bound[*v as usize],
        _ => true,
    }
}

/// For each literal, the variables that occur in it and nowhere else in the
/// rule (not in the head, outputs, or any other literal). In a negated
/// literal they are existential.
pub(crate) fn local_only(head: &[Term], body: &[Literal], vars: u32) -> Vec<Vec<u32>> {
    let mut count = vec![0u32; vars as usize];
    for t in head {
        if let Term::Var(v) = t {
            count[*v as usize] += 2;
        }
    }
    for l in body {
        let mut vs = l.vars();
        vs.sort();
        vs.dedup();
        for v in vs {
            count[v as usize] += 1;
        }
    }
    body.iter()
        .map(|l| {
            let mut vs = l.vars();
            vs.sort();
            vs.dedup();
            vs.retain(|v| count[*v as usize] == 1);
            vs
        })
        .collect()
}

fn score(l: &Literal, bound: &[bool], local: &[u32]) -> Option<u32> {
    let nb = |a: &Atom| a.args.iter().filter(|t| is_bound(t, bound)).count() as u32;
    match l {
        Literal::Pos(a) => Some(match a.pred {
            Pred::Base(b) => {
                let paths: Vec<usize> = access_columns(b).iter().copied().filter(|&c| is_bound(&a.args[c], bound)).collect();
                if paths.iter().any(|&c| !wide(b, c)) {
                    60 + nb(a)
                } else if !paths.is_empty() {
                    // A wide lookup (all definitions of a kind) binds many
                    // tuples. It comes after local literals, bound or free:
                    // a free local predicate is computed once from the
                    // query's parameters, while binding it from a wide
                    // lookup makes magic sets search backwards from every
                    // match (measured on set.mm).
                    8
                } else if nb(a) > 0 {
                    5
                } else {
                    1
                }
            }
            Pred::Local(_) => {
                if nb(a) > 0 {
                    40 + nb(a)
                } else {
                    10
                }
            }
        }),
        Literal::Neg(a) => a
            .args
            .iter()
            .all(|t| is_bound(t, bound) || matches!(t, Term::Var(v) if local.contains(v)))
            .then_some(95),
        Literal::Eq(x, y) => match (is_bound(x, bound), is_bound(y, bound)) {
            (true, true) => Some(100),
            (true, false) | (false, true) => Some(90),
            _ => None,
        },
        Literal::Ne(x, y) | Literal::StartsWith(x, y) => (is_bound(x, bound) && is_bound(y, bound)).then_some(100),
    }
}

/// Marks what a literal binds once it has run.
pub(crate) fn bind(l: &Literal, bound: &mut [bool]) {
    match l {
        Literal::Pos(_) | Literal::Eq(..) => {
            for v in l.vars() {
                bound[v as usize] = true;
            }
        }
        _ => {}
    }
}

/// Orders a body. `bound` says which variables are bound on entry and is
/// updated; `first` forces one positive literal to run first (the delta in
/// semi-naive evaluation). Fails with a variable that no literal can bind.
pub fn order(
    head: &[Term],
    body: &[Literal],
    bound: &mut [bool],
    first: Option<usize>,
) -> Result<Vec<usize>, u32> {
    let local = local_only(head, body, bound.len() as u32);
    let mut done = vec![false; body.len()];
    let mut out = Vec::with_capacity(body.len());
    if let Some(f) = first {
        done[f] = true;
        out.push(f);
        bind(&body[f], bound);
    }
    while out.len() < body.len() {
        let mut best: Option<(u32, usize)> = None;
        for (i, l) in body.iter().enumerate() {
            if done[i] {
                continue;
            }
            if let Some(s) = score(l, bound, &local[i]) {
                if best.is_none_or(|(b, _)| s > b) {
                    best = Some((s, i));
                }
            }
        }
        let Some((_, i)) = best else {
            let i = (0..body.len()).find(|&i| !done[i]).unwrap();
            let v = body[i].vars().into_iter().find(|v| !bound[*v as usize]).unwrap_or(0);
            return Err(v);
        };
        done[i] = true;
        out.push(i);
        bind(&body[i], bound);
    }
    Ok(out)
}

/// Safety (range restriction): every variable of the head is bound by the
/// body, and every literal can run.
pub fn check_safe(head: &[Term], body: &[Literal], vars: u32, names: &[String]) -> Result<(), String> {
    let name = |v: u32| names.get(v as usize).filter(|n| !n.is_empty()).cloned().unwrap_or_else(|| format!("x{v}"));
    let mut bound = vec![false; vars as usize];
    order(head, body, &mut bound, None).map_err(|v| format!("`{}` is not bound by a positive literal", name(v)))?;
    for t in head {
        if let Term::Var(v) = t {
            if !bound[*v as usize] {
                return Err(format!("`{}` is not bound by a positive literal", name(*v)));
            }
        }
    }
    Ok(())
}
