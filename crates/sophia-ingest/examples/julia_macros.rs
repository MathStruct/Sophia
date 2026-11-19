//! Part 5 of the Julia conformance suite (macros): for every module-level
//! form that calls a macro, what Julia's form made against the definitions
//! Sophia records in it, and whether Sophia's id sees a macro around a
//! definition. See `julia_macros.md` and [[Julia Conformance]].
//!
//! Usage:
//!   julia tests/julia-semantics/macros.jl PKG SRCDIR > macros.tsv
//!   cargo run --release -p sophia-ingest --example julia_macros PKG_DIR macros.tsv [--rows]

use std::collections::BTreeMap;
use std::path::Path;

use sophia_ingest::conformance::MacroOutcome;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let truth = std::fs::read_to_string(&args[2]).unwrap();
    let ledger = sophia_ingest::conformance::macro_ledger(Path::new(&args[1]), &truth).unwrap();
    if args.iter().any(|a| a == "--rows") {
        for r in &ledger.rows {
            println!("{}\t{}\t{}\t{}\t{:?}\t{}", r.file, r.line, r.macro_name, r.name, r.outcome, r.detail);
        }
        return;
    }
    println!("{} forms compared, {} not found in Sophia's parse", ledger.forms, ledger.unmatched.len());
    for u in ledger.unmatched.iter().take(5) {
        println!("  not found: {}:{}", u.0, u.1);
    }
    println!("lines with several forms (not compared): {}", ledger.ambiguous.len());
    println!("Julia's methods made by them: {} ({} left open)", ledger.methods_seen, ledger.methods_open);
    println!("{} rows", ledger.rows.len());
    for (k, v) in ledger.tally() {
        println!("  {v:6}  {k:?}");
    }
    // Per macro and outcome.
    let mut by: BTreeMap<(String, String), usize> = BTreeMap::new();
    for r in &ledger.rows {
        *by.entry((r.macro_name.clone(), format!("{:?}", r.outcome))).or_default() += 1;
    }
    println!("by macro:");
    for ((m, o), n) in by {
        println!("  {n:6}  {m}  {o}");
    }
    // Methods left open per macro (constructors included).
    let mut open: BTreeMap<String, usize> = BTreeMap::new();
    for r in ledger.rows.iter().filter(|r| matches!(r.outcome, MacroOutcome::Open | MacroOutcome::OpenConstructors)) {
        let n: usize = r.detail.strip_prefix("julia methods: ").and_then(|d| d.split(' ').next()).and_then(|n| n.parse().ok()).unwrap_or(0);
        *open.entry(r.macro_name.clone()).or_default() += n;
    }
    println!("open methods by macro:");
    for (m, n) in open {
        println!("  {n:6}  {m}");
    }
}
