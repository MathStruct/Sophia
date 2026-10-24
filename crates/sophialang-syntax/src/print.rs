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

/// For every binder in `t`, whether its body uses it: one pass with an
/// explicit stack, any depth. Lam, Pi and Let bind slot 0 at their own
/// address; a match case binds slot k at its body's address.
fn binder_uses(t: &Term, out: &mut HashMap<(usize, u32), bool>) {
    enum W<'t> {
        Visit(&'t Term),
        /// Open `n` binders.
        Bind(usize),
        /// Close `n` binders, recording each under (`key`, slot).
        Unbind(usize, usize),
    }
    let at = |x: &Term| x as *const Term as usize;
    let mut used: Vec<bool> = Vec::new();
    let mut work = vec![W::Visit(t)];
    while let Some(w) = work.pop() {
        match w {
            W::Bind(n) => used.extend(std::iter::repeat_n(false, n)),
            W::Unbind(key, n) => {
                for slot in (0..n).rev() {
                    let u = used.pop().unwrap_or(true);
                    out.insert((key, slot as u32), u);
                }
            }
            W::Visit(t) => {
                // Pushed in reverse: the last pushed runs first.
                match t {
                    Term::Var(i) => {
                        let n = used.len();
                        if (*i as usize) < n {
                            used[n - 1 - *i as usize] = true;
                        }
                    }
                    Term::Con { ty, .. } | Term::Hole(ty) | Term::Unsafe(_, ty) => work.push(W::Visit(ty)),
                    Term::Lam { domain, body, .. } => {
                        work.extend([W::Unbind(at(t), 1), W::Visit(body), W::Bind(1)]);
                        work.extend(domain.iter().map(|d| W::Visit(d)));
                    }
                    Term::App(h, args) => {
                        work.extend(args.iter().rev().map(W::Visit));
                        work.push(W::Visit(h));
                    }
                    Term::Pi { domain, codomain, .. } => {
                        work.extend([W::Unbind(at(t), 1), W::Visit(codomain), W::Bind(1), W::Visit(domain)]);
                    }
                    Term::Let { ty, value, body, .. } => {
                        work.extend([W::Unbind(at(t), 1), W::Visit(body), W::Bind(1), W::Visit(value)]);
                        work.extend(ty.iter().map(|x| W::Visit(x)));
                    }
                    Term::Match { scrutinee, data, motive, cases } => {
                        for c in cases.iter().rev() {
                            let k = c.binders.len();
                            work.extend([W::Unbind(at(&c.body), k), W::Visit(&c.body), W::Bind(k)]);
                        }
                        work.extend(motive.iter().chain(data.iter()).map(|x| W::Visit(x)));
                        work.push(W::Visit(scrutinee));
                    }
                    _ => {}
                }
            }
        }
    }
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
            binder_uses(t, &mut uses);
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

/// Whether `Var(depth)` occurs free in `t`, with an explicit stack: any
/// depth (SP-015).
fn occurs(t: &Term, depth: u32) -> bool {
    let mut stack = vec![(t, depth)];
    while let Some((t, d)) = stack.pop() {
        match t {
            Term::Var(i) if *i == d => return true,
            Term::Con { ty, .. } | Term::Hole(ty) | Term::Unsafe(_, ty) => stack.push((ty, d)),
            Term::Lam { domain, body, .. } => {
                stack.extend(domain.iter().map(|x| (x.as_ref(), d)));
                stack.push((body, d + 1));
            }
            Term::App(h, args) => {
                stack.push((h, d));
                stack.extend(args.iter().map(|a| (a, d)));
            }
            Term::Pi { domain, codomain, .. } => {
                stack.push((domain, d));
                stack.push((codomain, d + 1));
            }
            Term::Let { ty, value, body, .. } => {
                stack.extend(ty.iter().map(|x| (x.as_ref(), d)));
                stack.push((value, d));
                stack.push((body, d + 1));
            }
            Term::Match { scrutinee, data, motive, cases } => {
                stack.push((scrutinee, d));
                stack.extend(data.iter().chain(motive.iter()).map(|x| (x.as_ref(), d)));
                stack.extend(cases.iter().map(|c| (&c.body, d + c.binders.len() as u32)));
            }
            _ => {}
        }
    }
    false
}

/// The printer's output before it is joined: a tree of pieces, so a parent
/// takes its children's text without copying it.
enum Doc {
    Text(String),
    Cat(Vec<Doc>),
}

/// Joins a [`Doc`] with an explicit stack: any depth, linear time.
fn flatten(d: Doc) -> String {
    let mut out = String::new();
    let mut stack = vec![d];
    while let Some(d) = stack.pop() {
        match d {
            Doc::Text(s) => out.push_str(&s),
            Doc::Cat(mut v) => {
                while let Some(x) = v.pop() {
                    stack.push(x);
                }
            }
        }
    }
    out
}

fn paren(d: Doc, level: u8, min: u8) -> Doc {
    if level < min { Doc::Cat(vec![Doc::Text("(".into()), d, Doc::Text(")".into())]) } else { d }
}

/// A part of a node's text: literal text or the text of its k-th child.
enum Piece {
    Text(String),
    Child(usize),
}

/// One step of the printer's machine ([`Printer::doc`]).
enum Step<'t> {
    /// Print this term at this minimum level, pushing its text.
    Eval(&'t Term, u8),
    Push(String),
    Pop,
    Truncate(usize),
    Indent(isize),
    /// Pop `children` texts, join them with `pieces`, parenthesise if
    /// `level < min`, push the result.
    Combine { pieces: Vec<Piece>, children: usize, level: u8, min: u8 },
    /// Pop a type's text and push the constructor `j` of that type.
    Con { names: Option<Vec<String>>, j: u32, min: u8 },
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

    /// A constructor's text from its type's text: `T.name` when that
    /// spelling is unique and resolves back, else `@con(T, j)`.
    fn con_text(&self, t: &str, names: Option<Vec<String>>, j: u32) -> String {
        if let Some(names) = names {
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

    /// A term's text, parenthesised when its level is below `min`.
    pub fn term(&mut self, t: &Term, min: u8) -> String {
        flatten(self.doc(t, min))
    }

    /// Prints a term with an explicit stack of steps instead of recursion
    /// (SP-015): any depth on every target, wasm included, and linear time,
    /// since children's text is assembled as a tree of pieces ([`Doc`]) and
    /// joined once ([`flatten`]) instead of being copied into every parent.
    /// Each node is planned when its `Eval` step runs, so the scope and the
    /// indentation are exactly those the recursive printer saw.
    fn doc(&mut self, t: &Term, min: u8) -> Doc {
        let mut steps: Vec<Step> = vec![Step::Eval(t, min)];
        let mut vals: Vec<Doc> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Eval(t, min) => self.plan(t, min, &mut steps, &mut vals),
                Step::Push(n) => self.scope.push(n),
                Step::Pop => self.scope.pop(),
                Step::Truncate(n) => self.scope.truncate(n),
                Step::Indent(d) => self.indent = self.indent.wrapping_add_signed(d),
                Step::Combine { pieces, children, level, min } => {
                    let mut args: Vec<Option<Doc>> = vals.split_off(vals.len() - children).into_iter().map(Some).collect();
                    let parts = pieces
                        .into_iter()
                        .map(|p| match p {
                            Piece::Text(s) => Doc::Text(s),
                            Piece::Child(i) => args[i].take().unwrap_or(Doc::Text(String::new())),
                        })
                        .collect();
                    vals.push(paren(Doc::Cat(parts), level, min));
                }
                Step::Con { names, j, min } => {
                    let ty = flatten(vals.pop().unwrap_or(Doc::Text(String::new())));
                    vals.push(paren(Doc::Text(self.con_text(&ty, names, j)), ATOM, min));
                }
            }
        }
        vals.pop().unwrap_or(Doc::Text(String::new()))
    }

    /// Plans one node: a leaf's text goes to `vals`; an inner node pushes
    /// its steps (children, scope changes, the combination) onto `steps`.
    fn plan<'t>(&mut self, t: &'t Term, min: u8, steps: &mut Vec<Step<'t>>, vals: &mut Vec<Doc>) {
        let leaf = |vals: &mut Vec<Doc>, s: String, level: u8| vals.push(paren(Doc::Text(s), level, min));
        // Steps are run first to last; the stack wants them reversed.
        let mut seq: Vec<Step<'t>> = Vec::new();
        match t {
            Term::Var(i) => {
                let n = self.scope.len();
                match (*i as usize).checked_add(1).and_then(|k| n.checked_sub(k)) {
                    Some(at) => leaf(vals, self.scope[at].clone(), ATOM),
                    None => leaf(vals, format!("@var({i})"), ATOM),
                }
                return;
            }
            Term::Ref(d) => return leaf(vals, self.ref_name(d), ATOM),
            Term::Member(k) => return leaf(vals, self.member_name(*k), ATOM),
            Term::Draft(i) => {
                let s = match self.group.get(*i as usize) {
                    Some(g) if !self.scope.contains(g.name) => g.name.to_string(),
                    _ => format!("@member({i})"),
                };
                return leaf(vals, s, ATOM);
            }
            Term::Nominal(n) => return leaf(vals, self.nominal_name(n), ATOM),
            Term::Type(0) => return leaf(vals, "Type".into(), ATOM),
            Term::Type(l) => return leaf(vals, format!("Type{{{l}}}"), ATOM),
            Term::Prop => return leaf(vals, "Prop".into(), ATOM),
            Term::Lit(l) => {
                let (s, level) = self.lit(l);
                return leaf(vals, s, level);
            }
            Term::Prim(n) => return leaf(vals, format!("@prim({n})"), ATOM),
            Term::Unresolved(path) => return leaf(vals, path.join("."), ATOM),
            Term::Con { ty, index } => {
                seq.push(Step::Eval(ty, ATOM));
                seq.push(Step::Con { names: self.ctor_names(ty), j: *index, min });
            }
            Term::Lam { binder, domain, body } => {
                let name = self.fresh(&binder.name, self.used(t, 0, || occurs(body, 0)));
                let mut pieces = Vec::new();
                let pre = Self::binder_prefix(binder);
                let mut children = 0;
                match domain {
                    None if binder.usage == Usage::Many && !binder.implicit => pieces.push(Piece::Text(name.clone())),
                    None if binder.implicit => pieces.push(Piece::Text(format!("{{{pre}{name}}}"))),
                    None => pieces.push(Piece::Text(format!("({pre}{name})"))),
                    Some(d) => {
                        seq.push(Step::Eval(d, 1));
                        let (open, close) = if binder.implicit { ("{", "}") } else { ("(", ")") };
                        pieces.push(Piece::Text(format!("{open}{pre}{name}{}", self.p.ann())));
                        pieces.push(Piece::Child(0));
                        pieces.push(Piece::Text(close.into()));
                        children = 1;
                    }
                }
                pieces.push(Piece::Text(" -> ".into()));
                pieces.push(Piece::Child(children));
                seq.push(Step::Push(name));
                seq.push(Step::Eval(body, ARROW));
                seq.push(Step::Pop);
                seq.push(Step::Combine { pieces, children: children + 1, level: ARROW, min });
            }
            Term::Pi { binder, domain, codomain } => {
                let used = self.used(t, 0, || occurs(codomain, 0));
                let plain = !used && binder.usage == Usage::Many && !binder.implicit;
                seq.push(Step::Eval(domain, if plain { 1 } else { ARROW }));
                let mut pieces = Vec::new();
                let name = if plain {
                    pieces.push(Piece::Child(0));
                    "_".to_string()
                } else {
                    let name = self.fresh(&binder.name, true);
                    let (open, close) = if binder.implicit { ("{", "}") } else { ("(", ")") };
                    pieces.push(Piece::Text(format!("{open}{}{name}{}", Self::binder_prefix(binder), self.p.ann())));
                    pieces.push(Piece::Child(0));
                    pieces.push(Piece::Text(close.into()));
                    name
                };
                pieces.push(Piece::Text(format!(" {} ", self.p.pi_arrow())));
                pieces.push(Piece::Child(1));
                seq.push(Step::Push(name));
                seq.push(Step::Eval(codomain, ARROW));
                seq.push(Step::Pop);
                seq.push(Step::Combine { pieces, children: 2, level: ARROW, min });
            }
            Term::App(h, args) => {
                let infix = match (h.as_ref(), args.as_slice()) {
                    (Term::Nominal(id), [l, r]) => builtin_name(id).filter(|o| infix_prec(o).is_some()).map(|op| (op, l, r)),
                    _ => None,
                };
                if let Some((op, l, r)) = infix {
                    let prec = infix_prec(op).unwrap();
                    let op = match (self.p.unicode, op) {
                        (true, "<=") => "≤",
                        (true, ">=") => "≥",
                        (true, "!=") => "≠",
                        (_, o) => o,
                    };
                    seq.push(Step::Eval(l, if prec == 1 { 2 } else { prec }));
                    seq.push(Step::Eval(r, prec + 1));
                    let pieces = vec![Piece::Child(0), Piece::Text(format!(" {op} ")), Piece::Child(1)];
                    seq.push(Step::Combine { pieces, children: 2, level: prec, min });
                } else {
                    seq.push(Step::Eval(h, ATOM));
                    let mut pieces = vec![Piece::Child(0), Piece::Text("(".into())];
                    for (k, a) in args.iter().enumerate() {
                        seq.push(Step::Eval(a, ARROW));
                        if k > 0 {
                            pieces.push(Piece::Text(", ".into()));
                        }
                        pieces.push(Piece::Child(k + 1));
                    }
                    pieces.push(Piece::Text(")".into()));
                    seq.push(Step::Combine { pieces, children: args.len() + 1, level: ATOM, min });
                }
            }
            Term::Let { binder, ty, value, body } => {
                let name = self.fresh(&binder.name, self.used(t, 0, || occurs(body, 0)));
                let pre = Self::binder_prefix(binder);
                let (open_b, close_b) = if binder.implicit {
                    ("{", "}")
                } else if binder.usage != Usage::Many {
                    ("(", ")")
                } else {
                    ("", "")
                };
                seq.push(Step::Eval(value, ARROW));
                let mut pieces = vec![Piece::Text(format!("let {open_b}{pre}{name}"))];
                let mut children = 1;
                if let Some(ty) = ty {
                    seq.push(Step::Eval(ty, 1));
                    pieces.push(Piece::Text(self.p.ann().to_string()));
                    pieces.push(Piece::Child(1));
                    children = 2;
                }
                let nl = self.nl();
                let (open, close) = match self.p.blocks {
                    Blocks::End => (String::new(), format!("{nl}end")),
                    Blocks::Braces => (" {".into(), format!("{nl}}}")),
                    Blocks::Indent => (":".into(), String::new()),
                };
                pieces.push(Piece::Text(format!("{close_b} = ")));
                pieces.push(Piece::Child(0));
                pieces.push(Piece::Text(format!("{open}{nl}    ")));
                pieces.push(Piece::Child(children));
                pieces.push(Piece::Text(close));
                seq.push(Step::Push(name));
                seq.push(Step::Indent(4));
                seq.push(Step::Eval(body, ARROW));
                seq.push(Step::Indent(-4));
                seq.push(Step::Pop);
                seq.push(Step::Combine { pieces, children: children + 1, level: ARROW, min });
            }
            Term::Match { scrutinee, data, motive: _, cases } => {
                let names = data.as_ref().and_then(|d| self.ctor_names(d));
                seq.push(Step::Eval(scrutinee, 1));
                let mut pieces = vec![Piece::Text("match ".into()), Piece::Child(0)];
                let mut children = 1;
                if let Some(d) = data {
                    seq.push(Step::Eval(d, 1));
                    pieces.push(Piece::Text(if self.p.annotation == ":" { ": " } else { " :: " }.into()));
                    pieces.push(Piece::Child(1));
                    children = 2;
                }
                pieces.push(Piece::Text(
                    match self.p.blocks {
                        Blocks::End => "",
                        Blocks::Braces => " {",
                        Blocks::Indent => ":",
                    }
                    .into(),
                ));
                let outer = self.nl();
                self.indent += 4;
                let inner = self.nl();
                for c in cases {
                    let pat = match names.as_ref().and_then(|ns| ns.get(c.constructor as usize)) {
                        Some(n) if valid_ident(n) => n.clone(),
                        _ => format!("@ctor({})", c.constructor),
                    };
                    // Binder names as the recursive printer chose them: each
                    // sees the ones before it in scope.
                    let n0 = self.scope.len();
                    let mut bs = Vec::new();
                    for (k, b) in c.binders.iter().enumerate() {
                        let depth_from_body = (c.binders.len() - 1 - k) as u32;
                        let name = self.fresh(&b.name, self.used(&c.body, k as u32, || occurs(&c.body, depth_from_body)));
                        self.scope.push(name.clone());
                        bs.push(name);
                    }
                    self.scope.truncate(n0);
                    let args = if bs.is_empty() { String::new() } else { format!("({})", bs.join(", ")) };
                    pieces.push(Piece::Text(format!("{inner}{pat}{args} => ")));
                    pieces.push(Piece::Child(children));
                    children += 1;
                    seq.push(Step::Indent(4));
                    for b in bs {
                        seq.push(Step::Push(b));
                    }
                    seq.push(Step::Eval(&c.body, ARROW));
                    seq.push(Step::Truncate(n0));
                    seq.push(Step::Indent(-4));
                }
                self.indent -= 4;
                match self.p.blocks {
                    Blocks::End => pieces.push(Piece::Text(format!("{outer}end"))),
                    Blocks::Braces => pieces.push(Piece::Text(format!("{outer}}}"))),
                    Blocks::Indent => {}
                }
                seq.push(Step::Combine { pieces, children, level: ARROW, min });
            }
            Term::Hole(t) => {
                seq.push(Step::Eval(t, ARROW));
                let pieces = vec![Piece::Text("?(".into()), Piece::Child(0), Piece::Text(")".into())];
                seq.push(Step::Combine { pieces, children: 1, level: ATOM, min });
            }
            Term::Unsafe(a, t) => {
                seq.push(Step::Eval(t, ARROW));
                let pieces = vec![Piece::Text(format!("@unsafe({}, ", a.as_str())), Piece::Child(0), Piece::Text(")".into())];
                seq.push(Step::Combine { pieces, children: 1, level: ATOM, min });
            }
        }
        steps.extend(seq.into_iter().rev());
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
