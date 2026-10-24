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

    /// An expression. The grammar (arrow > comparison > sum > product >
    /// unary > postfix > atom, with `let`, `match` and brackets nesting
    /// expressions) is run as a machine with an explicit stack of
    /// continuations instead of recursive calls (SP-015): any depth on every
    /// target, wasm included. The bound is the node budget, not nesting.
    fn expr(&mut self) -> R<SExpr> {
        self.run(Goal::Expr)
    }

    fn run(&mut self, goal: Goal) -> R<SExpr> {
        let mut frames: Vec<Frame> = Vec::new();
        let mut call = Some(goal);
        let mut ret: Option<SExpr> = None;
        loop {
            if let Some(g) = call.take() {
                match self.start(g, &mut frames)? {
                    Start::Value(v) => ret = Some(v),
                    Start::Call(g) => call = Some(g),
                }
                continue;
            }
            let v = ret.take().expect("a value");
            let Some(f) = frames.pop() else { return Ok(v) };
            match self.resume(f, v, &mut frames)? {
                Start::Value(v) => ret = Some(v),
                Start::Call(g) => call = Some(g),
            }
        }
    }

    /// Starts a grammar rule: a value at once, or frames and a first call.
    fn start(&mut self, g: Goal, frames: &mut Vec<Frame>) -> R<Start> {
        let call = |frames: &mut Vec<Frame>, f: Frame, g: Goal| {
            frames.push(f);
            Ok(Start::Call(g))
        };
        match g {
            Goal::Expr => call(frames, Frame::Arrow, Goal::Cmp),
            Goal::Cmp => call(frames, Frame::Cmp, Goal::Add),
            Goal::Add => call(frames, Frame::AddLoop, Goal::Mul),
            Goal::Mul => call(frames, Frame::MulLoop, Goal::Unary),
            Goal::Unary => {
                if !self.sym("-") {
                    return Ok(Start::Call(Goal::Postfix));
                }
                let sp = self.bump().span;
                let v = match self.tok().clone() {
                    Tok::Int(v) => {
                        let s = self.bump().span;
                        SExpr { kind: SKind::Int(v.wrapping_neg()), span: sp.to(s) }
                    }
                    Tok::Float(bits) => {
                        let s = self.bump().span;
                        SExpr { kind: SKind::Float((-f64::from_bits(bits)).to_bits()), span: sp.to(s) }
                    }
                    Tok::Ident(i) if i == "Inf" => {
                        let s = self.bump().span;
                        SExpr { kind: SKind::Float(f64::NEG_INFINITY.to_bits()), span: sp.to(s) }
                    }
                    _ => return self.err("expected a number after unary `-`"),
                };
                Ok(Start::Value(v))
            }
            Goal::Postfix => call(frames, Frame::PostfixLoop, Goal::Atom),
            Goal::Atom => self.atom(frames),
        }
    }

    fn atom(&mut self, frames: &mut Vec<Frame>) -> R<Start> {
        self.nodes += 1;
        if self.nodes > self.limits.max_nodes {
            return self.err(format!("more than {} expression nodes", self.limits.max_nodes));
        }
        let t = self.tok().clone();
        let sp = self.span();
        let val = |kind, span| Ok(Start::Value(SExpr { kind, span }));
        let call = |frames: &mut Vec<Frame>, f: Frame, g: Goal| {
            frames.push(f);
            Ok(Start::Call(g))
        };
        match t {
            Tok::Ident(ref s) if s == "true" || s == "false" => {
                self.bump();
                val(SKind::Bool(s == "true"), sp)
            }
            Tok::Ident(ref s) if s == "NaN" => {
                self.bump();
                val(SKind::Float(f64::NAN.to_bits()), sp)
            }
            Tok::Ident(ref s) if s == "Inf" => {
                self.bump();
                val(SKind::Float(f64::INFINITY.to_bits()), sp)
            }
            Tok::Ident(ref s) if s == "Prop" => {
                self.bump();
                val(SKind::Prop, sp)
            }
            Tok::Ident(ref s) if s == "Type" => {
                self.bump();
                // `Type{1}` only when the brace touches `Type`: `Type {` opens a block.
                if matches!(self.toks[self.at].tok, Tok::Sym("{")) && self.toks[self.at].span.start == sp.end {
                    self.open_paren("{")?;
                    let Tok::Int(l) = self.bump().tok else { return self.err("expected a universe level") };
                    let end = self.close_paren("}")?;
                    return val(SKind::Type(l as u32), sp.to(end));
                }
                val(SKind::Type(0), sp)
            }
            Tok::Ident(ref s) if s == "let" => {
                let start = self.expect_kw("let")?;
                call(frames, Frame::LetLhs(start), Goal::Postfix)
            }
            Tok::Ident(ref s) if s == "match" => {
                let start = self.expect_kw("match")?;
                call(frames, Frame::MatchScrutinee(start), Goal::Cmp)
            }
            Tok::Ident(_) | Tok::Quoted(_) => {
                let mut path = vec![self.name()?.0];
                let mut end = sp;
                while matches!(self.toks[self.at].tok, Tok::Sym(".")) {
                    self.bump();
                    let (n, s) = self.name()?;
                    path.push(n);
                    end = s;
                }
                val(SKind::Path(path), sp.to(end))
            }
            Tok::HashRef(h) => {
                self.bump();
                val(SKind::HashRef(h), sp)
            }
            Tok::NominalRef(h) => {
                self.bump();
                val(SKind::NominalRef(h), sp)
            }
            Tok::Int(v) => {
                self.bump();
                val(SKind::Int(v), sp)
            }
            Tok::UInt(v) => {
                self.bump();
                val(SKind::UInt(v), sp)
            }
            Tok::Float(v) => {
                self.bump();
                val(SKind::Float(v), sp)
            }
            Tok::Str(s) => {
                self.bump();
                val(SKind::Str(s), sp)
            }
            Tok::Char(c) => {
                self.bump();
                val(SKind::Char(c), sp)
            }
            Tok::Sym("(") => {
                self.open_paren("(")?;
                call(frames, Frame::Paren(sp), Goal::Expr)
            }
            Tok::Sym("{") => {
                self.open_paren("{")?;
                call(frames, Frame::Brace(sp), Goal::Expr)
            }
            Tok::Sym("?") => {
                self.bump();
                self.open_paren("(")?;
                call(frames, Frame::Hole(sp), Goal::Expr)
            }
            Tok::At(m) => {
                self.bump();
                match m.as_str() {
                    "erased" | "linear" => {
                        let u = if m == "erased" { Usage::Zero } else { Usage::One };
                        call(frames, Frame::Usage(sp, u), Goal::Postfix)
                    }
                    "prim" => {
                        self.open_paren("(")?;
                        let (n, _) = self.name()?;
                        let end = self.close_paren(")")?;
                        val(SKind::Prim(n), sp.to(end))
                    }
                    "unsafe" => {
                        self.open_paren("(")?;
                        let (a, asp) = self.name()?;
                        let aspect = Aspect::parse(&a)
                            .ok_or_else(|| Diagnostic::error(asp, format!("unknown unsafe aspect `{a}`")))?;
                        self.expect_sym(",")?;
                        call(frames, Frame::Unsafe(sp, aspect), Goal::Expr)
                    }
                    "con" => {
                        self.open_paren("(")?;
                        call(frames, Frame::Con(sp), Goal::Expr)
                    }
                    "member" => {
                        self.open_paren("(")?;
                        let Tok::Int(k) = self.bump().tok else { return self.err("expected a member index") };
                        let end = self.close_paren(")")?;
                        val(SKind::Member(k as u32), sp.to(end))
                    }
                    "float" => {
                        self.open_paren("(")?;
                        let Tok::UInt(bits) = self.bump().tok else { return self.err("expected 0x… bits") };
                        let end = self.close_paren(")")?;
                        val(SKind::Float(bits), sp.to(end))
                    }
                    other => Err(Diagnostic::error(sp, format!("unknown marker `@{other}`"))),
                }
            }
            _ => self.err("expected an expression"),
        }
    }

    /// Continues a rule with the value `v` its last call returned.
    fn resume(&mut self, f: Frame, v: SExpr, frames: &mut Vec<Frame>) -> R<Start> {
        let call = |frames: &mut Vec<Frame>, f: Frame, g: Goal| {
            frames.push(f);
            Ok(Start::Call(g))
        };
        let val = |kind, span| Ok(Start::Value(SExpr { kind, span }));
        match f {
            Frame::Arrow => {
                if self.sym("->") || self.sym("↦") {
                    self.bump();
                    return call(frames, Frame::Lam(v), Goal::Expr);
                }
                if self.sym("-->") || self.sym("→") {
                    self.bump();
                    return call(frames, Frame::Pi(v), Goal::Expr);
                }
                Ok(Start::Value(v))
            }
            Frame::Lam(lhs) => {
                let b = to_binder(&lhs, false)?;
                let span = lhs.span.to(v.span);
                val(SKind::Lam(b, Box::new(v)), span)
            }
            Frame::Pi(lhs) => {
                let b = to_binder(&lhs, true)?;
                let span = lhs.span.to(v.span);
                val(SKind::Pi(b, Box::new(v)), span)
            }
            Frame::Cmp => {
                for op in ["==", "!=", "<=", ">=", "<", ">", "≤", "≥", "≠"] {
                    if self.sym(op) {
                        self.bump();
                        let op = match op {
                            "≤" => "<=",
                            "≥" => ">=",
                            "≠" => "!=",
                            o => o,
                        };
                        return call(frames, Frame::BinOp(v, op), Goal::Add);
                    }
                }
                Ok(Start::Value(v))
            }
            Frame::BinOp(l, op) => {
                let span = l.span.to(v.span);
                val(SKind::BinOp(op.into(), Box::new(l), Box::new(v)), span)
            }
            Frame::AddLoop => {
                if self.sym("+") || self.sym("-") {
                    let Tok::Sym(op) = self.bump().tok else { unreachable!() };
                    return call(frames, Frame::AddR(v, op), Goal::Mul);
                }
                Ok(Start::Value(v))
            }
            Frame::AddR(l, op) => {
                let span = l.span.to(v.span);
                frames.push(Frame::AddLoop);
                val(SKind::BinOp(op.into(), Box::new(l), Box::new(v)), span)
            }
            Frame::MulLoop => {
                if self.sym("*") || self.sym("/") {
                    let Tok::Sym(op) = self.bump().tok else { unreachable!() };
                    return call(frames, Frame::MulR(v, op), Goal::Unary);
                }
                Ok(Start::Value(v))
            }
            Frame::MulR(l, op) => {
                let span = l.span.to(v.span);
                frames.push(Frame::MulLoop);
                val(SKind::BinOp(op.into(), Box::new(l), Box::new(v)), span)
            }
            Frame::PostfixLoop => {
                if !matches!(self.toks[self.at].tok, Tok::Sym("(")) {
                    return Ok(Start::Value(v));
                }
                self.open_paren("(")?;
                self.call_args(v, Vec::new(), frames)
            }
            Frame::Args(callee, mut args) => {
                args.push(v);
                if !self.sym(")") {
                    self.expect_sym(",")?;
                }
                self.call_args(callee, args, frames)
            }
            Frame::Paren(sp) => {
                if self.sym("::") || self.sym(":") {
                    self.bump();
                    return call(frames, Frame::ParenAnnot(sp, v), Goal::Expr);
                }
                let end = self.close_paren(")")?;
                let mut e = v;
                e.span = sp.to(end);
                Ok(Start::Value(e))
            }
            Frame::ParenAnnot(sp, e) => {
                let end = self.close_paren(")")?;
                val(SKind::Annot(Box::new(e), Box::new(v)), sp.to(end))
            }
            Frame::Brace(sp) => {
                if self.sym("::") || self.sym(":") {
                    self.bump();
                    return call(frames, Frame::BraceAnnot(sp, v), Goal::Expr);
                }
                let end = self.close_paren("}")?;
                val(SKind::Implicit(Box::new(v)), sp.to(end))
            }
            Frame::BraceAnnot(sp, e) => {
                let inner = SExpr { span: e.span.to(v.span), kind: SKind::Annot(Box::new(e), Box::new(v)) };
                let end = self.close_paren("}")?;
                val(SKind::Implicit(Box::new(inner)), sp.to(end))
            }
            Frame::Hole(sp) => {
                let end = self.close_paren(")")?;
                val(SKind::Hole(Box::new(v)), sp.to(end))
            }
            Frame::Usage(sp, u) => {
                let span = sp.to(v.span);
                val(SKind::Usage(u, Box::new(v)), span)
            }
            Frame::Unsafe(sp, aspect) => {
                let end = self.close_paren(")")?;
                val(SKind::Unsafe(aspect, Box::new(v)), sp.to(end))
            }
            Frame::Con(sp) => {
                self.expect_sym(",")?;
                let Tok::Int(j) = self.bump().tok else { return self.err("expected a constructor index") };
                let end = self.close_paren(")")?;
                val(SKind::ConIndex(Box::new(v), j as u32), sp.to(end))
            }
            Frame::LetLhs(start) => {
                if self.sym("::") || self.sym(":") {
                    self.bump();
                    // A type in `let x::T = v`: an expression, which cannot contain `=` anyway.
                    return call(frames, Frame::LetLhsType(start, v), Goal::Expr);
                }
                self.let_value(start, v, frames)
            }
            Frame::LetLhsType(start, lhs) => {
                let lhs = SExpr { span: lhs.span.to(v.span), kind: SKind::Annot(Box::new(lhs), Box::new(v)) };
                self.let_value(start, lhs, frames)
            }
            Frame::LetValue(start, b) => {
                // `let x = v {` body `}` (brace profiles), when the brace is on the
                // `let`'s line; otherwise `let x = v [in]` body `end`.
                let brace = self.sym("{");
                let same_line =
                    !self.toks[..self.at].iter().rev().take_while(|t| t.span.start >= v.span.end).any(|t| t.tok == Tok::Newline);
                if brace && same_line {
                    self.bump();
                    self.skip_separators();
                    return call(frames, Frame::LetBody(start, b, v, true), Goal::Expr);
                }
                if self.kw("in") {
                    self.bump();
                }
                self.skip_separators();
                call(frames, Frame::LetBody(start, b, v, false), Goal::Expr)
            }
            Frame::LetBody(start, b, value, braces) => {
                self.skip_separators();
                let end = if braces { self.expect_sym("}")? } else { self.expect_kw("end")? };
                val(SKind::Let(b, Box::new(value), Box::new(v)), start.to(end))
            }
            Frame::MatchScrutinee(start) => {
                if self.sym("::") || self.sym(":") {
                    self.bump();
                    return call(frames, Frame::MatchData(start, v), Goal::Cmp);
                }
                let st = MatchState::open(self, start, v, None);
                self.match_cases(st, frames)
            }
            Frame::MatchData(start, scrutinee) => {
                let st = MatchState::open(self, start, scrutinee, Some(Box::new(v)));
                self.match_cases(st, frames)
            }
            Frame::MatchCase(mut st, pattern, binders, cs) => {
                let span = cs.to(v.span);
                st.cases.push(SCase { pattern, binders, body: v, span });
                self.match_cases(st, frames)
            }
        }
    }

    /// After `(` or a `,` of a call: the next argument, or the call itself.
    fn call_args(&mut self, callee: SExpr, args: Vec<SExpr>, frames: &mut Vec<Frame>) -> R<Start> {
        if self.sym(")") {
            let end = self.close_paren(")")?;
            let span = callee.span.to(end);
            frames.push(Frame::PostfixLoop);
            return Ok(Start::Value(SExpr { kind: SKind::Call(Box::new(callee), args), span }));
        }
        frames.push(Frame::Args(callee, args));
        Ok(Start::Call(Goal::Expr))
    }

    fn let_value(&mut self, start: Span, lhs: SExpr, frames: &mut Vec<Frame>) -> R<Start> {
        let b = to_binder(&lhs, false)?;
        self.expect_sym("=")?;
        frames.push(Frame::LetValue(start, b));
        Ok(Start::Call(Goal::Expr))
    }

    /// The next case of a match, or the end of the match.
    fn match_cases(&mut self, st: MatchState, frames: &mut Vec<Frame>) -> R<Start> {
        self.skip_separators();
        while self.sym(",") {
            self.bump();
            self.skip_separators();
        }
        if (st.braces && self.sym("}")) || (!st.braces && self.kw("end")) {
            let end = self.bump().span;
            let MatchState { start, scrutinee, data, cases, .. } = st;
            return Ok(Start::Value(SExpr { kind: SKind::Match { scrutinee: Box::new(scrutinee), data, cases }, span: start.to(end) }));
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
        frames.push(Frame::MatchCase(st, pattern, binders, cs));
        Ok(Start::Call(Goal::Expr))
    }
}

/// The grammar rules the expression machine calls.
#[derive(Clone, Copy)]
enum Goal {
    Expr,
    Cmp,
    Add,
    Mul,
    Unary,
    Postfix,
    Atom,
}

/// What starting or resuming a rule gives: a value, or a rule to call.
enum Start {
    Value(SExpr),
    Call(Goal),
}

/// A rule waiting for the value of the rule it called.
enum Frame {
    Arrow,
    Lam(SExpr),
    Pi(SExpr),
    Cmp,
    BinOp(SExpr, &'static str),
    AddLoop,
    AddR(SExpr, &'static str),
    MulLoop,
    MulR(SExpr, &'static str),
    PostfixLoop,
    Args(SExpr, Vec<SExpr>),
    Paren(Span),
    ParenAnnot(Span, SExpr),
    Brace(Span),
    BraceAnnot(Span, SExpr),
    Hole(Span),
    Usage(Span, Usage),
    Unsafe(Span, Aspect),
    Con(Span),
    LetLhs(Span),
    LetLhsType(Span, SExpr),
    LetValue(Span, SBinder),
    LetBody(Span, SBinder, SExpr, bool),
    MatchScrutinee(Span),
    MatchData(Span, SExpr),
    MatchCase(MatchState, Pattern, Vec<SBinder>, Span),
}

/// A match being read: what precedes its cases, and the cases so far.
struct MatchState {
    start: Span,
    scrutinee: SExpr,
    data: Option<Box<SExpr>>,
    braces: bool,
    cases: Vec<SCase>,
}

impl MatchState {
    fn open(p: &mut Parser, start: Span, scrutinee: SExpr, data: Option<Box<SExpr>>) -> MatchState {
        let braces = p.sym("{");
        if braces {
            p.bump();
        }
        MatchState { start, scrutinee, data, braces, cases: Vec::new() }
    }
}

/// Reads an expression as a binder: `x`, `x::T`, `{x}`, `@erased x`.
/// For a Pi (`allow_anon`), any other expression is an anonymous domain.
fn to_binder(e: &SExpr, allow_anon: bool) -> R<SBinder> {
    /// Peels annotations, implicit braces and usage markers down to the
    /// name: a loop, since each form has one inner expression.
    fn go(mut e: &SExpr, b: &mut SBinder) -> Result<(), ()> {
        loop {
            match &e.kind {
                SKind::Path(p) if p.len() == 1 => {
                    b.name = p[0].clone();
                    return Ok(());
                }
                SKind::Annot(inner, ty) => {
                    b.ty = Some(ty.clone());
                    e = inner;
                }
                SKind::Implicit(inner) => {
                    b.implicit = true;
                    if b.usage == Usage::Many {
                        b.usage = Usage::Zero;
                    }
                    e = inner;
                }
                SKind::Usage(u, inner) => {
                    b.usage = *u;
                    e = inner;
                }
                _ => return Err(()),
            }
        }
    }
    let mut b = SBinder { name: String::new(), ty: None, usage: Usage::Many, implicit: false, span: e.span };
    let named_form = matches!(e.kind, SKind::Annot(..) | SKind::Implicit(_) | SKind::Usage(..));
    if allow_anon && !named_form {
        return Ok(SBinder { name: "_".into(), ty: Some(Box::new(e.clone())), usage: Usage::Many, implicit: false, span: e.span });
    }
    match go(e, &mut b) {
        Ok(()) => Ok(b),
        Err(()) => Err(Diagnostic::error(e.span, "expected a binder (`x`, `(x::T)`, `{x}` or `@erased x`)")),
    }
}
