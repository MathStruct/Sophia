//! Parser: tokens → surface items. Profile-agnostic: it accepts the Julia
//! and Rust forms alike (`function`/`fn`/`def`, `end` or braces, `::` or
//! `:`), so every editable profile's output parses. See `parser.md`.

use sophialang_core::{Aspect, Usage};

use crate::ast::*;
use crate::diagnostic::{Diagnostic, Span};
use crate::lexer::{Limits, Tok, Token, lex};

pub fn parse(src: &str, limits: Limits) -> Result<Vec<SItem>, Diagnostic> {
    if src.len() > limits.max_bytes {
        return Err(Diagnostic::error(Span::new(0, 0), format!("text exceeds {} bytes", limits.max_bytes)));
    }
    let toks = lex(src)?;
    let mut p = Parser { toks, at: 0, parens: 0, nodes: 0, limits };
    let mut items = Vec::new();
    p.skip_nl();
    while !p.is(&Tok::Eof) {
        if items.len() == limits.max_definitions {
            return Err(Diagnostic::error(p.span(), format!("more than {} definitions", limits.max_definitions)));
        }
        items.push(p.item()?);
        p.skip_separators();
    }
    Ok(items)
}

struct Parser {
    toks: Vec<Token>,
    at: usize,
    /// Inside parentheses or brackets, newlines are insignificant.
    parens: usize,
    /// Expression nodes read so far, against `limits.max_nodes`.
    nodes: usize,
    limits: Limits,
}

type R<T> = Result<T, Diagnostic>;

impl Parser {
    fn raw(&self) -> &Token {
        &self.toks[self.at]
    }

    fn tok(&mut self) -> &Tok {
        if self.parens > 0 {
            self.skip_nl();
        }
        &self.toks[self.at].tok
    }

    fn span(&self) -> Span {
        self.raw().span
    }

    fn is(&mut self, t: &Tok) -> bool {
        self.tok() == t
    }

    fn sym(&mut self, s: &str) -> bool {
        matches!(self.tok(), Tok::Sym(x) if *x == s)
    }

    fn kw(&mut self, k: &str) -> bool {
        matches!(self.tok(), Tok::Ident(x) if x == k)
    }

    fn at_(&mut self, k: &str) -> bool {
        matches!(self.tok(), Tok::At(x) if x == k)
    }

    fn bump(&mut self) -> Token {
        let _ = self.tok();
        let t = self.toks[self.at].clone();
        if !matches!(t.tok, Tok::Eof) {
            self.at += 1;
        }
        t
    }

    fn skip_nl(&mut self) {
        while matches!(self.toks[self.at].tok, Tok::Newline) {
            self.at += 1;
        }
    }

    fn skip_separators(&mut self) {
        while matches!(self.toks[self.at].tok, Tok::Newline | Tok::Sym(";")) {
            self.at += 1;
        }
    }

    fn err<T>(&mut self, msg: impl Into<String>) -> R<T> {
        let found = self.tok().to_string();
        Err(Diagnostic::error(self.span(), format!("{} (found {found})", msg.into())))
    }

    fn expect_sym(&mut self, s: &str) -> R<Span> {
        if self.sym(s) { Ok(self.bump().span) } else { self.err(format!("expected `{s}`")) }
    }

    fn expect_kw(&mut self, k: &str) -> R<Span> {
        if self.kw(k) { Ok(self.bump().span) } else { self.err(format!("expected `{k}`")) }
    }

    fn name(&mut self) -> R<(String, Span)> {
        match self.tok().clone() {
            Tok::Ident(s) => Ok((s, self.bump().span)),
            Tok::Quoted(s) => Ok((format!("`{s}`"), self.bump().span)),
            _ => self.err("expected a name"),
        }
    }

    fn ann(&mut self) -> R<SExpr> {
        if self.sym("::") || self.sym(":") {
            self.bump();
            self.expr()
        } else {
            self.err("expected `::` or `:` and a type")
        }
    }

    fn open_paren(&mut self, s: &str) -> R<Span> {
        let sp = self.expect_sym(s)?;
        self.parens += 1;
        Ok(sp)
    }

    fn close_paren(&mut self, s: &str) -> R<Span> {
        self.parens -= 1;
        // Allow newlines before the closing bracket.
        self.skip_nl();
        self.expect_sym(s)
    }

    // ---- items ----

    fn item(&mut self) -> R<SItem> {
        let start = self.span();
        let Tok::Ident(kw) = self.tok().clone() else { return self.err("expected a definition") };
        self.bump();
        let (name, name_span) = self.name()?;
        let body = match kw.as_str() {
            "function" | "fn" | "def" => {
                if self.sym("(") {
                    self.open_paren("(")?;
                    let mut params = Vec::new();
                    while !self.sym(")") {
                        params.push(self.param()?);
                        if !self.sym(")") {
                            self.expect_sym(",")?;
                        }
                    }
                    self.close_paren(")")?;
                    if !(self.sym("::") || self.sym("->") || self.sym(":")) {
                        return self.err("expected a return type (`::R` or `-> R`)");
                    }
                    self.bump();
                    let ret = self.expr()?;
                    let body = self.block()?;
                    SBody::Sugar { params, ret, body }
                } else {
                    let ty = self.ann()?;
                    let body = self.block()?;
                    SBody::Typed { ty, body }
                }
            }
            "const" => {
                let ty = self.ann()?;
                self.expect_sym("=")?;
                SBody::Constant { ty, value: self.expr()? }
            }
            "data" => {
                let ty = self.ann()?;
                SBody::Data { ty, constructors: self.decl_block()? }
            }
            "layout" => SBody::Layout { fields: self.decl_block()? },
            "relation" => SBody::Relation { ty: self.ann()? },
            "postulate" => SBody::Postulate { ty: self.ann()? },
            "test" => {
                self.expect_sym("=")?;
                SBody::Test { body: self.expr()? }
            }
            "rule" => {
                self.open_paren("(")?;
                let mut vars = Vec::new();
                while !self.sym(")") {
                    let (n, sp) = self.name()?;
                    vars.push(SBinder { name: n, ty: None, usage: Usage::Many, implicit: false, span: sp });
                    if !self.sym(")") {
                        self.expect_sym(",")?;
                    }
                }
                self.close_paren(")")?;
                let weight = if self.at_("weight") {
                    self.bump();
                    self.open_paren("(")?;
                    let w = self.expr()?;
                    self.close_paren(")")?;
                    Some(w)
                } else {
                    None
                };
                self.expect_sym("=")?;
                let head = self.expr()?;
                let mut premises = Vec::new();
                if self.sym(":-") {
                    self.bump();
                    premises.push(self.expr()?);
                    while self.sym(",") {
                        self.bump();
                        premises.push(self.expr()?);
                    }
                }
                SBody::Rule { vars, weight, head, premises }
            }
            other => return Err(Diagnostic::error(start, format!("unknown definition keyword `{other}`"))),
        };
        let end = self.toks[self.at.saturating_sub(1)].span;
        Ok(SItem { name, name_span, body, span: start.to(end) })
    }

    fn param(&mut self) -> R<SBinder> {
        let start = self.span();
        let usage = if self.at_("erased") {
            self.bump();
            Usage::Zero
        } else if self.at_("linear") {
            self.bump();
            Usage::One
        } else {
            Usage::Many
        };
        let implicit = self.sym("{");
        if implicit {
            self.open_paren("{")?;
        }
        let (name, _) = self.name()?;
        let ty = if self.sym("::") || self.sym(":") { Some(Box::new(self.ann()?)) } else { None };
        if implicit {
            self.close_paren("}")?;
        }
        let usage = if implicit && usage == Usage::Many { Usage::Zero } else { usage };
        Ok(SBinder { name, ty, usage, implicit, span: start.to(self.toks[self.at - 1].span) })
    }

    /// A body: `{ expr }`, or `expr … end` (Julia), allowing newlines.
    fn block(&mut self) -> R<SExpr> {
        if self.sym("{") {
            self.bump();
            self.skip_nl();
            let e = self.expr()?;
            self.skip_separators();
            self.expect_sym("}")?;
            Ok(e)
        } else if self.sym("=") {
            self.bump();
            self.expr()
        } else {
            self.skip_nl();
            let e = self.expr()?;
            self.skip_separators();
            self.expect_kw("end")?;
            Ok(e)
        }
    }

    /// `name :: T` lines, in `{ … }` (comma or newline separated) or until `end`.
    fn decl_block(&mut self) -> R<Vec<(String, SExpr)>> {
        let braces = self.sym("{");
        if braces {
            self.bump();
        }
        let mut out = Vec::new();
        loop {
            self.skip_separators();
            while self.sym(",") {
                self.bump();
                self.skip_separators();
            }
            if braces && self.sym("}") {
                self.bump();
                break;
            }
            if !braces && self.kw("end") {
                self.bump();
                break;
            }
            let (n, _) = self.name()?;
            out.push((n, self.ann()?));
        }
        Ok(out)
    }

    // ---- expressions ----

    /// Any depth: one `deep::grow` per level (SP-015). The bound is the
    /// node budget in `atom`, not nesting.
    fn expr(&mut self) -> R<SExpr> {
        sophia_hash::deep::grow(|| self.arrow())
    }

    fn arrow(&mut self) -> R<SExpr> {
        let lhs = self.cmp()?;
        if self.sym("->") || self.sym("↦") {
            self.bump();
            let body = self.expr()?;
            let b = to_binder(&lhs, false)?;
            let span = lhs.span.to(body.span);
            return Ok(SExpr { kind: SKind::Lam(b, Box::new(body)), span });
        }
        if self.sym("-->") || self.sym("→") {
            self.bump();
            let cod = self.expr()?;
            let b = to_binder(&lhs, true)?;
            let span = lhs.span.to(cod.span);
            return Ok(SExpr { kind: SKind::Pi(b, Box::new(cod)), span });
        }
        Ok(lhs)
    }

    fn cmp(&mut self) -> R<SExpr> {
        let l = self.add()?;
        for op in ["==", "!=", "<=", ">=", "<", ">", "≤", "≥", "≠"] {
            if self.sym(op) {
                self.bump();
                let r = self.add()?;
                let op = match op {
                    "≤" => "<=",
                    "≥" => ">=",
                    "≠" => "!=",
                    o => o,
                };
                let span = l.span.to(r.span);
                return Ok(SExpr { kind: SKind::BinOp(op.into(), Box::new(l), Box::new(r)), span });
            }
        }
        Ok(l)
    }

    fn add(&mut self) -> R<SExpr> {
        let mut l = self.mul()?;
        while self.sym("+") || self.sym("-") {
            let Tok::Sym(op) = self.bump().tok else { unreachable!() };
            let r = self.mul()?;
            let span = l.span.to(r.span);
            l = SExpr { kind: SKind::BinOp(op.into(), Box::new(l), Box::new(r)), span };
        }
        Ok(l)
    }

    fn mul(&mut self) -> R<SExpr> {
        let mut l = self.unary()?;
        while self.sym("*") || self.sym("/") {
            let Tok::Sym(op) = self.bump().tok else { unreachable!() };
            let r = self.unary()?;
            let span = l.span.to(r.span);
            l = SExpr { kind: SKind::BinOp(op.into(), Box::new(l), Box::new(r)), span };
        }
        Ok(l)
    }

    fn unary(&mut self) -> R<SExpr> {
        if self.sym("-") {
            let sp = self.bump().span;
            return match self.tok().clone() {
                Tok::Int(v) => {
                    let s = self.bump().span;
                    Ok(SExpr { kind: SKind::Int(v.wrapping_neg()), span: sp.to(s) })
                }
                Tok::Float(bits) => {
                    let s = self.bump().span;
                    Ok(SExpr { kind: SKind::Float((-f64::from_bits(bits)).to_bits()), span: sp.to(s) })
                }
                Tok::Ident(i) if i == "Inf" => {
                    let s = self.bump().span;
                    Ok(SExpr { kind: SKind::Float(f64::NEG_INFINITY.to_bits()), span: sp.to(s) })
                }
                _ => self.err("expected a number after unary `-`"),
            };
        }
        self.postfix()
    }

    fn postfix(&mut self) -> R<SExpr> {
        let mut e = self.atom()?;
        while matches!(self.toks[self.at].tok, Tok::Sym("(")) {
            self.open_paren("(")?;
            let mut args = Vec::new();
            while !self.sym(")") {
                args.push(self.expr()?);
                if !self.sym(")") {
                    self.expect_sym(",")?;
                }
            }
            let end = self.close_paren(")")?;
            let span = e.span.to(end);
            e = SExpr { kind: SKind::Call(Box::new(e), args), span };
        }
        Ok(e)
    }

    fn atom(&mut self) -> R<SExpr> {
        self.nodes += 1;
        if self.nodes > self.limits.max_nodes {
            return self.err(format!("more than {} expression nodes", self.limits.max_nodes));
        }
        sophia_hash::deep::grow(|| self.atom_step())
    }

    fn atom_step(&mut self) -> R<SExpr> {
        let t = self.tok().clone();
        let sp = self.span();
        let mk = |kind, span| Ok(SExpr { kind, span });
        match t {
            Tok::Ident(ref s) if s == "true" || s == "false" => {
                self.bump();
                mk(SKind::Bool(s == "true"), sp)
            }
            Tok::Ident(ref s) if s == "NaN" => {
                self.bump();
                mk(SKind::Float(f64::NAN.to_bits()), sp)
            }
            Tok::Ident(ref s) if s == "Inf" => {
                self.bump();
                mk(SKind::Float(f64::INFINITY.to_bits()), sp)
            }
            Tok::Ident(ref s) if s == "Prop" => {
                self.bump();
                mk(SKind::Prop, sp)
            }
            Tok::Ident(ref s) if s == "Type" => {
                self.bump();
                // `Type{1}` only when the brace touches `Type`: `Type {` opens a block.
                if matches!(self.toks[self.at].tok, Tok::Sym("{")) && self.toks[self.at].span.start == sp.end {
                    self.open_paren("{")?;
                    let Tok::Int(l) = self.bump().tok else { return self.err("expected a universe level") };
                    let end = self.close_paren("}")?;
                    return mk(SKind::Type(l as u32), sp.to(end));
                }
                mk(SKind::Type(0), sp)
            }
            Tok::Ident(ref s) if s == "let" => self.let_(),
            Tok::Ident(ref s) if s == "match" => self.match_(),
            Tok::Ident(_) | Tok::Quoted(_) => {
                let mut path = vec![self.name()?.0];
                let mut end = sp;
                while matches!(self.toks[self.at].tok, Tok::Sym(".")) {
                    self.bump();
                    let (n, s) = self.name()?;
                    path.push(n);
                    end = s;
                }
                mk(SKind::Path(path), sp.to(end))
            }
            Tok::HashRef(h) => {
                self.bump();
                mk(SKind::HashRef(h), sp)
            }
            Tok::NominalRef(h) => {
                self.bump();
                mk(SKind::NominalRef(h), sp)
            }
            Tok::Int(v) => {
                self.bump();
                mk(SKind::Int(v), sp)
            }
            Tok::UInt(v) => {
                self.bump();
                mk(SKind::UInt(v), sp)
            }
            Tok::Float(v) => {
                self.bump();
                mk(SKind::Float(v), sp)
            }
            Tok::Str(s) => {
                self.bump();
                mk(SKind::Str(s), sp)
            }
            Tok::Char(c) => {
                self.bump();
                mk(SKind::Char(c), sp)
            }
            Tok::Sym("(") => {
                self.open_paren("(")?;
                let e = self.expr()?;
                if self.sym("::") || self.sym(":") {
                    self.bump();
                    let ty = self.expr()?;
                    let end = self.close_paren(")")?;
                    return mk(SKind::Annot(Box::new(e), Box::new(ty)), sp.to(end));
                }
                let end = self.close_paren(")")?;
                let mut e = e;
                e.span = sp.to(end);
                Ok(e)
            }
            Tok::Sym("{") => {
                self.open_paren("{")?;
                let e = self.expr()?;
                let inner = if self.sym("::") || self.sym(":") {
                    self.bump();
                    let ty = self.expr()?;
                    SExpr { span: e.span.to(ty.span), kind: SKind::Annot(Box::new(e), Box::new(ty)) }
                } else {
                    e
                };
                let end = self.close_paren("}")?;
                mk(SKind::Implicit(Box::new(inner)), sp.to(end))
            }
            Tok::Sym("?") => {
                self.bump();
                self.open_paren("(")?;
                let ty = self.expr()?;
                let end = self.close_paren(")")?;
                mk(SKind::Hole(Box::new(ty)), sp.to(end))
            }
            Tok::At(m) => {
                self.bump();
                match m.as_str() {
                    "erased" | "linear" => {
                        let inner = self.postfix()?;
                        let u = if m == "erased" { Usage::Zero } else { Usage::One };
                        let span = sp.to(inner.span);
                        mk(SKind::Usage(u, Box::new(inner)), span)
                    }
                    "prim" => {
                        self.open_paren("(")?;
                        let (n, _) = self.name()?;
                        let end = self.close_paren(")")?;
                        mk(SKind::Prim(n), sp.to(end))
                    }
                    "unsafe" => {
                        self.open_paren("(")?;
                        let (a, asp) = self.name()?;
                        let aspect = Aspect::parse(&a)
                            .ok_or_else(|| Diagnostic::error(asp, format!("unknown unsafe aspect `{a}`")))?;
                        self.expect_sym(",")?;
                        let e = self.expr()?;
                        let end = self.close_paren(")")?;
                        mk(SKind::Unsafe(aspect, Box::new(e)), sp.to(end))
                    }
                    "con" => {
                        self.open_paren("(")?;
                        let ty = self.expr()?;
                        self.expect_sym(",")?;
                        let Tok::Int(j) = self.bump().tok else { return self.err("expected a constructor index") };
                        let end = self.close_paren(")")?;
                        mk(SKind::ConIndex(Box::new(ty), j as u32), sp.to(end))
                    }
                    "member" => {
                        self.open_paren("(")?;
                        let Tok::Int(k) = self.bump().tok else { return self.err("expected a member index") };
                        let end = self.close_paren(")")?;
                        mk(SKind::Member(k as u32), sp.to(end))
                    }
                    "float" => {
                        self.open_paren("(")?;
                        let Tok::UInt(bits) = self.bump().tok else { return self.err("expected 0x… bits") };
                        let end = self.close_paren(")")?;
                        mk(SKind::Float(bits), sp.to(end))
                    }
                    other => Err(Diagnostic::error(sp, format!("unknown marker `@{other}`"))),
                }
            }
            _ => self.err("expected an expression"),
        }
    }

    fn let_(&mut self) -> R<SExpr> {
        let start = self.expect_kw("let")?;
        let lhs = self.postfix()?;
        let lhs = if self.sym("::") || self.sym(":") {
            self.bump();
            let ty = self.expr_no_eq()?;
            SExpr { span: lhs.span.to(ty.span), kind: SKind::Annot(Box::new(lhs), Box::new(ty)) }
        } else {
            lhs
        };
        let b = to_binder(&lhs, false)?;
        self.expect_sym("=")?;
        let value = self.expr()?;
        // `let x = v {` body `}` (brace profiles), when the brace is on the
        // `let`'s line; otherwise `let x = v [in]` body `end`.
        let brace = self.sym("{");
        let same_line = !self.toks[..self.at].iter().rev().take_while(|t| t.span.start >= value.span.end).any(|t| t.tok == Tok::Newline);
        if brace && same_line {
            self.bump();
            self.skip_separators();
            let body = self.expr()?;
            self.skip_separators();
            let end = self.expect_sym("}")?;
            return Ok(SExpr { kind: SKind::Let(b, Box::new(value), Box::new(body)), span: start.to(end) });
        }
        if self.kw("in") {
            self.bump();
        }
        self.skip_separators();
        let body = self.expr()?;
        self.skip_separators();
        let end = self.expect_kw("end")?;
        Ok(SExpr { kind: SKind::Let(b, Box::new(value), Box::new(body)), span: start.to(end) })
    }

    /// A type in `let x::T = v`: an expression, which cannot contain `=` anyway.
    fn expr_no_eq(&mut self) -> R<SExpr> {
        self.expr()
    }

    fn match_(&mut self) -> R<SExpr> {
        let start = self.expect_kw("match")?;
        let scrutinee = self.cmp()?;
        let data = if self.sym("::") || self.sym(":") {
            self.bump();
            Some(Box::new(self.cmp()?))
        } else {
            None
        };
        let braces = self.sym("{");
        if braces {
            self.bump();
        }
        let mut cases = Vec::new();
        loop {
            self.skip_separators();
            while self.sym(",") {
                self.bump();
                self.skip_separators();
            }
            if (braces && self.sym("}")) || (!braces && self.kw("end")) {
                break;
            }
            let cs = self.span();
            let pattern = if self.at_("ctor") {
                self.bump();
                self.open_paren("(")?;
                let Tok::Int(j) = self.bump().tok else { return self.err("expected a constructor index") };
                self.close_paren(")")?;
                Pattern::Index(j as u32)
            } else {
                Pattern::Name(self.name()?.0)
            };
            let mut binders = Vec::new();
            if matches!(self.toks[self.at].tok, Tok::Sym("(")) {
                self.open_paren("(")?;
                while !self.sym(")") {
                    let (n, s) = self.name()?;
                    binders.push(SBinder { name: n, ty: None, usage: Usage::Many, implicit: false, span: s });
                    if !self.sym(")") {
                        self.expect_sym(",")?;
                    }
                }
                self.close_paren(")")?;
            }
            self.expect_sym("=>")?;
            let body = self.expr()?;
            let span = cs.to(body.span);
            cases.push(SCase { pattern, binders, body, span });
        }
        let end = self.bump().span;
        Ok(SExpr { kind: SKind::Match { scrutinee: Box::new(scrutinee), data, cases }, span: start.to(end) })
    }
}

/// Reads an expression as a binder: `x`, `x::T`, `{x}`, `@erased x`.
/// For a Pi (`allow_anon`), any other expression is an anonymous domain.
fn to_binder(e: &SExpr, allow_anon: bool) -> R<SBinder> {
    fn go(e: &SExpr, b: &mut SBinder, top: bool) -> Result<bool, ()> {
        match &e.kind {
            SKind::Path(p) if p.len() == 1 && !top => {
                b.name = p[0].clone();
                Ok(true)
            }
            SKind::Path(p) if p.len() == 1 => {
                b.name = p[0].clone();
                Ok(true)
            }
            SKind::Annot(inner, ty) => {
                b.ty = Some(ty.clone());
                go(inner, b, false)
            }
            SKind::Implicit(inner) => {
                b.implicit = true;
                if b.usage == Usage::Many {
                    b.usage = Usage::Zero;
                }
                go(inner, b, false)
            }
            SKind::Usage(u, inner) => {
                b.usage = *u;
                go(inner, b, false)
            }
            _ => Err(()),
        }
    }
    let mut b = SBinder { name: String::new(), ty: None, usage: Usage::Many, implicit: false, span: e.span };
    let named_form = matches!(e.kind, SKind::Annot(..) | SKind::Implicit(_) | SKind::Usage(..));
    if allow_anon && !named_form {
        return Ok(SBinder { name: "_".into(), ty: Some(Box::new(e.clone())), usage: Usage::Many, implicit: false, span: e.span });
    }
    match go(e, &mut b, true) {
        Ok(_) => Ok(b),
        Err(()) => Err(Diagnostic::error(e.span, "expected a binder (`x`, `(x::T)`, `{x}` or `@erased x`)")),
    }
}
