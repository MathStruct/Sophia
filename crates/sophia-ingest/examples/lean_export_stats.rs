//! Measures a lean4export NDJSON file (format 3.1) before Sophia reads it:
//! declarations by kind, the size of the shared expression graph, and how
//! much of it only proofs use. Not a reader for the store; a measurement for
//! [[Lean Overview]]. See `lean_export_stats.md`.
//!
//! Usage: cargo run --release -p sophia-ingest --example lean_export_stats -- FILE.ndjson
//!
//! The export shares expressions (each node is printed once, with an index
//! `ie`, and refers to its children by index), so "size" means distinct
//! nodes. Three sets are marked from the declarations:
//! - S: reachable from any declaration's **type** (statements; inductive
//!   types, constructors and recursors included);
//! - D: reachable from the **values** of definitions and opaques (programs);
//! - P: reachable from the **values** of theorems (proof terms).
//!
//! P \ (S ∪ D) is what only proofs need: the part of the library that is
//! evidence, not statement or program (SPEC-002 §3).

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::time::Instant;

const NONE: u32 = u32::MAX;

/// The integer after `"key":` in `s`, searching from the start.
fn int_after(s: &str, key: &str) -> Option<u32> {
    let pat = format!("\"{key}\":");
    let i = s.find(&pat)? + pat.len();
    let digits: String = s[i..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Every integer after `"key":` in `s`.
fn ints_after(s: &str, key: &str) -> Vec<u32> {
    let pat = format!("\"{key}\":");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = s[from..].find(&pat) {
        let at = from + i + pat.len();
        let digits: String = s[at..].chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse() {
            out.push(n);
        }
        from = at;
    }
    out
}

/// The line's kind: its first key other than `ie`.
fn kind(line: &str) -> &str {
    let mut rest = line.strip_prefix("{\"").unwrap_or(line);
    if let Some(r) = rest.strip_prefix("ie\":") {
        let r = r.trim_start_matches(|c: char| c.is_ascii_digit());
        rest = r.strip_prefix(",\"").unwrap_or(r);
    }
    rest.split('"').next().unwrap_or("")
}

fn main() {
    let path = std::env::args().nth(1).expect("FILE.ndjson");
    let t = Instant::now();
    let f = BufReader::with_capacity(1 << 24, std::fs::File::open(&path).unwrap());
    // children[ie] = up to three child indices.
    let mut children: Vec<[u32; 3]> = Vec::new();
    let mut node_kinds: HashMap<String, u64> = HashMap::new();
    let mut decls: HashMap<&'static str, u64> = HashMap::new();
    let (mut types, mut def_values, mut thm_values) = (Vec::new(), Vec::new(), Vec::new());
    let mut thm_types: Vec<u32> = Vec::new();
    for line in f.lines() {
        let line = line.unwrap();
        let k = kind(&line);
        let ie = if line.contains("\"ie\":") && !matches!(k, "thm" | "def" | "opaque" | "axiom" | "quot" | "inductive") {
            int_after(&line, "ie")
        } else {
            None
        };
        if let Some(ie) = ie {
            let c = match k {
                "app" => [int_after(&line, "fn").unwrap_or(NONE), int_after(&line, "arg").unwrap_or(NONE), NONE],
                "forallE" | "lam" => [int_after(&line, "type").unwrap_or(NONE), int_after(&line, "body").unwrap_or(NONE), NONE],
                "letE" => [
                    int_after(&line, "type").unwrap_or(NONE),
                    int_after(&line, "value").unwrap_or(NONE),
                    int_after(&line, "body").unwrap_or(NONE),
                ],
                "proj" => [int_after(&line, "struct").unwrap_or(NONE), NONE, NONE],
                "mdata" => [int_after(&line, "expr").unwrap_or(NONE), NONE, NONE],
                _ => [NONE; 3],
            };
            let i = ie as usize;
            if children.len() <= i {
                children.resize(i + 1, [NONE; 3]);
            }
            children[i] = c;
            *node_kinds.entry(k.to_string()).or_default() += 1;
            continue;
        }
        match k {
            "thm" => {
                *decls.entry("theorem").or_default() += 1;
                let ty = int_after(&line, "type").unwrap();
                types.push(ty);
                thm_types.push(ty);
                thm_values.push(int_after(&line, "value").unwrap());
            }
            "def" | "opaque" => {
                *decls.entry(if k == "def" { "definition" } else { "opaque" }).or_default() += 1;
                types.push(int_after(&line, "type").unwrap());
                def_values.push(int_after(&line, "value").unwrap());
            }
            "axiom" | "quot" => {
                *decls.entry(if k == "axiom" { "axiom" } else { "quotient" }).or_default() += 1;
                types.push(int_after(&line, "type").unwrap());
            }
            "inductive" => {
                *decls.entry("inductive group").or_default() += 1;
                // Types of the inductive types, constructors and recursors;
                // recursor rules' right-hand sides are derived by the kernel.
                types.extend(ints_after(&line, "type"));
                types.extend(ints_after(&line, "rhs"));
            }
            _ => {}
        }
    }
    let read = t.elapsed();
    let n = children.len();
    let mark = |roots: &[u32]| -> Vec<bool> {
        let mut seen = vec![false; n];
        let mut stack: Vec<u32> = roots.to_vec();
        while let Some(x) = stack.pop() {
            let i = x as usize;
            if x == NONE || i >= n || seen[i] {
                continue;
            }
            seen[i] = true;
            stack.extend(children[i].iter().copied().filter(|&c| c != NONE));
        }
        seen
    };
    let s = mark(&types);
    let d = mark(&def_values);
    let p = mark(&thm_values);
    let count = |v: &[bool]| v.iter().filter(|b| **b).count();
    let proof_only = (0..n).filter(|&i| p[i] && !s[i] && !d[i]).count();
    let program_only = (0..n).filter(|&i| d[i] && !s[i] && !p[i]).count();
    // Theorems whose statement is the same expression as another theorem's
    // (shared node: equal up to the export's sharing, binder names included).
    let mut by_type: HashMap<u32, u32> = HashMap::new();
    for t in &thm_types {
        *by_type.entry(*t).or_default() += 1;
    }
    let repeated: u32 = by_type.values().filter(|c| **c > 1).map(|c| c - 1).sum();
    let groups = by_type.values().filter(|c| **c > 1).count();

    let mut dk: Vec<_> = decls.into_iter().collect();
    dk.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
    println!("{path}: read in {read:.1?}, analysed in {:.1?}", t.elapsed() - read);
    println!("declarations: {}", dk.iter().map(|(k, v)| format!("{v} {k}")).collect::<Vec<_>>().join(", "));
    let mut nk: Vec<_> = node_kinds.into_iter().collect();
    nk.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
    println!("expression nodes: {n} ({})", nk.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", "));
    let pct = |x: usize| 100.0 * x as f64 / n as f64;
    println!("reachable from statements (types): {} ({:.1} %)", count(&s), pct(count(&s)));
    println!("reachable from definition values:  {} ({:.1} %)", count(&d), pct(count(&d)));
    println!("reachable from proofs:             {} ({:.1} %)", count(&p), pct(count(&p)));
    println!("only proofs need:                  {proof_only} ({:.1} %)", pct(proof_only));
    println!("only definition values need:       {program_only} ({:.1} %)", pct(program_only));
    println!(
        "theorems sharing their statement expression with another: {repeated} of {} ({groups} groups)",
        thm_types.len()
    );
}
