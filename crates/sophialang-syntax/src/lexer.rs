//! Tokens of sophialang's surface syntax, for every profile. See `lexer.md`.

use crate::diagnostic::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    /// A backquoted operator name: `` `+` ``.
    Quoted(String),
    Int(i64),
    /// `0x…`: an unsigned 64-bit literal.
    UInt(u64),
    Float(u64),
    Str(String),
    Char(char),
    /// `#` followed by 64 hex digits: a content id.
    HashRef(String),
    /// `##` followed by 64 hex digits: a nominal id.
    NominalRef(String),
    /// `@name`: a macro-like marker (`@prim`, `@unsafe`, `@erased`, …).
    At(String),
    Sym(&'static str),
    Newline,
    Eof,
}

/// How a token is named in a diagnostic: as written, in backquotes.
impl std::fmt::Display for Tok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tok::Ident(s) => write!(f, "`{s}`"),
            Tok::Quoted(s) => write!(f, "`{s}` (quoted)"),
            Tok::Int(i) => write!(f, "`{i}`"),
            Tok::UInt(u) => write!(f, "`0x{u:x}`"),
            Tok::Float(b) => write!(f, "`{}`", f64::from_bits(*b)),
            Tok::Str(s) => write!(f, "{s:?}"),
            Tok::Char(c) => write!(f, "{c:?}"),
            Tok::HashRef(h) => write!(f, "`#{}…`", &h[..h.len().min(12)]),
            Tok::NominalRef(h) => write!(f, "`##{}…`", &h[..h.len().min(12)]),
            Tok::At(s) => write!(f, "`@{s}`"),
            Tok::Sym(s) => write!(f, "`{s}`"),
            Tok::Newline => f.write_str("end of line"),
            Tok::Eof => f.write_str("end of text"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

const SYMS: [&str; 30] = [
    "-->", "::", "->", "=>", ":-", "==", "!=", "<=", ">=", "→", "↦", "≤", "≥", "≠", "(", ")", "{", "}", "[", "]",
    ",", ";", ":", "=", "+", "-", "*", "/", "<", ">",
];

/// Resource limits for a parse (SR-50: bounded parsers). Size, never
/// nesting: depth costs memory, not a refusal (SP-015, SR-145).
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_bytes: usize,
    /// Expression nodes (atoms) in one text.
    pub max_nodes: usize,
    pub max_definitions: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_bytes: 512 * 1024, max_nodes: 1 << 20, max_definitions: 4096 }
    }
}

pub fn lex(src: &str) -> Result<Vec<Token>, Diagnostic> {
    let mut out = Vec::new();
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = src[i..].chars().next().unwrap();
        let start = i;
        if c == '\n' {
            i += 1;
            if !matches!(out.last(), Some(Token { tok: Tok::Newline, .. }) | None) {
                out.push(Token { tok: Tok::Newline, span: Span::new(start, i) });
            }
            continue;
        }
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        if c == '#' && !src[i..].starts_with("##") && !is_hex_run(&src[i + 1..], 64) {
            // A comment runs to the end of the line.
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if src[i..].starts_with("//") {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if src[i..].starts_with("##") && is_hex_run(&src[i + 2..], 64) {
            out.push(Token { tok: Tok::NominalRef(src[i + 2..i + 66].to_string()), span: Span::new(i, i + 66) });
            i += 66;
            continue;
        }
        if c == '#' && is_hex_run(&src[i + 1..], 64) {
            out.push(Token { tok: Tok::HashRef(src[i + 1..i + 65].to_string()), span: Span::new(i, i + 65) });
            i += 65;
            continue;
        }
        if c == '@' {
            let mut j = i + 1;
            while j < b.len() && is_ident_char(src[j..].chars().next().unwrap()) {
                j += src[j..].chars().next().unwrap().len_utf8();
            }
            out.push(Token { tok: Tok::At(src[i + 1..j].to_string()), span: Span::new(i, j) });
            i = j;
            continue;
        }
        if c == '`' {
            let Some(end) = src[i + 1..].find('`') else {
                return Err(Diagnostic::error(Span::new(i, i + 1), "unterminated backquote"));
            };
            out.push(Token { tok: Tok::Quoted(src[i + 1..i + 1 + end].to_string()), span: Span::new(i, i + end + 2) });
            i += end + 2;
            continue;
        }
        if c == '"' {
            let (s, j) = string_lit(src, i)?;
            out.push(Token { tok: Tok::Str(s), span: Span::new(i, j) });
            i = j;
            continue;
        }
        if c == '\'' {
            let (s, j) = char_lit(src, i)?;
            out.push(Token { tok: Tok::Char(s), span: Span::new(i, j) });
            i = j;
            continue;
        }
        if c.is_ascii_digit() {
            let (t, j) = number(src, i)?;
            out.push(Token { tok: t, span: Span::new(i, j) });
            i = j;
            continue;
        }
        if is_ident_start(c) {
            let mut j = i;
            while j < b.len() {
                let d = src[j..].chars().next().unwrap();
                if !is_ident_char(d) {
                    break;
                }
                j += d.len_utf8();
            }
            out.push(Token { tok: Tok::Ident(src[i..j].to_string()), span: Span::new(i, j) });
            i = j;
            continue;
        }
        if let Some(s) = SYMS.iter().find(|s| src[i..].starts_with(**s)) {
            out.push(Token { tok: Tok::Sym(s), span: Span::new(i, i + s.len()) });
            i += s.len();
            continue;
        }
        if c == '?' {
            out.push(Token { tok: Tok::Sym("?"), span: Span::new(i, i + 1) });
            i += 1;
            continue;
        }
        if c == '.' {
            out.push(Token { tok: Tok::Sym("."), span: Span::new(i, i + 1) });
            i += 1;
            continue;
        }
        return Err(Diagnostic::error(Span::new(i, i + c.len_utf8()), format!("unexpected character {c:?}")));
    }
    out.push(Token { tok: Tok::Eof, span: Span::new(b.len(), b.len()) });
    Ok(out)
}

pub fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

pub fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '!'
}

fn is_hex_run(s: &str, n: usize) -> bool {
    s.len() >= n && s.as_bytes()[..n].iter().all(|c| c.is_ascii_hexdigit())
        && !s.as_bytes().get(n).is_some_and(|c| c.is_ascii_alphanumeric())
}

fn number(src: &str, i: usize) -> Result<(Tok, usize), Diagnostic> {
    let b = src.as_bytes();
    if src[i..].starts_with("0x") {
        let mut j = i + 2;
        while j < b.len() && b[j].is_ascii_hexdigit() {
            j += 1;
        }
        let v = u64::from_str_radix(&src[i + 2..j], 16)
            .map_err(|_| Diagnostic::error(Span::new(i, j), "hexadecimal literal out of range"))?;
        return Ok((Tok::UInt(v), j));
    }
    let mut j = i;
    let mut float = false;
    while j < b.len() && b[j].is_ascii_digit() {
        j += 1;
    }
    if j + 1 < b.len() && b[j] == b'.' && b[j + 1].is_ascii_digit() {
        float = true;
        j += 1;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
    }
    if j < b.len() && (b[j] == b'e' || b[j] == b'E') {
        let mut k = j + 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            k += 1;
        }
        if k < b.len() && b[k].is_ascii_digit() {
            float = true;
            j = k;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
        }
    }
    let text = &src[i..j];
    if float {
        let v: f64 = text.parse().map_err(|_| Diagnostic::error(Span::new(i, j), "bad float literal"))?;
        Ok((Tok::Float(v.to_bits()), j))
    } else {
        let v: i64 = text.parse().map_err(|_| Diagnostic::error(Span::new(i, j), "integer literal out of range"))?;
        Ok((Tok::Int(v), j))
    }
}

fn escape(src: &str, j: usize, quote: char) -> Result<(char, usize), Diagnostic> {
    let rest = &src[j..];
    let mut it = rest.chars();
    let c = it.next().ok_or_else(|| Diagnostic::error(Span::new(j, j), "unterminated literal"))?;
    if c != '\\' {
        return Ok((c, j + c.len_utf8()));
    }
    let e = it.next().ok_or_else(|| Diagnostic::error(Span::new(j, j + 1), "unterminated escape"))?;
    Ok(match e {
        'n' => ('\n', j + 2),
        't' => ('\t', j + 2),
        'r' => ('\r', j + 2),
        '0' => ('\0', j + 2),
        '\\' => ('\\', j + 2),
        '"' => ('"', j + 2),
        '\'' => ('\'', j + 2),
        'u' => {
            let body = rest.strip_prefix("\\u{").ok_or_else(|| Diagnostic::error(Span::new(j, j + 2), "expected \\u{…}"))?;
            let end = body.find('}').ok_or_else(|| Diagnostic::error(Span::new(j, j + 2), "unterminated \\u{…}"))?;
            let v = u32::from_str_radix(&body[..end], 16)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| Diagnostic::error(Span::new(j, j + 4 + end), "bad unicode escape"))?;
            (v, j + 4 + end)
        }
        other if other == quote => (other, j + 2),
        other => return Err(Diagnostic::error(Span::new(j, j + 2), format!("unknown escape \\{other}"))),
    })
}

fn string_lit(src: &str, i: usize) -> Result<(String, usize), Diagnostic> {
    let mut j = i + 1;
    let mut s = String::new();
    loop {
        if j >= src.len() {
            return Err(Diagnostic::error(Span::new(i, j), "unterminated string"));
        }
        if src[j..].starts_with('"') {
            return Ok((s, j + 1));
        }
        let (c, k) = escape(src, j, '"')?;
        s.push(c);
        j = k;
    }
}

fn char_lit(src: &str, i: usize) -> Result<(char, usize), Diagnostic> {
    let (c, j) = escape(src, i + 1, '\'')?;
    if !src[j..].starts_with('\'') {
        return Err(Diagnostic::error(Span::new(i, j), "unterminated character literal"));
    }
    Ok((c, j + 1))
}

/// Escapes a string for printing.
pub fn quote_str(s: &str, q: char) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push(q);
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            '\\' => out.push_str("\\\\"),
            c if c == q => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push(q);
    out
}
