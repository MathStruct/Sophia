//! E-3 (B-024): Sophia's stale set by signature overlap against what changed
//! and against Julia's invalidations, for runs of `dispatch_facts.jl`.
//!
//!     cargo run -p sophia-ingest --example julia_invalidation -- run1.tsv run2.tsv …
//!     cargo run -p sophia-ingest --example julia_invalidation -- --misses run.tsv
//!
//! One row per run, then the sums. `--misses` also lists every fact a rule
//! missed, every call that changed, every call only the specificity rule's
//! over-approximation marks, and the instances on which a rule and Julia
//! disagree. See
//! julia_invalidation.md.

use std::collections::BTreeSet;

use sophia_ingest::invalidation::{Fact, FactKind, Report, Rule, Run, Score};

fn pr(s: Score) -> String {
    format!("{}/{}/{} P {:.2} R {:.2}", s.tp, s.fp, s.fn_, s.precision(), s.recall())
}

fn add(a: Score, b: Score) -> Score {
    Score { tp: a.tp + b.tp, fp: a.fp + b.fp, fn_: a.fn_ + b.fn_ }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let misses = args.first().is_some_and(|a| a == "--misses");
    if misses {
        args.remove(0);
    }
    println!("scores are tp/fp/fn, precision, recall");
    println!("run\tfacts\tchanged\toverlap\tspecific\tinstances\tjulia\tunrecorded\toverlap~julia\tspecific~julia\tchanged~julia");
    let mut sum: Option<Report> = None;
    for path in &args {
        let run = Run::parse(&std::fs::read_to_string(path).expect("read")).expect("parse");
        let r = run.report();
        let name = std::path::Path::new(path).file_stem().unwrap().to_string_lossy();
        println!(
            "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            r.facts,
            r.changed,
            pr(r.overlap),
            pr(r.specific),
            r.instances,
            r.julia,
            r.julia_unrecorded,
            pr(r.overlap_vs_julia),
            pr(r.specific_vs_julia),
            pr(r.changed_vs_julia)
        );
        if misses {
            for f in &run.facts {
                for rule in [Rule::Overlap, Rule::Specific] {
                    if f.changed() && !run.stale(f, rule) {
                        println!("  {rule:?} missed fact: {} :: {:?}", f.caller, f.sig);
                    }
                }
                if f.kind != FactKind::Own && !f.changed() && run.stale(f, Rule::Specific) {
                    println!("  Specific extra fact: {} :: {:?}", f.caller, f.sig);
                }
                if f.kind != FactKind::Own && f.changed() {
                    println!("  changed fact: {} :: {:?}", f.caller, f.sig);
                }
            }
            let julia: BTreeSet<String> = run.instances.iter().filter(|(_, l)| **l).map(|(k, _)| k.clone()).collect();
            for rule in [Rule::Overlap, Rule::Specific] {
                let sophia = run.affected(|f: &Fact| run.stale(f, rule));
                for m in julia.difference(&sophia) {
                    println!("  {rule:?}: julia only: {m}");
                }
                for m in sophia.difference(&julia) {
                    println!("  {rule:?}: sophia only: {m}");
                }
            }
        }
        sum = Some(match sum {
            None => r,
            Some(s) => Report {
                facts: s.facts + r.facts,
                changed: s.changed + r.changed,
                overlap: add(s.overlap, r.overlap),
                specific: add(s.specific, r.specific),
                instances: s.instances + r.instances,
                julia: s.julia + r.julia,
                julia_unrecorded: s.julia_unrecorded + r.julia_unrecorded,
                overlap_vs_julia: add(s.overlap_vs_julia, r.overlap_vs_julia),
                specific_vs_julia: add(s.specific_vs_julia, r.specific_vs_julia),
                changed_vs_julia: add(s.changed_vs_julia, r.changed_vs_julia),
            },
        });
    }
    if let Some(s) = sum {
        println!(
            "sum\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            s.facts,
            s.changed,
            pr(s.overlap),
            pr(s.specific),
            s.instances,
            s.julia,
            s.julia_unrecorded,
            pr(s.overlap_vs_julia),
            pr(s.specific_vs_julia),
            pr(s.changed_vs_julia)
        );
    }
}
