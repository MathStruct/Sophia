//! Text for queries (SPEC-007 §5): the Julia-flavoured default syntax, its
//! parser, and two printers (`julia`, which round-trips, and `datalog`, for
//! display). The parser produces the IR in written order; [`crate::canon`]
//! makes it canonical. See `syntax.md`.

use std::collections::HashMap;

use sophia_hash::Digest;

use crate::Error;
use crate::ir::*;

/// Bounds on what the parser accepts (SR-50: parsers are bounded).
pub const MAX_TEXT: usize = 64 * 1024;
pub const MAX_RULES: usize = 256;
pub const MAX_LITERALS: usize = 64;
pub const MAX_ARITY: usize = 16;

const KEYWORDS: [&str; 6] = ["query", "select", "where", "end", "count", "startswith"];

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    Sym(String),
    Int(i64),
    Hex(Digest),
    LParen,
    RParen,
    Comma,
    ColonColon,
    Assign,
    EqEq,
    NotEq,
    AndAnd,
    Bang,
}

fn lex(text: &str) -> Result<Vec<(Tok, usize)>, Error> {
    if text.len() > MAX_TEXT {
        return Err(Error::parse(0, format!("query text longer than {MAX_TEXT} bytes")));
    }
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let ident_start = |c: char| c.is_alphabetic() || c == '_';
    let ident_cont = |c: char| c.is_alphanumeric() || c == '_';
    while i < b.len() {
        let c = text[i..].chars().next().unwrap();
        let at = i;
        match c {
            ' ' | '\t' | '\r' | '\n' | ';' => i += 1,
            '#' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            '(' => {
                out.push((Tok::LParen, at));
                i += 1
            }
            ')' => {
                out.push((Tok::RParen, at));
                i += 1
            }
            ',' => {
                out.push((Tok::Comma, at));
                i += 1
            }
            ':' if text[i..].starts_with("::") => {
                out.push((Tok::ColonColon, at));
                i += 2
            }
            ':' => {
                i += 1;
                let s = i;
                while i < b.len() && text[i..].chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    i += text[i..].chars().next().unwrap().len_utf8();
                }
                if s == i {
                    return Err(Error::parse(at, "expected a symbol after ':'"));
                }
                out.push((Tok::Sym(text[s..i].to_string()), at));
            }
            '=' if text[i..].starts_with("==") => {
                out.push((Tok::EqEq, at));
                i += 2
            }
            '=' => {
                out.push((Tok::Assign, at));
                i += 1
            }
            '!' if text[i..].starts_with("!=") => {
                out.push((Tok::NotEq, at));
                i += 2
            }
            '!' => {
                out.push((Tok::Bang, at));
                i += 1
            }
            '&' if text[i..].starts_with("&&") => {
                out.push((Tok::AndAnd, at));
                i += 2
            }
            '"' => {
                i += 1;
                let mut s = String::new();
                loop {
                    let Some(c) = text[i..].chars().next() else {
                        return Err(Error::parse(at, "unterminated string"));
                    };
                    i += c.len_utf8();
                    match c {
                        '"' => break,
                        '\\' => {
                            let Some(e) = text[i..].chars().next() else {
                                return Err(Error::parse(at, "unterminated string"));
                            };
                            i += e.len_utf8();
                            s.push(match e {
                                'n' => '\n',
                                't' => '\t',
                                '"' | '\\' => e,
                                _ => return Err(Error::parse(i, format!("unknown escape \\{e}"))),
                            });
                        }
                        c => s.push(c),
                    }
                }
                out.push((Tok::Str(s), at));
            }
            '0' if text[i..].starts_with("0x") => {
                i += 2;
                let s = i;
                while i < b.len() && b[i].is_ascii_hexdigit() {
                    i += 1;
                }
                let d: Digest = text[s..i]
                    .parse()
                    .map_err(|_| Error::parse(at, "an id literal is 0x followed by 64 hex digits"))?;
                out.push((Tok::Hex(d), at));
            }
            c if c.is_ascii_digit() || (c == '-' && b.get(i + 1).is_some_and(u8::is_ascii_digit)) => {
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                let v = text[at..i].parse().map_err(|_| Error::parse(at, "integer out of range"))?;
                out.push((Tok::Int(v), at));
            }
            c if ident_start(c) => {
                while i < b.len() && text[i..].chars().next().is_some_and(ident_cont) {
                    i += text[i..].chars().next().unwrap().len_utf8();
                }
                out.push((Tok::Ident(text[at..i].to_string()), at));
            }
            _ => return Err(Error::parse(at, format!("unexpected character {c:?}"))),
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
    end: usize,
    params: Vec<(String, Sort)>,
    preds: Vec<(String, Option<u32>, usize)>,
    pred_index: HashMap<String, u32>,
}

/// Variables of one rule or of the select, numbered by first occurrence.
#[derive(Default)]
struct Scope {
    names: Vec<String>,
    index: HashMap<String, u32>,
}

impl Scope {
    fn var(&mut self, name: &str) -> u32 {
        if name == "_" {
            self.names.push("_".into());
            return self.names.len() as u32 - 1;
        }
        if let Some(&v) = self.index.get(name) {
            return v;
        }
        let v = self.names.len() as u32;
        self.names.push(name.into());
        self.index.insert(name.into(), v);
        v
    }
}

impl Parser {
    fn at(&self) -> usize {
        self.toks.get(self.pos).map(|t| t.1).unwrap_or(self.end)
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.0)
    }

    fn peek2(&self) -> Option<&Tok> {
        self.toks.get(self.pos + 1).map(|t| &t.0)
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).map(|t| t.0.clone());
        self.pos += 1;
        t
    }

    fn expect(&mut self, t: Tok, what: &str) -> Result<(), Error> {
        if self.peek() == Some(&t) {
            self.pos += 1;
            Ok(())
        } else {
            Err(Error::parse(self.at(), format!("expected {what}")))
        }
    }

    fn keyword(&self, k: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(s)) if s == k)
    }

    fn ident(&mut self, what: &str) -> Result<String, Error> {
        match self.peek() {
            Some(Tok::Ident(s)) => {
                let s = s.clone();
                self.pos += 1;
                Ok(s)
            }
            _ => Err(Error::parse(self.at(), format!("expected {what}"))),
        }
    }

    fn pred(&mut self, name: &str, arity: usize, defining: bool, at: usize) -> Result<Pred, Error> {
        if let Some(b) = Base::parse(name) {
            if defining {
                return Err(Error::parse(at, format!("`{name}` is a base relation and cannot be defined by rules")));
            }
            if arity != b.arity() {
                return Err(Error::parse(at, format!("`{name}` takes {} arguments, not {arity}", b.arity())));
            }
            return Ok(Pred::Base(b));
        }
        if KEYWORDS.contains(&name) {
            return Err(Error::parse(at, format!("`{name}` is reserved")));
        }
        if arity == 0 || arity > MAX_ARITY {
            return Err(Error::parse(at, format!("a predicate takes 1 to {MAX_ARITY} arguments")));
        }
        let p = match self.pred_index.get(name) {
            Some(&p) => p,
            None => {
                let p = self.preds.len() as u32;
                self.preds.push((name.into(), None, at));
                self.pred_index.insert(name.into(), p);
                p
            }
        };
        let entry = &mut self.preds[p as usize];
        if entry.1.is_some_and(|a| a as usize != arity) {
            return Err(Error::parse(at, format!("`{name}` is used with {} and {arity} arguments", entry.1.unwrap())));
        }
        entry.1 = Some(arity as u32);
        if defining {
            entry.2 = usize::MAX;
        }
        Ok(Pred::Local(p))
    }

    fn term(&mut self, scope: &mut Scope) -> Result<Term, Error> {
        let at = self.at();
        Ok(match self.bump() {
            Some(Tok::Ident(s)) => {
                if KEYWORDS.contains(&s.as_str()) {
                    return Err(Error::parse(at, format!("`{s}` is reserved")));
                }
                match self.params.iter().position(|(n, _)| *n == s) {
                    Some(i) => Term::Param(i as u32),
                    None => Term::Var(scope.var(&s)),
                }
            }
            Some(Tok::Str(s)) | Some(Tok::Sym(s)) => Term::Const(Val::Text(s)),
            Some(Tok::Int(i)) => Term::Const(Val::Int(i)),
            Some(Tok::Hex(d)) => Term::Const(Val::Id(d)),
            _ => return Err(Error::parse(at, "expected a variable, parameter or constant")),
        })
    }

    fn args(&mut self, scope: &mut Scope) -> Result<Vec<Term>, Error> {
        self.expect(Tok::LParen, "`(`")?;
        let mut out = Vec::new();
        if self.peek() != Some(&Tok::RParen) {
            loop {
                out.push(self.term(scope)?);
                if out.len() > MAX_ARITY {
                    return Err(Error::parse(self.at(), "too many arguments"));
                }
                if self.peek() == Some(&Tok::Comma) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        self.expect(Tok::RParen, "`)`")?;
        Ok(out)
    }

    fn atom(&mut self, scope: &mut Scope) -> Result<Atom, Error> {
        let at = self.at();
        let name = self.ident("a predicate")?;
        let args = self.args(scope)?;
        let pred = self.pred(&name, args.len(), false, at)?;
        Ok(Atom { pred, args })
    }

    fn literal(&mut self, scope: &mut Scope) -> Result<Literal, Error> {
        if self.peek() == Some(&Tok::Bang) {
            self.pos += 1;
            return Ok(Literal::Neg(self.atom(scope)?));
        }
        if self.keyword("startswith") {
            self.pos += 1;
            let at = self.at();
            let args = self.args(scope)?;
            let [a, b]: [Term; 2] =
                args.try_into().map_err(|_| Error::parse(at, "startswith takes two arguments"))?;
            return Ok(Literal::StartsWith(a, b));
        }
        if matches!(self.peek(), Some(Tok::Ident(_))) && self.peek2() == Some(&Tok::LParen) {
            return Ok(Literal::Pos(self.atom(scope)?));
        }
        let a = self.term(scope)?;
        let op = self.bump();
        let b = self.term(scope)?;
        match op {
            Some(Tok::EqEq) => Ok(Literal::Eq(a, b)),
            Some(Tok::NotEq) => Ok(Literal::Ne(a, b)),
            _ => Err(Error::parse(self.at(), "expected `==` or `!=`")),
        }
    }

    fn body(&mut self, scope: &mut Scope) -> Result<Vec<Literal>, Error> {
        let mut out = vec![self.literal(scope)?];
        while self.peek() == Some(&Tok::AndAnd) {
            self.pos += 1;
            out.push(self.literal(scope)?);
            if out.len() > MAX_LITERALS {
                return Err(Error::parse(self.at(), "too many literals in one body"));
            }
        }
        Ok(out)
    }

    fn select(&mut self) -> Result<(Select, Vec<String>), Error> {
        self.pos += 1; // `select`
        let mut scope = Scope::default();
        let mut pending: Vec<(bool, String, usize)> = Vec::new();
        loop {
            let at = self.at();
            if self.keyword("count") && self.peek2() == Some(&Tok::LParen) {
                self.pos += 2;
                let v = self.ident("a variable")?;
                self.expect(Tok::RParen, "`)`")?;
                pending.push((true, v, at));
            } else {
                pending.push((false, self.ident("an output variable")?, at));
            }
            if self.peek() == Some(&Tok::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        if !self.keyword("where") {
            return Err(Error::parse(self.at(), "expected `where`"));
        }
        self.pos += 1;
        // Output variables are numbered first, as they appear.
        let mut outputs = Vec::new();
        for (count, name, at) in pending {
            if name == "_" || KEYWORDS.contains(&name.as_str()) || self.params.iter().any(|(n, _)| *n == name) {
                return Err(Error::parse(at, format!("`{name}` cannot be an output")));
            }
            let v = scope.var(&name);
            outputs.push(if count { Output::Count(v) } else { Output::Var(v) });
        }
        let body = self.body(&mut scope)?;
        let vars = scope.names.len() as u32;
        Ok((Select { outputs, body, vars }, scope.names))
    }
}

/// Parses a query in the Julia-flavoured syntax:
///
/// ```text
/// query transitive_dependants(target::Id)
///     uses(a, b) = refs(a, b)
///     uses(a, c) = refs(a, b) && uses(b, c)
///     select a where uses(a, target)
/// end
/// ```
///
/// A bare `select … where …` is an anonymous query without parameters or
/// rules.
pub fn parse(text: &str) -> Result<(Query, Names), Error> {
    let toks = lex(text)?;
    let mut p = Parser { toks, pos: 0, end: text.len(), params: Vec::new(), preds: Vec::new(), pred_index: HashMap::new() };
    let mut names = Names::default();
    let mut rules = Vec::new();
    let mut select = None;

    if p.keyword("select") {
        let (s, v) = p.select()?;
        select = Some(s);
        names.select_vars = v;
    } else {
        if !p.keyword("query") {
            return Err(Error::parse(p.at(), "expected `query` or `select`"));
        }
        p.pos += 1;
        names.query = p.ident("the query's name")?;
        if p.peek() == Some(&Tok::LParen) {
            p.pos += 1;
            if p.peek() != Some(&Tok::RParen) {
                loop {
                    let at = p.at();
                    let n = p.ident("a parameter")?;
                    p.expect(Tok::ColonColon, "`::` and a sort")?;
                    let s = p.ident("a sort")?;
                    let sort = Sort::parse(&s).ok_or_else(|| Error::parse(at, format!("unknown sort `{s}`: Id, Text or Int")))?;
                    if p.params.iter().any(|(m, _)| *m == n) || n == "_" || KEYWORDS.contains(&n.as_str()) {
                        return Err(Error::parse(at, format!("bad parameter name `{n}`")));
                    }
                    p.params.push((n, sort));
                    if p.peek() == Some(&Tok::Comma) {
                        p.pos += 1;
                    } else {
                        break;
                    }
                }
            }
            p.expect(Tok::RParen, "`)`")?;
        }
        loop {
            if p.keyword("end") {
                p.pos += 1;
                break;
            }
            if p.keyword("select") {
                if select.is_some() {
                    return Err(Error::parse(p.at(), "a query has one `select`"));
                }
                let (s, v) = p.select()?;
                select = Some(s);
                names.select_vars = v;
                continue;
            }
            if p.peek().is_none() {
                return Err(Error::parse(p.at(), "expected `end`"));
            }
            let at = p.at();
            let mut scope = Scope::default();
            let head = p.ident("a rule head")?;
            let args = p.args(&mut scope)?;
            if args.iter().any(|a| matches!(a, Term::Var(v) if scope.names[*v as usize] == "_")) {
                return Err(Error::parse(at, "`_` cannot appear in a rule head"));
            }
            let Pred::Local(h) = p.pred(&head, args.len(), true, at)? else { unreachable!() };
            p.expect(Tok::Assign, "`=` and the rule body")?;
            let body = p.body(&mut scope)?;
            rules.push(Rule { head: h, args, body, vars: scope.names.len() as u32 });
            names.rule_vars.push(scope.names);
            if rules.len() > MAX_RULES {
                return Err(Error::parse(at, "too many rules"));
            }
        }
    }
    if p.pos < p.toks.len() {
        return Err(Error::parse(p.at(), "unexpected text after the query"));
    }
    let select = select.ok_or_else(|| Error::parse(p.end, "a query needs a `select`"))?;
    if let Some((name, _, at)) = p.preds.iter().find(|(_, _, at)| *at != usize::MAX) {
        return Err(Error::parse(*at, format!("`{name}` is neither a base relation nor defined by a rule")));
    }
    names.params = p.params.iter().map(|(n, _)| n.clone()).collect();
    names.preds = p.preds.iter().map(|(n, _, _)| n.clone()).collect();
    let q = Query {
        params: p.params.iter().map(|(_, s)| *s).collect(),
        preds: p.preds.iter().map(|(_, a, _)| a.unwrap()).collect(),
        rules,
        select,
    };
    Ok((q, names))
}

/// The printing profiles of the query syntax (SPEC-007 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// The default, Julia-flavoured; parses back to the same query.
    Julia,
    /// Datalog notation, for display.
    Datalog,
}

impl Profile {
    pub fn parse(s: &str) -> Option<Profile> {
        match s {
            "julia" => Some(Profile::Julia),
            "datalog" => Some(Profile::Datalog),
            _ => None,
        }
    }
}

fn is_symbol(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(|c| c.is_alphabetic() || c == '_') && cs.all(|c| c.is_alphanumeric() || c == '_')
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

struct Printer<'a> {
    q: &'a Query,
    n: &'a Names,
    profile: Profile,
}

impl Printer<'_> {
    fn var(&self, names: &[String], v: u32) -> String {
        let raw = names.get(v as usize).cloned().unwrap_or_else(|| format!("x{v}"));
        let raw = if raw.is_empty() { format!("x{v}") } else { raw };
        match self.profile {
            Profile::Julia => raw,
            Profile::Datalog if raw == "_" => raw,
            Profile::Datalog => {
                let mut cs = raw.chars();
                let first = cs.next().unwrap();
                format!("{}{}", first.to_uppercase(), cs.as_str())
            }
        }
    }

    fn param(&self, i: u32) -> String {
        let raw = self.n.params.get(i as usize).cloned().unwrap_or_else(|| format!("p{i}"));
        match self.profile {
            Profile::Julia => raw,
            Profile::Datalog => format!("${raw}"),
        }
    }

    fn term(&self, names: &[String], t: &Term) -> String {
        match t {
            Term::Var(v) => self.var(names, *v),
            Term::Param(i) => self.param(*i),
            Term::Const(Val::Text(s)) if self.profile == Profile::Julia && is_symbol(s) && !KEYWORDS.contains(&s.as_str()) => {
                format!(":{s}")
            }
            Term::Const(Val::Text(s)) => quote(s),
            Term::Const(Val::Int(i)) => i.to_string(),
            Term::Const(Val::Id(d)) => format!("0x{}", d.to_hex()),
        }
    }

    fn pred(&self, p: Pred) -> String {
        match p {
            Pred::Base(b) => b.name().into(),
            Pred::Local(i) => self.n.preds.get(i as usize).cloned().unwrap_or_else(|| format!("p{i}")),
        }
    }

    fn atom(&self, names: &[String], a: &Atom) -> String {
        let args: Vec<String> = a.args.iter().map(|t| self.term(names, t)).collect();
        format!("{}({})", self.pred(a.pred), args.join(", "))
    }

    fn literal(&self, names: &[String], l: &Literal) -> String {
        let not = if self.profile == Profile::Julia { "!" } else { "\\+ " };
        match l {
            Literal::Pos(a) => self.atom(names, a),
            Literal::Neg(a) => format!("{not}{}", self.atom(names, a)),
            Literal::Eq(a, b) => format!("{} == {}", self.term(names, a), self.term(names, b)),
            Literal::Ne(a, b) => format!("{} != {}", self.term(names, a), self.term(names, b)),
            Literal::StartsWith(a, b) => format!("startswith({}, {})", self.term(names, a), self.term(names, b)),
        }
    }

    fn body(&self, names: &[String], body: &[Literal]) -> String {
        let sep = if self.profile == Profile::Julia { " && " } else { ", " };
        body.iter().map(|l| self.literal(names, l)).collect::<Vec<_>>().join(sep)
    }

    fn outputs(&self) -> String {
        let s = &self.q.select;
        s.outputs
            .iter()
            .map(|o| match o {
                Output::Var(v) => self.var(&self.n.select_vars, *v),
                Output::Count(v) => format!("count({})", self.var(&self.n.select_vars, *v)),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn print(&self) -> String {
        let q = self.q;
        let empty = Vec::new();
        let mut out = String::new();
        match self.profile {
            Profile::Julia => {
                let sel = format!(
                    "select {} where {}",
                    self.outputs(),
                    self.body(&self.n.select_vars, &q.select.body)
                );
                if q.params.is_empty() && q.rules.is_empty() && self.n.query.is_empty() {
                    return sel + "\n";
                }
                let params: Vec<String> = q
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, s)| format!("{}::{}", self.param(i as u32), s.as_str()))
                    .collect();
                let name = if self.n.query.is_empty() { "q" } else { &self.n.query };
                if params.is_empty() {
                    out.push_str(&format!("query {name}\n"));
                } else {
                    out.push_str(&format!("query {name}({})\n", params.join(", ")));
                }
                for (i, r) in q.rules.iter().enumerate() {
                    let names = self.n.rule_vars.get(i).unwrap_or(&empty);
                    let args: Vec<String> = r.args.iter().map(|t| self.term(names, t)).collect();
                    out.push_str(&format!(
                        "    {}({}) = {}\n",
                        self.pred(Pred::Local(r.head)),
                        args.join(", "),
                        self.body(names, &r.body)
                    ));
                }
                out.push_str(&format!("    {sel}\nend\n"));
            }
            Profile::Datalog => {
                let params: Vec<String> =
                    q.params.iter().enumerate().map(|(i, s)| format!("{} : {}", self.param(i as u32), s.as_str())).collect();
                if !self.n.query.is_empty() || !params.is_empty() {
                    out.push_str(&format!("% query {}({})\n", self.n.query, params.join(", ")));
                }
                for (i, r) in q.rules.iter().enumerate() {
                    let names = self.n.rule_vars.get(i).unwrap_or(&empty);
                    let args: Vec<String> = r.args.iter().map(|t| self.term(names, t)).collect();
                    let head = format!("{}({})", self.pred(Pred::Local(r.head)), args.join(", "));
                    if r.body.is_empty() {
                        out.push_str(&format!("{head}.\n"));
                    } else {
                        out.push_str(&format!("{head} :- {}.\n", self.body(names, &r.body)));
                    }
                }
                out.push_str(&format!(
                    "?- select({}) :- {}.\n",
                    self.outputs(),
                    self.body(&self.n.select_vars, &q.select.body)
                ));
            }
        }
        out
    }
}

/// Prints a query in a profile. With [`Profile::Julia`], parsing the text
/// gives back the same query (the round-trip guarantee of SPEC-005, applied
/// to queries).
pub fn print(q: &Query, names: &Names, profile: Profile) -> String {
    Printer { q, n: names, profile }.print()
}
