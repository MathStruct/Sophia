//! sophialang's surface syntax: rendering profiles, a printer per profile and
//! one parser for all editable profiles (SPEC-005). See
//! `sophialang_syntax.md`.

pub mod ast;
pub mod diagnostic;
pub mod elab;
pub mod lexer;
pub mod parser;
pub mod print;
pub mod profile;
pub mod resolve;

pub use diagnostic::{Diagnostic, Severity, Span};
pub use elab::Item;
pub use lexer::Limits;
pub use print::{ToPrint, print};
pub use profile::Profile;
pub use resolve::{EmptyEnv, Env, Found};

/// Splits a dotted path on dots outside backquotes.
pub fn split_path(path: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut start, mut quoted) = (0, false);
    for (i, c) in path.char_indices() {
        match c {
            '`' => quoted = !quoted,
            '.' if !quoted => {
                out.push(&path[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&path[start..]);
    out
}

/// Parses and elaborates a text written in namespace `ns`. On success,
/// returns the items and any warnings (such as unresolved names); on
/// failure, the errors. Nothing is stored either way.
///
/// Parsing and elaborating are recursive and run at any depth through
/// explicit stacks (SP-015), on the caller's thread: no parser thread, which
/// wasm would not have. A panic is reported as an error, not propagated.
pub fn parse_text(env: &dyn Env, ns: &str, text: &str, limits: Limits) -> Result<(Vec<Item>, Vec<Diagnostic>), Vec<Diagnostic>> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let items = parser::parse(text, limits).map_err(|d| vec![d])?;
        elab::elaborate(env, ns, &items).map_err(|d| vec![d])
    }))
    .unwrap_or_else(|_| Err(vec![Diagnostic::error(Span::default(), "the parser panicked")]))
}
