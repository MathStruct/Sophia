//! Part 6 of the Julia conformance suite (closures and captured variables):
//! for every closure Julia's lowering made, each name in the matching
//! closure of Sophia's parse, classified by Julia (captured, the closure's
//! own, global) and by Sophia's encoding; and every call's keyword name the
//! encoding loses. See `julia_closures.md` and [[Julia Conformance]].
//!
//! Usage:
//!   julia tests/julia-semantics/closures.jl PKG SRCDIR > closures.tsv
//!   cargo run --release -p sophia-ingest --example julia_closures PKG_DIR closures.tsv [--rows]

use std::collections::BTreeMap;
use std::path::Path;

use sophia_ingest::conformance::ClosureOutcome;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1].starts_with('-') {
        eprintln!("usage: julia_closures PKG_DIR closures.tsv [--rows | --all]");
        std::process::exit(2);
    }
    let truth = std::fs::read_to_string(&args[2]).unwrap();
    let l = sophia_ingest::conformance::closure_ledger(Path::new(&args[1]), &truth).unwrap();
    if args.iter().any(|a| a == "--rows") {
        for r in &l.rows {
            println!("{}\t{}\t{}\t{}\t{}\t{:?}\t{}", r.file, r.line, r.caller, r.closure, r.name, r.outcome, r.detail);
        }
        for (f, at, def, name) in &l.keywords_unspelled {
            println!("{f}\t{at}\t{def}\t-\t{name}\tKeywordUnspelled\t-");
        }
        return;
    }
    println!("Julia's closure methods: {} ({} malformed truth lines skipped)", l.julia_closures, l.malformed);
    println!("  matched to a closure of Sophia's parse: {}", l.matched);
    println!("  in a definition Sophia refuses (left open): {}", l.refused.len());
    println!("  in no definition of Sophia's: {}", l.outside.len());
    println!("  no closure of Sophia's matches: {}", l.unmatched.len());
    println!("  several match: {}", l.ambiguous.len());
    for (f, at, c) in
        l.refused.iter().chain(&l.outside).chain(&l.unmatched).chain(&l.ambiguous).take(if args.iter().any(|a| a == "--all") {
            usize::MAX
        } else {
            10
        })
    {
        println!("    {f}:{at} {c}");
    }
    println!("{} names compared ({} names Julia does not list, not compared)", l.rows.len(), l.unlisted);
    for (k, v) in l.tally() {
        println!("  {v:6}  {k:?}");
    }
    // Julia's class against the outcome.
    let mut by: BTreeMap<(String, String), usize> = BTreeMap::new();
    for r in &l.rows {
        *by.entry((r.julia.clone(), format!("{:?}", r.outcome))).or_default() += 1;
    }
    println!("by Julia's class:");
    for ((j, o), n) in by {
        println!("  {n:6}  {j}  {o}");
    }
    for r in l
        .rows
        .iter()
        .filter(|r| !matches!(r.outcome, ClosureOutcome::KnowsCaptured | ClosureOutcome::KnowsOwn | ClosureOutcome::KnowsGlobal))
    {
        println!("  {:?}: {}:{} {} {} `{}`: {}", r.outcome, r.file, r.line, r.caller, r.closure, r.name, r.detail);
    }
    // Keyword names lost, per name.
    let mut kw: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, _, _, n) in &l.keywords_unspelled {
        *kw.entry(n.as_str()).or_default() += 1;
    }
    let defs: std::collections::BTreeSet<_> = l.keywords_unspelled.iter().map(|(f, _, d, _)| (f, d)).collect();
    println!("keyword arguments whose name is in no id: {} in {} definitions", l.keywords_unspelled.len(), defs.len());
    let mut top: Vec<_> = kw.into_iter().collect();
    top.sort_by_key(|t| std::cmp::Reverse(t.1));
    for (n, c) in top.iter().take(10) {
        println!("  {c:6}  {n}");
    }
}
