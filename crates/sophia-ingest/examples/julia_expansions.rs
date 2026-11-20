//! Experiment E-4 (macro expansions as transformation records) on a real
//! package: Julia's expansion of every module-level form that calls a macro,
//! spliced into the package and ingested; which methods the expansions
//! recover, what they cost, and whether they are reproducible from their
//! recorded inputs. See `julia_expansions.md` and [[Semantic Boundary
//! Experiments]] §E-4.
//!
//! Usage:
//!   julia tests/julia-semantics/expansions.jl PKG SRCDIR > expansions.tsv
//!   julia tests/julia-semantics/macros.jl PKG SRCDIR > macros.tsv     # optional
//!   cargo run --release -p sophia-ingest --example julia_expansions PKG_DIR expansions.tsv [macros.tsv] [--rows]

use std::collections::BTreeMap;
use std::path::Path;

use sophia_ingest::expansion::MethodOutcome;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let tsv = std::fs::read_to_string(&args[2]).unwrap();
    let t = std::time::Instant::now();
    let l = sophia_ingest::expansion::expansion_ledger(Path::new(&args[1]), &tsv).unwrap();
    let ingest = t.elapsed();
    if args.iter().any(|a| a == "--rows") {
        for r in &l.rows {
            println!("{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{}", r.file, r.line, r.k, r.by, r.owner, r.sig, r.outcome, r.detail);
        }
        return;
    }
    println!("julia {}", l.julia);
    if let Some(m) = args.get(3).filter(|a| !a.starts_with("--")) {
        let f = sophia_ingest::expansion::faithful(&tsv, &std::fs::read_to_string(m).unwrap());
        let differ: Vec<_> = f.iter().filter(|x| !x.2).collect();
        println!("forms whose records made the form's methods: {} of {}", f.len() - differ.len(), f.len());
        for x in differ.iter().take(5) {
            println!("  differs: {}:{}", x.0, x.1);
        }
    }
    println!("{} forms compared ({} records), {} not found, {} on shared lines", l.forms, l.records, l.unmatched.len(), l.ambiguous.len());
    println!("records not stable: {}; not parsed back or failed: {}", l.unstable.len(), l.not_ok.len());
    for x in l.not_ok.iter().take(5) {
        println!("  {x:?}");
    }
    println!("methods: {}", l.rows.len());
    for (k, v) in l.tally() {
        println!("  {v:6}  {k:?}");
    }
    println!("bindings made by records: {} ({} named by a definition of the expansion)", l.bindings, l.bindings_covered);
    println!(
        "definitions: as written {} ({} templates); in the expansions {} ({} refused)",
        l.definitions_before, l.templates_before, l.definitions_after, l.refused_after
    );
    println!("bytes: as written {}, expansions {}", l.bytes_before, l.bytes_after);
    println!(
        "julia: expansion {:.1} ms, evaluation {:.1} ms; sophia (two ingests) {:.1} ms",
        l.ns_expand as f64 / 1e6,
        l.ns_eval as f64 / 1e6,
        ingest.as_secs_f64() * 1e3
    );
    let addresses: std::collections::HashSet<_> = l.transformations.iter().map(|x| x.address).collect();
    println!("transformation records: {} ({} distinct addresses)", l.transformations.len(), addresses.len());
    // Equal addresses must give equal outputs (E-4's failure condition).
    println!("records whose address is shared with a different text: {}", l.clashes.len());
    for x in l.clashes.iter().take(4) {
        println!("  {}:{} k={}", x.0, x.1, x.2);
    }
    let mut by: BTreeMap<(String, String), usize> = BTreeMap::new();
    for r in &l.rows {
        *by.entry((r.by.clone(), format!("{:?}", r.outcome))).or_default() += 1;
    }
    println!("methods by macro and outcome:");
    for ((m, o), n) in by {
        println!("  {n:6}  {m}  {o}");
    }
    let open: Vec<_> = l.rows.iter().filter(|r| matches!(r.outcome, MethodOutcome::Open | MethodOutcome::RecoveredRefused)).collect();
    println!("open or refused (first 15 of {}):", open.len());
    for r in open.iter().take(15) {
        println!("  {}:{} k={} {} {}{} {:?} {}", r.file, r.line, r.k, r.by, r.owner, r.sig, r.outcome, r.detail);
    }
}
