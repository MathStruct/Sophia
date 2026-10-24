//! Printer: core definitions → text in a rendering profile. Every name it
//! prints is checked to resolve back to the same thing (otherwise it prints
//! a hash literal), so text from an editable profile parses back to the same
//! content (SPEC-005 §5). See `print.md`.

use std::collections::HashMap;

use sophia_hash::Digest;
use sophialang_core::{Binder, Body, Definition, Kind, Lit, Term, Usage};

use crate::lexer::{is_ident_char, is_ident_start, quote_str};
use crate::profile::{Blocks, Profile};
use crate::resolve::{Env, Found, KEYWORDS, builtin_name, infix_prec, resolve_global, spellings};

/// One definition to print, with its short name. `index` is its canonical
/// index in its recursive group (for `Term::Member`), `id` its content id.
#[derive(Debug, Clone)]
pub struct ToPrint<'a> {
    pub name: &'a str,
    pub def: &'a Definition,
    pub id: Option<Digest>,
    pub index: u32,
    /// A relation's signature prints as `relation NAME :: T` (S-ADR-0009).
    pub relation: bool,
}

pub struct Printer<'a> {
    env: &'a dyn Env,
    ns: &'a str,
    p: &'a Profile,
    group: &'a [ToPrint<'a>],
    scope: Scope,
    indent: usize,
    /// Whether each binder is used, from one pass over the item (see
    /// [`binder_uses`]): keyed by the binding node's address and the
    /// binder's slot. Saves an `occurs` walk per binder, which was
    /// quadratic in depth.
    uses: HashMap<(usize, u32), bool>,
}

/// The names in scope, innermost last, with a count per name and, per base
/// name, the smallest suffix that might be free (`x_3`): all smaller ones
/// are taken. Lookups and fresh names cost O(1) amortised instead of a scan
/// of the scope, which made deep printing cubic (SP-015). Same results as
/// the scan.
#[derive(Default)]
struct Scope {
    names: Vec<String>,
    count: HashMap<String, usize>,
    hint: std::cell::RefCell<HashMap<String, usize>>,
}

impl Scope {
    fn contains(&self, n: &str) -> bool {
        self.count.get(n).is_some_and(|c| *c > 0)
    }
    fn len(&self) -> usize {
        self.names.len()
    }
    fn push(&mut self, n: String) {
        *self.count.entry(n.clone()).or_default() += 1;
        self.names.push(n);
    }
    fn pop(&mut self) {
        let Some(n) = self.names.pop() else { return };
        if let Some(c) = self.count.get_mut(&n) {
            *c -= 1;
        }
        // `base_k` is free again: the smallest free suffix may now be k.
        if let Some((base, k)) = n.rsplit_once('_') {
            if let Ok(k) = k.parse::<usize>() {
                let mut h = self.hint.borrow_mut();
                if let Some(old) = h.get_mut(base) {
                    *old = (*old).min(k);
                }
            }
        }
    }
    fn truncate(&mut self, len: usize) {
        while self.names.len() > len {
            self.pop();
        }
    }
    fn clear(&mut self) {
        self.truncate(0);
    }
}

impl std::ops::Index<usize> for Scope {
    type Output = String;
    fn index(&self, i: usize) -> &String {
        &self.names[i]
    }
}

/// For every binder in `t`, whether its body uses it: one pass, any depth.
/// Lam, Pi and Let bind slot 0 at their own address; a match case binds
/// slot k at its body's address.
fn binder_uses(t: &Term, stack: &mut Vec<bool>, out: &mut HashMap<(usize, u32), bool>) {
    sophia_hash::deep::grow(|| {
        let at = |x: &Term| x as *const Term as usize;
        let under = |b: &Term, stack: &mut Vec<bool>, out: &mut HashMap<(usize, u32), bool>, key: usize| {
            stack.push(false);
            binder_uses(b, stack, out);
            let used = stack.pop().unwrap_or(true);
            out.insert((key, 0), used);
        };
        match t {
            Term::Var(i) => {
                let n = stack.len();
                if (*i as usize) < n {
                    stack[n - 1 - *i as usize] = true;
                }
            }
            Term::Con { ty, .. } | Term::Hole(ty) | Term::Unsafe(_, ty) => binder_uses(ty, stack, out),
            Term::Lam { domain, body, .. } => {
                if let Some(d) = domain {
                    binder_uses(d, stack, out);
                }
                under(body, stack, out, at(t));
            }
            Term::App(h, args) => {
                binder_uses(h, stack, out);
                for a in args {
                    binder_uses(a, stack, out);
                }
            }
            Term::Pi { domain, codomain, .. } => {
                binder_uses(domain, stack, out);
                under(codomain, stack, out, at(t));
            }
            Term::Let { ty, value, body, .. } => {
                if let Some(x) = ty {
                    binder_uses(x, stack, out);
                }
                binder_uses(value, stack, out);
                under(body, stack, out, at(t));
            }
            Term::Match { scrutinee, data, motive, cases } => {
                binder_uses(scrutinee, stack, out);
                for x in [data, motive].into_iter().flatten() {
                    binder_uses(x, stack, out);
                }
                for c in cases {
                    let k = c.binders.len();
                    stack.extend(std::iter::repeat_n(false, k));
                    binder_uses(&c.body, stack, out);
                    for j in (0..k).rev() {
                        let used = stack.pop().unwrap_or(true);
                        out.insert((at(&c.body), j as u32), used);
                    }
                }
            }
            _ => {}
        }
    })
}

/// Indentation stops growing after this many columns: nested blocks keep
/// their line breaks, but a text nested n levels deep stays O(n) in size
/// instead of O(n²) (SP-015). The parser ignores indentation.
const MAX_INDENT: usize = 64 * 4;

/// Prints definitions as one text.
pub fn print(env: &dyn Env, ns: &str, profile: &Profile, items: &[ToPrint]) -> String {
    if profile.core {
        return items.iter().map(|i| format!("{} = {:#?}\n", i.name, i.def)).collect::<Vec<_>>().join("\n");
    }
    let mut out = String::new();
    for (k, it) in items.iter().enumerate() {
        if k > 0 {
            out.push('\n');
        }
        let mut uses = HashMap::new();
        for t in it.def.terms() {
            binder_uses(t, &mut Vec::new(), &mut uses);
        }
        let mut pr = Printer { env, ns, p: profile, group: items, scope: Scope::default(), indent: 0, uses };
        out.push_str(&pr.item(it));
        out.push('\n');
    }
    out
}

const ARROW: u8 = 0;
const ATOM: u8 = 4;

fn valid_ident(s: &str) -> bool {
    let mut cs = s.chars();
    matches!(cs.next(), Some(c) if is_ident_start(c)) && cs.all(is_ident_char) && !KEYWORDS.contains(&s)
}

/// Whether `Var(depth)` occurs free in `t`, at any depth (SP-015).
fn occurs(t: &Term, depth: u32) -> bool {
    sophia_hash::deep::grow(|| occurs_step(t, depth))
}

fn occurs_step(t: &Term, depth: u32) -> bool {
    match t {
        Term::Var(i) => *i == depth,
        Term::Con { ty, .. } => occurs(ty, depth),
        Term::Lam { domain, body, .. } => domain.as_ref().is_some_and(|d| occurs(d, depth)) || occurs(body, depth + 1),
        Term::App(h, args) => occurs(h, depth) || args.iter().any(|a| occurs(a, depth)),
        Term::Pi { domain, codomain, .. } => occurs(domain, depth) || occurs(codomain, depth + 1),
        Term::Let { ty, value, body, .. } => {
            ty.as_ref().is_some_and(|t| occurs(t, depth)) || occurs(value, depth) || occurs(body, depth + 1)
        }
        Term::Match { scrutinee, data, motive, cases } => {
            occurs(scrutinee, depth)
                || data.as_ref().is_some_and(|d| occurs(d, depth))
                || motive.as_ref().is_some_and(|m| occurs(m, depth))
                || cases.iter().any(|c| occurs(&c.body, depth + c.binders.len() as u32))
        }
        Term::Hole(t) | Term::Unsafe(_, t) => occurs(t, depth),
        _ => false,
    }
}

impl Printer<'_> {
    fn nl(&self) -> String {
        format!("\n{}", " ".repeat(self.indent.min(MAX_INDENT)))
    }

    /// Whether the binder in `slot` of the node at `at` is used, from the
    /// precomputed table, else by walking.
    fn used(&self, at: &Term, slot: u32, walk: impl FnOnce() -> bool) -> bool {
        match self.uses.get(&(at as *const Term as usize, slot)) {
            Some(u) => *u,
            None => walk(),
        }
    }

    // ---- names ----

    fn fresh(&self, name: &str, used: bool) -> String {
        let base = if name.is_empty() || name == "_" {
            if used { "x".to_string() } else { return "_".into() }
        } else if valid_ident(name) {
            name.to_string()
        } else {
            "x".to_string()
        };
        let taken = |n: &str| {
            self.scope.contains(n)
                || self.group.iter().any(|g| g.name == n)
                || crate::resolve::builtin_named(n).is_some()
        };
        if !taken(&base) {
            return base;
        }
        // The smallest free suffix, searched from the hint (all below it are taken).
        let start = self.scope.hint.borrow().get(&base).copied().unwrap_or(1);
        let i = (start..).find(|i| !taken(&format!("{base}_{i}"))).unwrap();
        self.scope.hint.borrow_mut().insert(base.clone(), i);
        format!("{base}_{i}")
    }

    /// What the parser would make of a non-local path.
    fn resolves(&self, spelling: &str) -> Option<Found> {
        let segs = crate::split_path(spelling);
        if self.scope.contains(segs[0]) {
            return None;
        }
        if segs.len() == 1 && self.group.iter().any(|g| g.name == segs[0]) {
            return None;
        }
        resolve_global(self.env, self.ns, spelling)
    }

    fn group_name_for(&self, id: &Digest) -> Option<String> {
        let g = self.group.iter().find(|g| g.id.as_ref() == Some(id))?;
        (!self.scope.contains(g.name)).then(|| g.name.to_string())
    }

    fn ref_name(&self, d: &Digest) -> String {
        if let Some(n) = self.group_name_for(d) {
            return n;
        }
        let mut names = self.env.names_of(d);
        names.sort_by_key(|n| (n.len(), n.clone()));
        for abs in names {
            for s in spellings(&abs) {
                if self.resolves(&s) == Some(Found::Content(*d)) {
                    return s;
                }
            }
        }
        format!("#{}", d.to_hex())
    }

    fn nominal_name(&self, id: &Digest) -> String {
        if let Some(b) = builtin_name(id) {
            let s = if valid_ident(b) { b.to_string() } else { format!("`{b}`") };
            if self.resolves(&s) == Some(Found::Nominal(*id)) {
                return s;
            }
        }
        let mut names = self.env.names_of(id);
        names.sort_by_key(|n| (n.len(), n.clone()));
        for abs in names {
            for s in spellings(&abs) {
                if self.resolves(&s) == Some(Found::Nominal(*id)) {
                    return s;
                }
            }
        }
        format!("##{}", id.to_hex())
    }

    fn member_name(&self, k: u32) -> String {
        match self.group.iter().find(|g| g.index == k && g.id.is_some()) {
            Some(g) if !self.scope.contains(g.name) => g.name.to_string(),
            _ => format!("@member({k})"),
        }
    }

    /// Constructor names of the data type a term denotes.
    fn ctor_names(&self, ty: &Term) -> Option<Vec<String>> {
        let from_def = |d: &Definition| match &d.body {
            Body::Constructors(cs) => Some(cs.iter().map(|c| c.name.clone()).collect()),
            _ => None,
        };
        match ty {
            Term::Ref(d) => match self.group.iter().find(|g| g.id.as_ref() == Some(d)) {
                Some(g) => from_def(g.def),
                None => self.env.constructors(d),
            },
            Term::Member(k) => self.group.iter().find(|g| g.index == *k).and_then(|g| from_def(g.def)),
            Term::Draft(i) => self.group.get(*i as usize).and_then(|g| from_def(g.def)),
            _ => None,
        }
    }

    fn con(&mut self, ty: &Term, j: u32) -> String {
        let t = self.term(ty, ATOM);
        if let Some(names) = self.ctor_names(ty) {
            if let Some(c) = names.get(j as usize) {
                let unique = names.iter().position(|x| x == c) == Some(j as usize);
                let spelled = format!("{t}.{c}");
                if unique && valid_ident(c) && !t.starts_with('#') && !t.starts_with('@') && self.resolves(&spelled).is_none() {
                    return spelled;
                }
            }
        }
        format!("@con({t}, {j})")
    }

    // ---- terms ----

    fn binder_prefix(b: &Binder) -> &'static str {
        match (b.usage, b.implicit) {
            (Usage::Zero, false) => "@erased ",
            (Usage::One, _) => "@linear ",
            _ => "",
        }
    }

    fn lit(&self, l: &Lit) -> (String, u8) {
        match l {
            Lit::Int64(v) if *v < 0 => (format!("(-{})", v.unsigned_abs()), ATOM),
            Lit::Int64(v) => (v.to_string(), ATOM),
            Lit::UInt64(v) => (format!("0x{v:016x}"), ATOM),
            Lit::Float64(bits) => {
                let x = f64::from_bits(*bits);
                let s = if x.is_nan() {
                    if *bits == f64::NAN.to_bits() { "NaN".into() } else { format!("@float(0x{bits:016x})") }
                } else if x.is_infinite() {
                    if x > 0.0 { "Inf".into() } else { "(-Inf)".into() }
                } else {
                    let mut r = format!("{:?}", x.abs());
                    if !r.contains(['.', 'e', 'E']) {
                        r.push_str(".0");
                    }
                    if x.is_sign_negative() { format!("(-{r})") } else { r }
                };
                (s, ATOM)
            }
            Lit::String(s) => (quote_str(s, '"'), ATOM),
            Lit::Bool(b) => (b.to_string(), ATOM),
            Lit::Char(c) => (quote_str(&c.to_string(), '\''), ATOM),
        }
    }

    pub fn term(&mut self, t: &Term, min: u8) -> String {
        let (s, level) = self.term_level(t);
        if level < min { format!("({s})") } else { s }
    }

    /// One level per call, through `deep::grow` (SP-015): any depth.
    fn term_level(&mut self, t: &Term) -> (String, u8) {
        sophia_hash::deep::grow(|| self.term_level_step(t))
    }

    fn term_level_step(&mut self, t: &Term) -> (String, u8) {
        match t {
            Term::Var(i) => {
                let n = self.scope.len();
                match (*i as usize).checked_add(1).and_then(|k| n.checked_sub(k)) {
                    Some(at) => (self.scope[at].clone(), ATOM),
                    None => (format!("@var({i})"), ATOM),
                }
            }
            Term::Ref(d) => (self.ref_name(d), ATOM),
            Term::Member(k) => (self.member_name(*k), ATOM),
            Term::Draft(i) => match self.group.get(*i as usize) {
                Some(g) if !self.scope.contains(g.name) => (g.name.to_string(), ATOM),
                _ => (format!("@member({i})"), ATOM),
            },
            Term::Nominal(n) => (self.nominal_name(n), ATOM),
            Term::Con { ty, index } => (self.con(ty, *index), ATOM),
            Term::Lam { binder, domain, body } => {
                let name = self.fresh(&binder.name, self.used(t, 0, || occurs(body, 0)));
                let head = match domain {
                    None if binder.usage == Usage::Many && !binder.implicit => name.clone(),
                    None if binder.implicit => format!("{{{}{name}}}", Self::binder_prefix(binder)),
                    None => format!("({}{name})", Self::binder_prefix(binder)),
                    Some(d) => {
                        let d = self.term(d, 1);
                        if binder.implicit {
                            format!("{{{}{name}{}{d}}}", Self::binder_prefix(binder), self.p.ann())
                        } else {
                            format!("({}{name}{}{d})", Self::binder_prefix(binder), self.p.ann())
                        }
                    }
                };
                self.scope.push(name);
                let b = self.term(body, ARROW);
                self.scope.pop();
                (format!("{head} -> {b}"), ARROW)
            }
            Term::Pi { binder, domain, codomain } => {
                let used = self.used(t, 0, || occurs(codomain, 0));
                let plain = !used && binder.usage == Usage::Many && !binder.implicit;
                let d = self.term(domain, if plain { 1 } else { ARROW });
                let (head, name) = if plain {
                    (d, "_".to_string())
                } else {
                    let name = self.fresh(&binder.name, true);
                    let inner = format!("{}{name}{}{d}", Self::binder_prefix(binder), self.p.ann());
                    let h = if binder.implicit { format!("{{{inner}}}") } else { format!("({inner})") };
                    (h, name)
                };
                self.scope.push(name);
                let c = self.term(codomain, ARROW);
                self.scope.pop();
                (format!("{head} {} {c}", self.p.pi_arrow()), ARROW)
            }
            Term::App(h, args) => {
                if let (Term::Nominal(id), [l, r]) = (h.as_ref(), args.as_slice()) {
                    if let Some(op) = builtin_name(id).filter(|o| infix_prec(o).is_some()) {
                        let prec = infix_prec(op).unwrap();
                        let ls = self.term(l, if prec == 1 { 2 } else { prec });
                        let rs = self.term(r, prec + 1);
                        let op = match (self.p.unicode, op) {
                            (true, "<=") => "≤",
                            (true, ">=") => "≥",
                            (true, "!=") => "≠",
                            (_, o) => o,
                        };
                        return (format!("{ls} {op} {rs}"), prec);
                    }
                }
                let hs = self.term(h, ATOM);
                let a: Vec<String> = args.iter().map(|a| self.term(a, ARROW)).collect();
                (format!("{hs}({})", a.join(", ")), ATOM)
            }
            Term::Let { binder, ty, value, body } => {
                let node = t;
                let v = self.term(value, ARROW);
                let t = ty.as_ref().map(|t| self.term(t, 1));
                let name = self.fresh(&binder.name, self.used(node, 0, || occurs(body, 0)));
                let inner = match t {
                    Some(t) => format!("{}{name}{}{t}", Self::binder_prefix(binder), self.p.ann()),
                    None => format!("{}{name}", Self::binder_prefix(binder)),
                };
                let lhs = if binder.implicit {
                    format!("{{{inner}}}")
                } else if binder.usage != Usage::Many {
                    format!("({inner})")
                } else {
                    inner
                };
                self.scope.push(name);
                self.indent += 4;
                let b = self.term(body, ARROW);
                self.indent -= 4;
                self.scope.pop();
                let (open, close) = match self.p.blocks {
                    Blocks::End => (String::new(), format!("{}end", self.nl())),
                    Blocks::Braces => (" {".into(), format!("{}}}", self.nl())),
                    Blocks::Indent => (":".into(), String::new()),
                };
                (format!("let {lhs} = {v}{open}{}    {b}{close}", self.nl()), ARROW)
            }
            Term::Match { scrutinee, data, motive: _, cases } => {
                let s = self.term(scrutinee, 1);
                let names = data.as_ref().and_then(|d| self.ctor_names(d));
                let d = data.as_ref().map(|d| self.term(d, 1));
                let mut out = format!("match {s}");
                if let Some(d) = d {
                    out.push_str(if self.p.annotation == ":" { ": " } else { " :: " });
                    out.push_str(&d);
                }
                out.push_str(match self.p.blocks {
                    Blocks::End => "",
                    Blocks::Braces => " {",
                    Blocks::Indent => ":",
                });
                self.indent += 4;
                for c in cases {
                    let pat = match names.as_ref().and_then(|ns| ns.get(c.constructor as usize)) {
                        Some(n) if valid_ident(n) => n.clone(),
                        _ => format!("@ctor({})", c.constructor),
                    };
                    let n0 = self.scope.len();
                    let mut bs = Vec::new();
                    for (k, b) in c.binders.iter().enumerate() {
                        let depth_from_body = (c.binders.len() - 1 - k) as u32;
                        let name = self.fresh(&b.name, self.used(&c.body, k as u32, || occurs(&c.body, depth_from_body)));
                        self.scope.push(name.clone());
                        bs.push(name);
                    }
                    let body = self.term(&c.body, ARROW);
                    self.scope.truncate(n0);
                    let args = if bs.is_empty() { String::new() } else { format!("({})", bs.join(", ")) };
                    out.push_str(&format!("{}{pat}{args} => {body}", self.nl()));
                }
                self.indent -= 4;
                match self.p.blocks {
                    Blocks::End => out.push_str(&format!("{}end", self.nl())),
                    Blocks::Braces => out.push_str(&format!("{}}}", self.nl())),
                    Blocks::Indent => {}
                }
                (out, ARROW)
            }
            Term::Type(0) => ("Type".into(), ATOM),
            Term::Type(l) => (format!("Type{{{l}}}"), ATOM),
            Term::Prop => ("Prop".into(), ATOM),
            Term::Lit(l) => self.lit(l),
            Term::Prim(n) => (format!("@prim({n})"), ATOM),
            Term::Hole(t) => {
                let s = self.term(t, ARROW);
                (format!("?({s})"), ATOM)
            }
            Term::Unresolved(path) => (path.join("."), ATOM),
            Term::Unsafe(a, t) => {
                let s = self.term(t, ARROW);
                (format!("@unsafe({}, {s})", a.as_str()), ATOM)
            }
        }
    }

    // ---- definitions ----

    fn open_block(&self) -> &'static str {
        match self.p.blocks {
            Blocks::End => "",
            Blocks::Braces => " {",
            Blocks::Indent => ":",
        }
    }

    fn close_block(&self) -> &'static str {
        match self.p.blocks {
            Blocks::End => "\nend",
            Blocks::Braces => "\n}",
            Blocks::Indent => "",
        }
    }

    fn body_block(&mut self, body: &Term) -> String {
        self.indent = 4;
        let b = self.term(body, ARROW);
        self.indent = 0;
        format!("{}\n    {b}{}", self.open_block(), self.close_block())
    }

    fn decls(&mut self, rows: Vec<(String, &Term)>) -> String {
        let mut out = String::from(self.open_block());
        for (n, t) in rows {
            let ts = self.term(t, ARROW);
            let sep = if self.p.blocks == Blocks::Braces { "," } else { "" };
            out.push_str(&format!("\n    {n}{}{ts}{sep}", self.p.ann()));
        }
        out.push_str(self.close_block());
        out
    }

    fn name(it: &ToPrint) -> String {
        if valid_ident(it.name) || (it.name.starts_with('`') && it.name.ends_with('`')) {
            it.name.to_string()
        } else {
            format!("`{}`", it.name)
        }
    }

    /// The `function f(params)::R` form applies when the body is n plain
    /// lambdas and the type has at least n binders.
    fn sugar(def: &Definition) -> Option<usize> {
        let (Some(ty), Body::Term(body)) = (&def.ty, &def.body) else { return None };
        let mut n = 0;
        let mut b = body;
        while let Term::Lam { binder, domain: None, body } = b {
            if binder.usage != Usage::Many || binder.implicit {
                break;
            }
            n += 1;
            b = body;
        }
        let mut t = ty;
        for _ in 0..n {
            match t {
                Term::Pi { binder, codomain, .. } if !binder.implicit || binder.usage == Usage::Zero => t = codomain,
                _ => return None,
            }
        }
        (n > 0).then_some(n)
    }

    fn item(&mut self, it: &ToPrint) -> String {
        let name = Self::name(it);
        let d = it.def;
        let semi = if self.p.blocks == Blocks::Braces { ";" } else { "" };
        if it.relation {
            let t = self.term(d.ty.as_ref().unwrap_or(&Term::Prop), ARROW);
            return format!("relation {name}{}{t}{semi}", self.p.ann());
        }
        match (d.kind, &d.body) {
            (Kind::Method, Body::Term(body)) => {
                if let Some(n) = Self::sugar(d) {
                    let mut ty = d.ty.as_ref().unwrap();
                    let mut b = body;
                    let mut params = Vec::new();
                    for _ in 0..n {
                        let (Term::Pi { binder, domain, codomain }, Term::Lam { binder: lb, body: lbody, .. }) = (ty, b) else {
                            unreachable!()
                        };
                        let used = self.used(ty, 0, || occurs(codomain, 0)) || self.used(b, 0, || occurs(lbody, 0));
                        let pname = self.fresh(&lb.name, used);
                        let is_any = matches!(domain.as_ref(), Term::Nominal(id) if builtin_name(id) == Some("Any"));
                        let p = if is_any && !binder.implicit && binder.usage == Usage::Many {
                            pname.clone()
                        } else {
                            let dom = self.term(domain, ARROW);
                            let inner = format!("{pname}{}{dom}", self.p.ann());
                            match (binder.implicit, binder.usage) {
                                (true, Usage::Zero) => format!("{{{inner}}}"),
                                (true, _) => format!("@linear {{{inner}}}"),
                                (false, Usage::Zero) => format!("@erased {inner}"),
                                (false, Usage::One) => format!("@linear {inner}"),
                                (false, Usage::Many) => inner,
                            }
                        };
                        params.push(p);
                        self.scope.push(pname);
                        ty = codomain;
                        b = lbody;
                    }
                    let ret = self.term(ty, ARROW);
                    let ret_sep = if self.p.annotation == ":" { " -> " } else { "::" };
                    let head = format!("{} {name}({}){ret_sep}{ret}", self.p.kw_function, params.join(", "));
                    let block = self.body_block(b);
                    self.scope.clear();
                    format!("{head}{block}")
                } else {
                    let t = self.term(d.ty.as_ref().unwrap_or(&Term::Prop), ARROW);
                    let block = self.body_block(body);
                    format!("{} {name}{}{t}{block}", self.p.kw_function, self.p.ann())
                }
            }
            (Kind::Constant, Body::Term(v)) => {
                let t = self.term(d.ty.as_ref().unwrap_or(&Term::Prop), ARROW);
                let v = self.term(v, ARROW);
                format!("const {name}{}{t} = {v}{semi}", self.p.ann())
            }
            (Kind::Data, Body::Constructors(cs)) => {
                let t = self.term(d.ty.as_ref().unwrap_or(&Term::Type(0)), ARROW);
                let rows = cs.iter().map(|c| (if valid_ident(&c.name) { c.name.clone() } else { "c".into() }, &c.ty)).collect();
                format!("data {name}{}{t}{}", self.p.ann(), self.decls(rows))
            }
            (Kind::Layout, Body::Fields(fs)) => {
                let rows = fs.iter().map(|f| (f.name.clone(), &f.ty)).collect();
                format!("layout {name}{}", self.decls(rows))
            }
            (Kind::Relation, _) => {
                let t = self.term(d.ty.as_ref().unwrap_or(&Term::Prop), ARROW);
                format!("relation {name}{}{t}{semi}", self.p.ann())
            }
            (Kind::Postulate, _) => {
                let t = self.term(d.ty.as_ref().unwrap_or(&Term::Prop), ARROW);
                format!("postulate {name}{}{t}{semi}", self.p.ann())
            }
            (Kind::Test, Body::Term(b)) => {
                let b = self.term(b, ARROW);
                format!("test {name} = {b}{semi}")
            }
            (Kind::Rule, Body::Rule { vars, head, premises, weight }) => {
                let mut names = Vec::new();
                for v in vars {
                    let n = self.fresh(&v.name, true);
                    self.scope.push(n.clone());
                    names.push(n);
                }
                let h = self.term(head, 1);
                let ps: Vec<String> = premises.iter().map(|p| self.term(p, 1)).collect();
                self.scope.clear();
                let w = weight.as_ref().map(|w| format!(" @weight({})", self.lit(w).0)).unwrap_or_default();
                let tail = if ps.is_empty() { String::new() } else { format!(" :- {}", ps.join(", ")) };
                format!("rule {name}({}){w} = {h}{tail}{semi}", names.join(", "))
            }
            _ => format!("# {name}: a {} definition this printer cannot show\n# {d:?}", d.kind.as_str()),
        }
    }
}
