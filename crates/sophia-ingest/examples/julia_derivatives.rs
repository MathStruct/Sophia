//! Experiment E-5 on the fixture `tests/julia-semantics/rules`: prints, per
//! variant, which derived programs and which keys changed against `base`,
//! each key's score, and the costs. See `julia_derivatives.md`.

use std::path::Path;

use sophia_ingest::derivative::{KEYS, derivative_ledger};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/rules");
const VARIANTS: [&str; 10] = [
    "base",
    "rule_g",
    "adjoint_g",
    "adjoint_g_other",
    "nondiff_sq",
    "method_g32",
    "edit_unused",
    "pirate_g",
    "pirate_g_other",
    "pirate_sin",
];

fn main() {
    let dir = Path::new(DIR);
    let mut sources: Vec<(&str, std::path::PathBuf)> = VARIANTS.iter().map(|v| (*v, dir.join("variants").join(v))).collect();
    sources.push(("base+session", dir.join("variants/rule_g")));
    let tsv = std::fs::read_to_string(dir.join("truth.tsv")).expect("truth.tsv");
    let (o, l) = derivative_ledger("Rules", &sources, &tsv).expect("ledger");
    println!("versions: {:?}, inconsistent: {:?}", o.versions.get("base"), o.inconsistent);
    for v in VARIANTS.iter().skip(1).chain(["base+session"].iter()) {
        println!("\n{v}: program changed {:?}", l.changed("base", v));
        for k in KEYS {
            println!("  {k:?} changed {:?}", l.rekeyed(k, "base", v));
        }
    }
    for k in KEYS {
        let s = l.score(k);
        println!("\n{k:?}: {} pairs, {} unsound {:?}, {} missed", s.pairs, s.unsound.len(), s.unsound, s.missed);
    }
    println!("\nlabel\ttarget\ttype\tpieces\tcomplete\tfirst_ms\twalk_ms\tgen_ms\tcached_ms\tgradient");
    for d in &o.derived {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{}",
            d.label,
            d.target,
            d.argtype,
            d.pieces.len(),
            d.complete,
            d.ns_first as f64 / 1e6,
            d.ns_walk as f64 / 1e6,
            d.ns_gen as f64 / 1e6,
            d.ns_cached as f64 / 1e6,
            d.gradient
        );
    }
    for r in l.rows.iter().filter(|r| matches!(r.label.as_str(), "base" | "adjoint_g" | "nondiff_sq") && r.argtype == "Float64") {
        println!("closure {} {}: {:?}, package rules {}", r.label, r.target, r.closure, r.package_rules);
    }
    println!("calls: {:?}", o.calls);
}
