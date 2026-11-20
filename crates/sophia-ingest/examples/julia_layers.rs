//! Experiment E-6 on the E-5 fixture's `base` variant: per gradient, the
//! size of the derived program at each layer, the agreement, the times; then
//! what the layers share and what a store holding them grows by. See
//! `julia_layers.md`.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use sophia_ingest::layers::{layer_records, split_module, store_layers};
use sophia_store::{Backend, Store};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics");

fn main() {
    let dir = Path::new(DIR);
    let base = dir.join("rules/variants/base");
    let tsv = std::fs::read_to_string(dir.join("layers/truth.tsv")).expect("layers/truth.tsv");
    let (l, _, r) = layer_records("Rules", "base", &base, &tsv).expect("records");
    println!("versions: {:?}", l.zygote.versions.get("base"));
    let piece_bytes: HashMap<_, usize> = r.pieces.iter().map(|p| (p.id, p.bytes.len())).collect();
    println!(
        "{:<4} {:<8} {:>9} {:>9} {:>9} {:>8} {:>8} {:>10} {:>10} {:>10} {:>10}",
        "f", "type", "zy bytes", "en bytes", "en raw", "addr", "between", "zy first", "zy gen", "en first", "en export"
    );
    let (mut zy_total, mut en_total, mut raw_total) = (0, 0, 0);
    let (mut zy_first, mut zy_gen, mut en_first, mut en_export) = (0u64, 0u64, 0u64, 0u64);
    for p in &r.pairs {
        let zy = p.zygote.bytes.len() + p.zygote.refs.iter().map(|i| piece_bytes[i]).sum::<usize>();
        let e = l.enzyme.iter().find(|e| e.target == p.target && e.argtype == p.argtype).expect("enzyme row");
        let d = l.zygote.derived.iter().find(|d| d.label == "base" && d.target == p.target && d.argtype == p.argtype).expect("zygote row");
        println!(
            "{:<4} {:<8} {:>9} {:>9} {:>9} {:>8} {:>8.2} {:>10.1} {:>10.2} {:>10.1} {:>10.1}",
            p.target,
            p.argtype,
            zy,
            p.enzyme.bytes.len(),
            e.raw_bytes,
            e.addresses,
            p.agreement.between,
            d.ns_first as f64 / 1e6,
            d.ns_gen as f64 / 1e6,
            e.ns_first as f64 / 1e6,
            e.ns_export as f64 / 1e6
        );
        zy_total += zy;
        en_total += p.enzyme.bytes.len();
        raw_total += e.raw_bytes;
        zy_first += d.ns_first;
        zy_gen += d.ns_gen;
        en_first += e.ns_first;
        en_export += e.ns_export;
    }
    println!(
        "total: Zygote {zy_total} B as programs ({} B stored: pieces once), Enzyme {en_total} B normalised ({raw_total} B raw)",
        r.pieces.iter().map(|p| p.bytes.len()).sum::<usize>() + r.pairs.iter().map(|p| p.zygote.bytes.len()).sum::<usize>()
    );
    println!(
        "times (ms, sums over {} gradients): Zygote first {:.0}, transformation {:.1}; Enzyme first {:.0}, export {:.0}",
        r.pairs.len(),
        zy_first as f64 / 1e6,
        zy_gen as f64 / 1e6,
        en_first as f64 / 1e6,
        en_export as f64 / 1e6
    );
    let worst = r.pairs.iter().map(|p| p.agreement.between).fold(0.0, f64::max);
    let far = r.pairs.iter().map(|p| p.agreement.zygote.max(p.agreement.enzyme)).fold(0.0, f64::max);
    println!(
        "agreement: {} of {} tested; worst Zygote–Enzyme {worst:.2} ulps; worst against the reference {far:.2} ulps",
        r.pairs.iter().filter(|p| p.agreement.tested).count(),
        r.pairs.len()
    );
    // Sharing: pieces across Zygote's programs, functions across Enzyme's.
    let refs: usize = r.pairs.iter().map(|p| p.zygote.refs.len()).sum();
    println!("Zygote: {refs} piece references, {} distinct pieces", r.pieces.len());
    let (mut funcs, mut fbytes, mut distinct, mut dbytes, mut header) = (0, 0, BTreeSet::new(), 0, 0);
    for p in &r.pairs {
        let (rest, fs) = split_module(std::str::from_utf8(&p.enzyme.bytes).expect("utf-8"));
        header += rest.len();
        for f in fs {
            funcs += 1;
            fbytes += f.len();
            if distinct.insert(f.clone()) {
                dbytes += f.len();
            }
        }
    }
    println!(
        "Enzyme: {funcs} functions ({fbytes} B), {} distinct by text ({dbytes} B); headers, declarations, attributes {header} B",
        distinct.len()
    );
    // What the store grows by (SQLite, on disk).
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("s.sqlite");
    let store = Store::new(sophia_store::sqlite::Sqlite::open(&path).expect("open"));
    let (_, root) = sophia_ingest::module::package_root(&base).expect("package");
    sophia_ingest::store_module(&store, "Rules", &root).expect("ingest");
    let before = store.backend().size_on_disk();
    store_layers(&store, &r).expect("store");
    let after = store.backend().size_on_disk();
    println!("SQLite on disk: package {before:?} B, with both layers {after:?} B");
}
