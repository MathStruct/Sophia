//! Part 7 of the Julia conformance suite (mutable globals): for each global
//! binding of a package that holds data, whether Sophia gives it a
//! definition with an id, and for each use of one in a method, whether
//! Sophia's id treats it as content, as a generic function, or as state.
//! See `julia_globals.md` and [[Julia Conformance]].
//!
//! Usage:
//!   julia tests/julia-semantics/globals.jl PKG SRCDIR > globals.tsv
//!   cargo run --release -p sophia-ingest --example julia_globals PKG_DIR globals.tsv [--rows]

use std::collections::BTreeMap;
use std::path::Path;

use sophia_ingest::conformance::GlobalOutcome;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1].starts_with('-') {
        eprintln!("usage: julia_globals PKG_DIR globals.tsv [--rows]");
        std::process::exit(2);
    }
    let truth = std::fs::read_to_string(&args[2]).unwrap();
    let l = sophia_ingest::conformance::global_ledger(Path::new(&args[1]), &truth).unwrap();
    if args.iter().any(|a| a == "--rows") {
        for b in &l.bindings {
            println!("binding\t{}\t{}\t{}\t{:?}", b.binding, b.julia, b.sophia.join(","), b.outcome);
        }
        for r in &l.rows {
            println!("use\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{}", r.file, r.line, r.caller, r.binding, r.access, r.julia, r.outcome, r.detail);
        }
        return;
    }
    println!("{} data bindings ({} malformed truth lines skipped)", l.bindings.len(), l.malformed);
    let mut by: BTreeMap<(String, String), usize> = BTreeMap::new();
    for b in &l.bindings {
        *by.entry((b.julia.clone(), format!("{:?}", b.outcome))).or_default() += 1;
    }
    for ((j, o), n) in by {
        println!("  {n:6}  {j}  {o}");
    }
    for b in l.bindings.iter().filter(|b| !matches!(b.outcome, GlobalOutcome::KnowsConstant | GlobalOutcome::KnowsState)) {
        println!("    {:?}: {} ({}) sophia [{}]", b.outcome, b.binding, b.julia, b.sophia.join(","));
    }
    println!("{} uses compared; {} in no definition of Sophia's, {} at ambiguous lines", l.rows.len(), l.outside.len(), l.ambiguous.len());
    let mut by: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for r in &l.rows {
        *by.entry((r.julia.clone(), r.access.clone(), format!("{:?}", r.outcome))).or_default() += 1;
    }
    for ((j, a, o), n) in by {
        println!("  {n:6}  {j}  {a}  {o}");
    }
    for r in l.rows.iter().filter(|r| !matches!(r.outcome, GlobalOutcome::KnowsConstant | GlobalOutcome::KnowsState)) {
        println!("    {:?}: {}:{} {} {} {} {}", r.outcome, r.file, r.line, r.caller, r.access, r.binding, r.detail);
    }
    for (f, at, b) in l.outside.iter().chain(&l.ambiguous) {
        println!("    not compared: {f}:{at} {b}");
    }
}
