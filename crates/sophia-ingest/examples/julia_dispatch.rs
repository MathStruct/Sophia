//! Part 3 of the Julia conformance suite (dispatch): for every module-level
//! name Sophia resolves in a definition's body, does Julia's lowered code
//! use the same function, method or binding? See `julia_dispatch.md` and
//! [[Julia Conformance]].
//!
//! Usage:
//!   julia tests/julia-semantics/calls.jl PKG [SRCDIR] > calls.tsv
//!   cargo run --release -p sophia-ingest --example julia_dispatch PKG_DIR calls.tsv [--rows]

use std::collections::BTreeMap;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let truth = std::fs::read_to_string(&args[2]).unwrap();
    let ledger = sophia_ingest::conformance::dispatch_ledger(Path::new(&args[1]), &truth).unwrap();
    println!("{} comparisons", ledger.rows.len());
    for (k, v) in ledger.tally() {
        println!("  {v:6}  {k:?}");
    }
    println!("  lines of Julia methods inside no Sophia definition: {}", ledger.julia_method_lines_outside);
    println!("  lines with several Sophia definitions (not compared): {}", ledger.julia_lines_ambiguous);
    println!("  Sophia definitions with no Julia method: {}", ledger.sophia_without_julia);
    // Part 4: what a second method for each single-definition function
    // would move (ids that contain its id).
    let mut dep = ledger.dependants.clone();
    dep.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    let direct: usize = dep.iter().map(|d| d.1).sum();
    let transitive: Vec<usize> = {
        let mut v: Vec<usize> = dep.iter().map(|d| d.2).collect();
        v.sort_unstable();
        v
    };
    let median = transitive.get(transitive.len() / 2).copied().unwrap_or(0);
    println!(
        "  functions referred to by content (one definition): {}; direct referrers {direct}; ids containing one, per function: median {median}, max {}",
        dep.len(),
        transitive.last().copied().unwrap_or(0)
    );
    for d in dep.iter().take(5) {
        println!("    {} direct {} transitive {}", d.0, d.1, d.2);
    }
    if args.iter().any(|a| a == "--rows") {
        for r in &ledger.rows {
            println!("{}:{}\t{}\t{}\t{:?}\t{}", r.file, r.line, r.caller, r.name, r.outcome, r.detail);
        }
        return;
    }
    // Examples of every outcome except the expected ones.
    let mut by: BTreeMap<_, Vec<String>> = BTreeMap::new();
    for r in &ledger.rows {
        by.entry(r.outcome).or_default().push(format!("{}:{} {} → {}: {}", r.file, r.line, r.caller, r.name, r.detail));
    }
    for (k, v) in by {
        println!("{k:?}, e.g.");
        for e in v.iter().take(6) {
            println!("    {e}");
        }
    }
}
