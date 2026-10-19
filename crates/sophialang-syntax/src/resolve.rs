//! Name resolution, shared by the parser (elaboration) and the printer, so
//! that the printer only ever emits a name that resolves back to the same
//! thing. See `resolve.md`.

use sophia_hash::{Digest, NominalId};
use sophialang_core::samples::builtin;

/// What a dotted path names in the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found {
    Content(Digest),
    Nominal(NominalId),
}

/// The store, as the syntax layer sees it.
pub trait Env: Sync {
    /// An absolute dotted path, exactly.
    fn lookup(&self, path: &str) -> Option<Found>;
    /// The constructor names of a stored `data` definition, by index.
    fn constructors(&self, data: &Digest) -> Option<Vec<String>>;
    /// Every name of a target, absolute.
    fn names_of(&self, target: &Digest) -> Vec<String>;
}

/// An environment with nothing in it.
pub struct EmptyEnv;

impl Env for EmptyEnv {
    fn lookup(&self, _: &str) -> Option<Found> {
        None
    }
    fn constructors(&self, _: &Digest) -> Option<Vec<String>> {
        None
    }
    fn names_of(&self, _: &Digest) -> Vec<String> {
        Vec::new()
    }
}

/// Built-in nominal entities with intrinsic names (types and operators).
pub const BUILTINS: [&str; 22] = [
    "Int64", "UInt64", "Float64", "Bool", "Char", "String", "Ptr", "Any", "Nothing", "+", "-", "*", "/", "==", "!=",
    "<", "<=", ">", ">=", "getindex", "length", "map",
];

/// Infix operators and their precedence (higher binds tighter).
pub fn infix_prec(op: &str) -> Option<u8> {
    Some(match op {
        "==" | "!=" | "<" | "<=" | ">" | ">=" => 1,
        "+" | "-" => 2,
        "*" | "/" => 3,
        _ => return None,
    })
}

pub fn builtin_named(name: &str) -> Option<NominalId> {
    let bare = name.trim_matches('`');
    BUILTINS.contains(&bare).then(|| builtin(bare))
}

pub fn builtin_name(id: &NominalId) -> Option<&'static str> {
    BUILTINS.iter().copied().find(|n| builtin(n) == *id)
}

/// The candidate absolute paths for `path` written inside namespace `ns`:
/// innermost namespace first, then outwards, then the path as written.
pub fn candidates(ns: &str, path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut prefix = ns.to_string();
    while !prefix.is_empty() {
        out.push(format!("{prefix}.{path}"));
        prefix = match prefix.rfind('.') {
            Some(i) => prefix[..i].to_string(),
            None => String::new(),
        };
    }
    out.push(path.to_string());
    out
}

/// Resolves a path in a namespace, through the store then the built-ins.
pub fn resolve_global(env: &dyn Env, ns: &str, path: &str) -> Option<Found> {
    candidates(ns, path).iter().find_map(|c| env.lookup(c)).or_else(|| builtin_named(path).map(Found::Nominal))
}

/// Shortest ways to write `abs` from inside `ns`: the suffixes of the path,
/// shortest first, ending with the absolute path.
pub fn spellings(abs: &str) -> Vec<String> {
    let segs = crate::split_path(abs);
    (0..segs.len()).rev().map(|i| segs[i..].join(".")).collect()
}

pub const KEYWORDS: [&str; 20] = [
    "function", "fn", "def", "end", "const", "data", "layout", "relation", "rule", "test", "postulate", "let", "in",
    "match", "true", "false", "Type", "Prop", "NaN", "Inf",
];
