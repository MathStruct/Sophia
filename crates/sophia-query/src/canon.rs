//! The canonical form of a query, its bytes, and its identities
//! (SPEC-007 §4): the query id, and the result id that adds the snapshot and
//! the parameters. A query is stored like any definition: a record of the
//! language `sophia-query`. See `canon.md`.

use std::collections::HashMap;

use sophia_hash::canonical::{Reader, Writer};
use sophia_hash::merkle::{component_digest, member_digest};
use sophia_hash::{Digest, NominalId, Record, Status};

use crate::Error;
use crate::ir::*;

pub const LANG: &str = "sophia-query";
pub const NORMALISER: &str = "sophia-query@0";
pub const KIND: &str = "Query";


fn term(w: &mut Writer, t: &Term) {
    match t {
        Term::Var(v) => w.u32("v", *v),
        Term::Param(p) => w.u32("p", *p),
        Term::Const(Val::Id(d)) => w.field("id", d.as_bytes()),
        Term::Const(Val::Text(s)) => w.str("s", s),
        Term::Const(Val::Int(i)) => w.u64("i", *i as u64),
    }
}

fn atom(w: &mut Writer, a: &Atom) {
    match a.pred {
        Pred::Base(b) => w.str("base", b.name()),
        Pred::Local(p) => w.u32("local", p),
    }
    for t in &a.args {
        term(w, t);
    }
}

fn literal(w: &mut Writer, l: &Literal) {
    let tag = match l {
        Literal::Pos(_) => "pos",
        Literal::Neg(_) => "neg",
        Literal::Eq(..) => "eq",
        Literal::Ne(..) => "ne",
        Literal::StartsWith(..) => "startswith",
    };
    let o = w.open(tag);
    match l {
        Literal::Pos(a) | Literal::Neg(a) => atom(w, a),
        Literal::Eq(x, y) | Literal::Ne(x, y) | Literal::StartsWith(x, y) => {
            term(w, x);
            term(w, y)
        }
    }
    w.close(o);
}

fn rule_bytes(r: &Rule) -> Vec<u8> {
    let mut w = Writer::new();
    w.u32("vars", r.vars);
    let o = w.open("head");
    for t in &r.args {
        term(&mut w, t);
    }
    w.close(o);
    for l in &r.body {
        literal(&mut w, l);
    }
    w.finish()
}

/// The canonical bytes of a query that is already canonical.
pub fn encode(q: &Query) -> Vec<u8> {
    let mut w = Writer::new();
    w.str("sophia", "query-v0");
    w.str("schema", Base::SCHEMA);
    for s in &q.params {
        w.u32("param", s.code());
    }
    for (p, arity) in q.preds.iter().enumerate() {
        let o = w.open("pred");
        w.u32("arity", *arity);
        for r in q.rules_of(p as u32) {
            w.field("rule", &rule_bytes(r));
        }
        w.close(o);
    }
    let o = w.open("select");
    w.u32("vars", q.select.vars);
    for out in &q.select.outputs {
        match out {
            Output::Var(v) => w.u32("out", *v),
            Output::Count(v) => w.u32("count", *v),
        }
    }
    for l in &q.select.body {
        literal(&mut w, l);
    }
    w.close(o);
    w.finish()
}

fn rd(e: sophia_hash::canonical::ReadError) -> Error {
    Error::Codec(e.to_string())
}

fn read_term(tag: &str, p: &[u8]) -> Result<Term, Error> {
    let u32_of = |p: &[u8]| p.try_into().map(u32::from_le_bytes).map_err(|_| Error::Codec("u32".into()));
    Ok(match tag {
        "v" => Term::Var(u32_of(p)?),
        "p" => Term::Param(u32_of(p)?),
        "id" => Term::Const(Val::Id(Digest::from_slice(p).ok_or_else(|| Error::Codec("id".into()))?)),
        "s" => Term::Const(Val::Text(String::from_utf8(p.to_vec()).map_err(|_| Error::Codec("text".into()))?)),
        "i" => Term::Const(Val::Int(u64::from_le_bytes(p.try_into().map_err(|_| Error::Codec("int".into()))?) as i64)),
        t => return Err(Error::Codec(format!("unknown term tag {t}"))),
    })
}

fn read_terms(r: &mut Reader) -> Result<Vec<Term>, Error> {
    let mut out = Vec::new();
    while !r.is_empty() {
        let (t, p) = r.next().map_err(rd)?;
        out.push(read_term(t, p)?);
    }
    Ok(out)
}

fn read_literal(tag: &str, p: &[u8]) -> Result<Literal, Error> {
    let mut r = Reader::new(p);
    let two = |r: &mut Reader| -> Result<(Term, Term), Error> {
        let ts = read_terms(r)?;
        let [a, b]: [Term; 2] = ts.try_into().map_err(|_| Error::Codec("two terms".into()))?;
        Ok((a, b))
    };
    let atom = |r: &mut Reader| -> Result<Atom, Error> {
        let (t, p) = r.next().map_err(rd)?;
        let pred = match t {
            "base" => Pred::Base(
                std::str::from_utf8(p).ok().and_then(Base::parse).ok_or_else(|| Error::Codec("base relation".into()))?,
            ),
            "local" => Pred::Local(p.try_into().map(u32::from_le_bytes).map_err(|_| Error::Codec("u32".into()))?),
            _ => return Err(Error::Codec("predicate".into())),
        };
        Ok(Atom { pred, args: read_terms(r)? })
    };
    Ok(match tag {
        "pos" => Literal::Pos(atom(&mut r)?),
        "neg" => Literal::Neg(atom(&mut r)?),
        "eq" => {
            let (a, b) = two(&mut r)?;
            Literal::Eq(a, b)
        }
        "ne" => {
            let (a, b) = two(&mut r)?;
            Literal::Ne(a, b)
        }
        "startswith" => {
            let (a, b) = two(&mut r)?;
            Literal::StartsWith(a, b)
        }
        t => return Err(Error::Codec(format!("unknown literal tag {t}"))),
    })
}

/// Reads canonical bytes back into a query.
pub fn decode(bytes: &[u8]) -> Result<Query, Error> {
    let mut r = Reader::new(bytes);
    if r.expect_str("sophia").map_err(rd)? != "query-v0" {
        return Err(Error::Codec("not a query-v0".into()));
    }
    if r.expect_str("schema").map_err(rd)? != Base::SCHEMA {
        return Err(Error::Codec("unknown base schema".into()));
    }
    let mut q = Query { params: Vec::new(), preds: Vec::new(), rules: Vec::new(), select: Select { outputs: Vec::new(), body: Vec::new(), vars: 0 } };
    while r.peek_tag().map_err(rd)? == "param" {
        q.params.push(Sort::from_code(r.expect_u32("param").map_err(rd)?).ok_or_else(|| Error::Codec("sort".into()))?);
    }
    while r.peek_tag().map_err(rd)? == "pred" {
        let mut p = r.enter("pred").map_err(rd)?;
        let head = q.preds.len() as u32;
        q.preds.push(p.expect_u32("arity").map_err(rd)?);
        while !p.is_empty() {
            let mut rr = Reader::new(p.expect("rule").map_err(rd)?);
            let vars = rr.expect_u32("vars").map_err(rd)?;
            let args = read_terms(&mut rr.enter("head").map_err(rd)?)?;
            let mut body = Vec::new();
            while !rr.is_empty() {
                let (t, p) = rr.next().map_err(rd)?;
                body.push(read_literal(t, p)?);
            }
            q.rules.push(Rule { head, args, body, vars });
        }
    }
    let mut s = r.enter("select").map_err(rd)?;
    q.select.vars = s.expect_u32("vars").map_err(rd)?;
    while !s.is_empty() {
        let (t, p) = s.next().map_err(rd)?;
        match t {
            "out" | "count" => {
                let v = p.try_into().map(u32::from_le_bytes).map_err(|_| Error::Codec("u32".into()))?;
                q.select.outputs.push(if t == "out" { Output::Var(v) } else { Output::Count(v) });
            }
            t => q.select.body.push(read_literal(t, p)?),
        }
    }
    if !r.is_empty() {
        return Err(Error::Codec("trailing bytes".into()));
    }
    Ok(q)
}

/// Renumbers a rule's variables by first occurrence (head, then body), and
/// returns the old-to-new map.
fn renumber_vars(args: &mut [Term], body: &mut [Literal], fixed: &[u32]) -> (u32, Vec<u32>) {
    let mut map: HashMap<u32, u32> = HashMap::new();
    for &v in fixed {
        let n = map.len() as u32;
        map.entry(v).or_insert(n);
    }
    let visit = |t: &mut Term, map: &mut HashMap<u32, u32>| {
        if let Term::Var(v) = t {
            let n = map.len() as u32;
            *v = *map.entry(*v).or_insert(n);
        }
    };
    for t in args.iter_mut() {
        visit(t, &mut map);
    }
    for l in body.iter_mut() {
        match l {
            Literal::Pos(a) | Literal::Neg(a) => a.args.iter_mut().for_each(|t| visit(t, &mut map)),
            Literal::Eq(x, y) | Literal::Ne(x, y) | Literal::StartsWith(x, y) => {
                visit(x, &mut map);
                visit(y, &mut map)
            }
        }
    }
    let mut old_of_new = vec![0; map.len()];
    for (o, n) in map {
        old_of_new[n as usize] = o;
    }
    (old_of_new.len() as u32, old_of_new)
}

fn map_preds(body: &mut [Literal], new_of_old: &[u32]) {
    for l in body {
        if let Literal::Pos(a) | Literal::Neg(a) = l {
            if let Pred::Local(p) = &mut a.pred {
                *p = new_of_old[*p as usize];
            }
        }
    }
}

/// Local predicates in the order a body mentions them.
fn mentioned(body: &[Literal]) -> impl Iterator<Item = u32> + '_ {
    body.iter().filter_map(|l| match l {
        Literal::Pos(Atom { pred: Pred::Local(p), .. }) | Literal::Neg(Atom { pred: Pred::Local(p), .. }) => Some(*p),
        _ => None,
    })
}

/// How many complete traversals canonicalisation may try before it refuses
/// the query as too symmetric (SPEC-007 §4.1).
pub const CANON_BUDGET: usize = 4096;

/// The search for the canonical form: the smallest encoding over every
/// valid traversal.
struct Search<'a> {
    q: &'a Query,
    rules_of: Vec<Vec<usize>>,
    leaves: usize,
    best: Option<(Vec<u8>, Vec<u32>, Vec<usize>)>,
}

impl Search<'_> {
    fn masked(&self, r: usize, num: &[u32]) -> Vec<u8> {
        let mut rule = self.q.rules[r].clone();
        map_preds(&mut rule.body, num);
        rule_bytes(&rule)
    }

    /// Every order of `rs` consistent with sorting by masked key: tied
    /// rules (which mention predicates not numbered yet) in every order.
    fn orderings(&self, rs: &[usize], num: &[u32]) -> Result<Vec<Vec<usize>>, Error> {
        let mut keyed: Vec<(Vec<u8>, usize)> = rs.iter().map(|&r| (self.masked(r, num), r)).collect();
        keyed.sort();
        let mut groups: Vec<Vec<usize>> = Vec::new();
        for (i, (k, r)) in keyed.iter().enumerate() {
            if i > 0 && keyed[i - 1].0 == *k {
                groups.last_mut().unwrap().push(*r);
            } else {
                groups.push(vec![*r]);
            }
        }
        let mut out: Vec<Vec<usize>> = vec![Vec::new()];
        for g in groups {
            let perms = permutations(&g);
            let mut next = Vec::with_capacity(out.len() * perms.len());
            for prefix in &out {
                for p in &perms {
                    let mut v = prefix.clone();
                    v.extend_from_slice(p);
                    next.push(v);
                }
            }
            if next.len() > CANON_BUDGET {
                return Err(Error::Check("the query is too symmetric to canonicalise within its budget".into()));
            }
            out = next;
        }
        Ok(out)
    }

    fn go(&mut self, num: Vec<u32>, order: Vec<u32>, seq: Vec<usize>, i: usize) -> Result<(), Error> {
        if i == order.len() {
            self.leaves += 1;
            if self.leaves > CANON_BUDGET {
                return Err(Error::Check("the query is too symmetric to canonicalise within its budget".into()));
            }
            let bytes = encode(&self.build(&num, &order, &seq).0);
            if self.best.as_ref().is_none_or(|(b, _, _)| bytes < *b) {
                self.best = Some((bytes, order, seq));
            }
            return Ok(());
        }
        let p = order[i];
        let rs = self.rules_of[p as usize].clone();
        for ordering in self.orderings(&rs, &num)? {
            let (mut num, mut order, mut seq) = (num.clone(), order.clone(), seq.clone());
            for &r in &ordering {
                for m in mentioned(&self.q.rules[r].body) {
                    if num[m as usize] == u32::MAX {
                        num[m as usize] = order.len() as u32;
                        order.push(m);
                    }
                }
                seq.push(r);
            }
            self.go(num, order, seq, i + 1)?;
        }
        Ok(())
    }

    /// The query for one traversal, and the old index of each new rule.
    fn build(&self, num: &[u32], order: &[u32], seq: &[usize]) -> (Query, Vec<usize>) {
        let rules = seq
            .iter()
            .map(|&r| {
                let mut rule = self.q.rules[r].clone();
                rule.head = num[rule.head as usize];
                map_preds(&mut rule.body, num);
                rule
            })
            .collect();
        let mut select = self.q.select.clone();
        map_preds(&mut select.body, num);
        (Query { params: self.q.params.clone(), preds: order.iter().map(|&o| self.q.preds[o as usize]).collect(), rules, select }, seq.to_vec())
    }
}

fn permutations(g: &[usize]) -> Vec<Vec<usize>> {
    if g.len() <= 1 {
        return vec![g.to_vec()];
    }
    let mut out = Vec::new();
    for i in 0..g.len() {
        let mut rest = g.to_vec();
        let x = rest.remove(i);
        for mut p in permutations(&rest) {
            p.insert(0, x);
            out.push(p);
            if out.len() > CANON_BUDGET {
                return out;
            }
        }
    }
    out
}

/// Rewrites a query (and its names) into canonical form (SPEC-007 §4.1).
///
/// The form is **defined** as the smallest encoding over every valid
/// traversal, where a traversal:
/// - numbers variables by first occurrence (outputs, head, then body);
/// - numbers predicates by first mention, starting from the select's body
///   in literal order;
/// - visits predicates in that order, and lists each one's rules sorted by
///   their bytes with not-yet-numbered predicates masked; rules tied under
///   the mask may come in any order, and every order is tried.
///
/// Exact duplicate rules and unreachable predicates are dropped first. So
/// renaming anything, permuting predicates or rules, duplicating rules and
/// adding unreachable predicates never change the result. The search is
/// exhaustive within [`CANON_BUDGET`] traversals; a query more symmetric
/// than that is refused, never given an unstable id. Literal order inside a
/// body is kept: it is part of the identity, like argument order in code.
pub fn canonicalise(q: &Query, names: &Names) -> Result<(Query, Names), Error> {
    let mut q = q.clone();
    let mut names = names.clone();
    names.rule_vars.resize(q.rules.len(), Vec::new());

    // Variables.
    {
        let fixed: Vec<u32> = q
            .select
            .outputs
            .iter()
            .map(|o| match o {
                Output::Var(v) | Output::Count(v) => *v,
            })
            .collect();
        let (n, old) = renumber_vars(&mut [], &mut q.select.body, &fixed);
        let mut outs = q.select.outputs.clone();
        for o in &mut outs {
            match o {
                Output::Var(v) | Output::Count(v) => *v = old.iter().position(|x| x == v).unwrap() as u32,
            }
        }
        q.select.outputs = outs;
        q.select.vars = n;
        names.select_vars = old.iter().map(|&o| names.select_vars.get(o as usize).cloned().unwrap_or_default()).collect();
        for (i, r) in q.rules.iter_mut().enumerate() {
            let (n, old) = renumber_vars(&mut r.args, &mut r.body, &[]);
            r.vars = n;
            names.rule_vars[i] =
                old.iter().map(|&o| names.rule_vars[i].get(o as usize).cloned().unwrap_or_default()).collect();
        }
    }

    // Exact duplicates go (the first one's names stay).
    let mut keep: Vec<usize> = Vec::new();
    for i in 0..q.rules.len() {
        if !keep.iter().any(|&k| q.rules[k] == q.rules[i]) {
            keep.push(i);
        }
    }
    let mut rules_of = vec![Vec::new(); q.preds.len()];
    for &i in &keep {
        rules_of[q.rules[i].head as usize].push(i);
    }

    let mut num = vec![u32::MAX; q.preds.len()];
    let mut order = Vec::new();
    for m in mentioned(&q.select.body) {
        if num[m as usize] == u32::MAX {
            num[m as usize] = order.len() as u32;
            order.push(m);
        }
    }
    let mut search = Search { q: &q, rules_of, leaves: 0, best: None };
    search.go(num.clone(), order.clone(), Vec::new(), 0)?;
    let (_, order, seq) = search.best.take().unwrap();
    // Recompute the numbering of the winning traversal.
    let mut num = vec![u32::MAX; q.preds.len()];
    for (n, &o) in order.iter().enumerate() {
        num[o as usize] = n as u32;
    }
    let (out, old_rules) = search.build(&num, &order, &seq);
    let out_names = Names {
        query: names.query.clone(),
        params: names.params.clone(),
        preds: order.iter().map(|&o| names.preds.get(o as usize).cloned().unwrap_or_default()).collect(),
        rule_vars: old_rules.iter().map(|&r| names.rule_vars[r].clone()).collect(),
        select_vars: names.select_vars.clone(),
    };
    Ok((out, out_names))
}

/// A query's id: the member id of a one-member component of the language
/// `sophia-query`, so queries share the store's identity rule (SPEC-001 §4).
pub fn query_id(bytes: &[u8]) -> Digest {
    member_digest(&component_digest(LANG, NORMALISER, bytes), 0)
}

/// A result's id (SPEC-007 §4.3): the query, the snapshot it ran against,
/// and the parameter values. Two runs with the same result id must give the
/// same rows.
pub fn result_id(query: &Digest, snapshot: &Digest, params: &[Val]) -> Digest {
    let mut w = Writer::new();
    w.str("sophia", "query-result-v0");
    w.field("query", query.as_bytes());
    w.field("snapshot", snapshot.as_bytes());
    for p in params {
        let o = w.open("param");
        term(&mut w, &Term::Const(p.clone()));
        w.close(o);
    }
    Digest::of(&w.finish())
}

/// The id constants a query mentions.
pub fn constants(q: &Query) -> Vec<Digest> {
    let mut out = Vec::new();
    let mut add = |t: &Term| {
        if let Term::Const(Val::Id(d)) = t {
            if !out.contains(d) {
                out.push(*d);
            }
        }
    };
    let bodies = q.rules.iter().map(|r| (&r.args[..], &r.body[..])).chain(std::iter::once((&[][..], &q.select.body[..])));
    for (args, body) in bodies {
        args.iter().for_each(&mut add);
        for l in body {
            match l {
                Literal::Pos(a) | Literal::Neg(a) => a.args.iter().for_each(&mut add),
                Literal::Eq(x, y) | Literal::Ne(x, y) | Literal::StartsWith(x, y) => {
                    add(x);
                    add(y)
                }
            }
        }
    }
    out
}

/// Display data: the names, which are not hashed.
pub fn encode_names(n: &Names) -> Vec<u8> {
    let mut w = Writer::new();
    w.str("query", &n.query);
    for p in &n.params {
        w.str("param", p);
    }
    for p in &n.preds {
        w.str("pred", p);
    }
    for r in &n.rule_vars {
        let o = w.open("rule");
        for v in r {
            w.str("v", v);
        }
        w.close(o);
    }
    let o = w.open("select");
    for v in &n.select_vars {
        w.str("v", v);
    }
    w.close(o);
    w.finish()
}

pub fn decode_names(bytes: &[u8]) -> Result<Names, Error> {
    let mut r = Reader::new(bytes);
    let mut n = Names { query: r.expect_str("query").map_err(rd)?.into(), ..Names::default() };
    let strs = |r: &mut Reader| -> Result<Vec<String>, Error> {
        let mut out = Vec::new();
        while !r.is_empty() {
            out.push(r.expect_str("v").map_err(rd)?.to_string());
        }
        Ok(out)
    };
    while !r.is_empty() {
        match r.peek_tag().map_err(rd)? {
            "param" => n.params.push(r.expect_str("param").map_err(rd)?.into()),
            "pred" => n.preds.push(r.expect_str("pred").map_err(rd)?.into()),
            "rule" => n.rule_vars.push(strs(&mut r.enter("rule").map_err(rd)?)?),
            "select" => n.select_vars = strs(&mut r.enter("select").map_err(rd)?)?,
            t => return Err(Error::Codec(format!("unknown names tag {t}"))),
        }
    }
    Ok(n)
}

/// The store record of a canonical query. `refs` and `nominal_refs` are the
/// id constants the store knows as definitions and as nominal entities.
pub fn record(q: &Query, names: &Names, refs: Vec<Digest>, nominal_refs: Vec<NominalId>) -> Record {
    let bytes = encode(q);
    let component = component_digest(LANG, NORMALISER, &bytes);
    Record {
        id: member_digest(&component, 0),
        lang: LANG.into(),
        normaliser: NORMALISER.into(),
        kind: KIND.into(),
        component,
        index: 0,
        component_size: 1,
        bytes,
        display: encode_names(names),
        status: Status::Complete,
        refs,
        nominal_refs,
    }
}
