//! Experiment E-6 (one derivative, two layers) on the E-5 fixture's `base`
//! variant: Zygote's derived program (Julia IR) and Enzyme's (LLVM IR) for
//! each gradient, printed by `julia-semantics/layer_programs.jl` into
//! `layers/truth.tsv` (and, in the opposite order, `layers/reverse.tsv`; see
//! `layer_programs.md` to regenerate), stored as records with a fact and a `tested`
//! equivalence on four backends. Each expected result is the one
//! [[Semantic Boundary Experiments]] §E-6 records.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sophia_hash::Digest;
use sophia_ingest::derivative::parse_derivatives;
use sophia_ingest::layers::{
    Agreement, DERIVED, EQUIVALENT_TESTED, LLVM_IR, LayerReport, Layers, Point, TOLERANCE_ULPS, agreement, layer_records, parse_layers,
    split_module, store_layers,
};
use sophia_query::{Limits, Strategy, Val, prepare_text};
use sophia_store::{Backend, Store, Verification};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics");

fn base() -> PathBuf {
    Path::new(DIR).join("rules/variants/base")
}

fn read(p: &str) -> String {
    std::fs::read_to_string(Path::new(DIR).join(p)).unwrap()
}

fn report() -> (Layers, LayerReport) {
    let (l, _, r) = layer_records("Rules", "base", &base(), &read("layers/truth.tsv")).unwrap();
    (l, r)
}

const TARGETS: [&str; 9] = ["f1", "f2", "f3", "f4", "g", "sq", "h", "p", "q"];

#[test]
fn both_layers_for_every_complete_gradient() {
    let (l, r) = report();
    let got: BTreeSet<(String, String)> = r.pairs.iter().map(|p| (p.target.clone(), p.argtype.clone())).collect();
    let want: BTreeSet<(String, String)> =
        TARGETS.iter().flat_map(|t| ["Float64", "Float32"].map(|ty| (t.to_string(), ty.to_string()))).collect();
    assert_eq!(got, want);
    // `dyn` (runtime dispatch): Zygote's program is incomplete, and Enzyme
    // refuses it outright.
    let skipped: BTreeSet<_> = r.skipped.iter().map(|s| (s.0.as_str(), s.1.as_str())).collect();
    assert_eq!(skipped, [("dyn", "Float64"), ("dyn", "Float32")].into());
    for e in l.enzyme.iter().filter(|e| e.target == "dyn") {
        assert_eq!(e.status, "refused:NonInferredActiveReturn");
    }
    for p in &r.pairs {
        assert_eq!(p.zygote.lang, "julia-ir");
        assert_eq!(p.enzyme.lang, LLVM_IR);
        assert_ne!(p.zygote.id, p.enzyme.id);
        // Each fact by its own tool, at its own version (not EnzymeCore's).
        assert_eq!((p.facts[0].by.as_str(), p.facts[0].by_version.as_str()), ("Zygote", "0.7.18"));
        assert_eq!((p.facts[1].by.as_str(), p.facts[1].by_version.as_str()), ("Enzyme", "0.13.216"));
        // Every piece the program names is a stored piece.
        assert!(p.zygote.refs.iter().all(|i| r.pieces.iter().any(|x| x.id == *i)));
    }
    // Every program is its own record: `p` and `q` share a closure but not a
    // derivative, and `f3 = sq + 1` has `sq`'s derivative under another name.
    let ids: BTreeSet<Digest> = r.pairs.iter().flat_map(|p| [p.zygote.id, p.enzyme.id]).collect();
    assert_eq!(ids.len(), 36);
}

#[test]
fn the_two_layers_agree_at_every_point() {
    let (_, r) = report();
    for p in &r.pairs {
        assert_eq!(p.agreement.points, 10, "{} {}", p.target, p.argtype);
        assert!(p.agreement.tested, "{} {} {:?}", p.target, p.argtype, p.agreement);
        assert_eq!(p.annotation.kind, EQUIVALENT_TESTED);
        assert_eq!(p.annotation.status, "tested");
        assert_eq!(p.annotation.subjects, vec![p.zygote.id, p.enzyme.id]);
        assert_eq!(p.annotation.inputs, vec![p.root]);
    }
    let worst = r.pairs.iter().map(|p| p.agreement.between).fold(0.0, f64::max);
    assert!(worst < 1.1, "{worst}");
    // Far from the reference in both, together: `f2 = g ∘ sq` at Float32 is
    // ill-conditioned for large x (sin of x²), not wrong in either system.
    let f2 = r.pairs.iter().find(|p| p.target == "f2" && p.argtype == "Float32").unwrap();
    assert!(f2.agreement.zygote > 16.0 && f2.agreement.enzyme > 16.0, "{:?}", f2.agreement);
}

fn pt(argtype: &str, z: Option<f64>, e: Option<f64>, r: f64) -> Point {
    Point { label: "base".into(), target: "f".into(), argtype: argtype.into(), x: 1.0, zygote: z, enzyme: e, reference: r }
}

#[test]
fn agreement_edge_cases() {
    let eps = f64::EPSILON;
    let ok = pt("Float64", Some(1.0), Some(1.0 + 2.0 * eps), 1.0);
    let at = pt("Float64", Some(1.0), Some(1.0 + TOLERANCE_ULPS * eps), 1.0);
    let over = pt("Float64", Some(1.0), Some(1.0 + 8.0 * eps), 1.0);
    let nan = pt("Float64", Some(f64::NAN), Some(1.0), 1.0);
    let missing = pt("Float64", None, Some(1.0), 1.0);
    let int = pt("Int64", Some(1.0), Some(1.0), 1.0);
    // Scaled by |ref| when it is above 1.
    let big = pt("Float32", Some(1000.0), Some(1000.0 + 1000.0 * 3.0 * f32::EPSILON as f64), 1000.0);
    assert!(agreement(&[&ok]).tested);
    assert!(agreement(&[&at]).tested);
    assert!(agreement(&[&big]).tested);
    assert!(!agreement(&[&ok, &over]).tested);
    assert!(!agreement(&[&nan]).tested);
    assert!(!agreement(&[&missing]).tested);
    assert!(!agreement(&[&int]).tested);
    // `f64::max` ignores NaN: an unreadable reference must still fail.
    assert!(!agreement(&[&pt("Float64", Some(1.0), Some(1.0), f64::NAN)]).tested);
    assert_eq!(agreement(&[]), Agreement::default(), "no points is not a test");
}

#[test]
fn normalised_modules_are_reproducible_across_session_histories() {
    let a = parse_layers(&read("layers/truth.tsv"));
    let b = parse_layers(&read("layers/reverse.tsv"));
    let key = |l: &Layers| -> BTreeMap<(String, String), (String, String)> {
        l.enzyme
            .iter()
            .filter(|e| e.status == "ok")
            .map(|e| ((e.target.clone(), e.argtype.clone()), (e.raw_digest.clone(), e.digest.clone())))
            .collect()
    };
    let (ka, kb) = (key(&a), key(&b));
    assert_eq!(ka.len(), 18);
    assert_eq!(ka.keys().collect::<Vec<_>>(), kb.keys().collect::<Vec<_>>());
    let mut raw_differs = 0;
    for (k, (raw, norm)) in &ka {
        assert_eq!(norm, &kb[k].1, "{k:?}");
        raw_differs += (raw != &kb[k].0) as usize;
    }
    // Julia's text, as printed, depends on the session: every module.
    assert_eq!(raw_differs, 18);
}

/// Addresses the normaliser left in: an `inttoptr (i64 N …)` with N of 4096
/// or more anywhere (constant data included), and outside `@_j_const` data
/// lines any decimal literal in the user address range.
fn addresses(text: &str) -> Vec<u64> {
    let mut out = Vec::new();
    for line in text.lines() {
        for part in line.split("inttoptr (i64 ").skip(1) {
            let n: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = n.parse::<u64>()
                && n >= 4096
            {
                out.push(n);
            }
        }
        if line.starts_with("@_j_const") {
            continue;
        }
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i].is_ascii_digit() && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'.' || b[i - 1] == b'-')) {
                let s = i;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                if let Ok(n) = line[s..i].parse::<u64>()
                    && ((1u64 << 44)..(1u64 << 47)).contains(&n)
                {
                    out.push(n);
                }
            } else {
                i += 1;
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn every_address_is_named() {
    let (l, r) = report();
    for e in l.enzyme.iter().filter(|e| e.status == "ok") {
        assert_eq!(e.addresses, e.described, "{} {}", e.target, e.argtype);
    }
    for p in &r.pairs {
        let text = std::str::from_utf8(&p.enzyme.bytes).unwrap();
        assert_eq!(addresses(text), Vec::<u64>::new(), "{} {}", p.target, p.argtype);
        assert!(!text.contains("{obj "), "an object the normaliser could not name");
    }
    // The finder finds what the normaliser leaves: an address it would have
    // replaced, a call it could not name, a pointer in constant data.
    assert_eq!(addresses("store ptr inttoptr (i64 140737185820208 to ptr)"), vec![140737185820208]);
    assert_eq!(addresses("call void inttoptr (i64 8192 to ptr)()"), vec![8192]);
    assert_eq!(addresses("@_j_const_2 = constant [1 x ptr] [ptr inttoptr (i64 4294967296 to ptr)]"), vec![4294967296]);
    assert_eq!(addresses("@_j_const_1 = private constant [1 x i64] [i64 17592186044416]"), Vec::<u64>::new());
}

#[test]
fn loading_enzyme_leaves_zygotes_programs_as_e5_found_them() {
    let layers = parse_derivatives(&read("layers/truth.tsv"));
    let e5 = parse_derivatives(&read("rules/truth.tsv"));
    let programs = |o: &sophia_ingest::derivative::Oracle| -> BTreeMap<(String, String), Digest> {
        o.derived.iter().filter(|d| d.label == "base").map(|d| ((d.target.clone(), d.argtype.clone()), d.output())).collect()
    };
    assert_eq!(programs(&layers).len(), 20);
    assert_eq!(programs(&layers), programs(&e5));
    assert_eq!(layers.rules.get("base"), e5.rules.get("base"));
    // EnzymeCore's ChainRulesCore extension adds one `rrule` and one
    // `frule` (for `ignore_derivatives`): E-5's `Tables` key would re-key
    // every derivative, its `Rules` key none.
    let tables = |o: &sophia_ingest::derivative::Oracle| -> BTreeMap<String, usize> {
        o.tables["base"].iter().map(|(n, k, _)| (n.clone(), *k)).collect()
    };
    let (a, b) = (tables(&layers), tables(&e5));
    for (n, k) in &b {
        let extra = if n == "rrule" || n == "frule" { 1 } else { 0 };
        assert_eq!(a[n], k + extra, "{n}");
    }
}

#[test]
fn modules_share_functions_only_by_text() {
    let (_, r) = report();
    let mut all = 0;
    let mut distinct = BTreeSet::new();
    for p in &r.pairs {
        let (_, funcs) = split_module(std::str::from_utf8(&p.enzyme.bytes).unwrap());
        assert!(!funcs.is_empty());
        all += funcs.len();
        distinct.extend(funcs);
    }
    // Some functions (`sin` at one type, reached from f1, f2 and g) recur.
    assert!(distinct.len() < all, "{} of {all}", distinct.len());
}

fn ids(store: &Store<impl Backend>, text: &str) -> BTreeSet<Digest> {
    let q = prepare_text(text).unwrap();
    let rows = store.read(|tx| Ok(q.run(tx, &[], Limits { max_rows: usize::MAX, ..Limits::default() }, Strategy::Auto).unwrap())).unwrap();
    rows.rows
        .into_iter()
        .map(|r| match &r[0] {
            Val::Id(d) => *d,
            v => panic!("{v:?}"),
        })
        .collect()
}

fn round_trip<B: Backend>(store: Store<B>) {
    let (_, root) = sophia_ingest::module::package_root(&base()).unwrap();
    sophia_ingest::store_module(&store, "Rules", &root).unwrap();
    let (_, r) = report();
    store_layers(&store, &r).unwrap();
    for rec in r.records() {
        assert_eq!(store.record(&rec.id).unwrap().as_ref(), Some(&rec));
        assert_eq!(store.verify(&rec.id).unwrap(), Verification::Valid);
    }
    for p in &r.pairs {
        // f is in the store, and its facts name both programs.
        assert!(store.record(&p.root).unwrap().is_some(), "{}", p.target);
        let facts = store.read(|tx| tx.facts_of(&p.root)).unwrap();
        for f in &p.facts {
            assert!(facts.contains(f), "{} {}", p.target, p.argtype);
            assert!(!store.is_stale(f).unwrap());
        }
        for prog in [&p.zygote, &p.enzyme] {
            let anns = store.read(|tx| tx.annotations_of(&prog.id)).unwrap();
            assert_eq!(anns, vec![p.annotation.clone()]);
        }
    }
    // Storing again adds nothing.
    let seq = store.last_seq().unwrap();
    store_layers(&store, &r).unwrap();
    assert_eq!(store.last_seq().unwrap(), seq);
    // Queries see both: the programs with a tested equivalence, and the
    // functions with a derived program.
    let tested = ids(&store, "query tested()\n select p where annotation(_, \"equivalent-tested\", p)\nend");
    assert_eq!(tested, r.pairs.iter().flat_map(|p| [p.zygote.id, p.enzyme.id]).collect());
    let derived = ids(&store, &format!("query derived()\n select f where fact(_, \"{DERIVED}\", f)\nend"));
    assert_eq!(derived, r.pairs.iter().map(|p| p.root).collect());
    assert_eq!(derived.len(), 9);
}

macro_rules! suite {
    ($name:ident, $open:expr) => {
        #[test]
        fn $name() {
            let dir = tempfile::tempdir().unwrap();
            let open: fn(&Path) -> _ = $open;
            round_trip(Store::new(open(dir.path())));
        }
    };
}

suite!(round_trip_sqlite, |p: &Path| sophia_store::sqlite::Sqlite::open(p.join("s.sqlite")).unwrap());
suite!(round_trip_redb, |p: &Path| sophia_store::redb_store::Redb::open(p.join("s.redb")).unwrap());
suite!(round_trip_lmdb, |p: &Path| sophia_store::lmdb_store::Lmdb::open_with_map_size(p.join("s.lmdb"), 1 << 30).unwrap());
suite!(round_trip_duckdb, |p: &Path| sophia_store::duckdb_store::DuckDb::open(p.join("s.duckdb")).unwrap());

// Review findings (Codex, 2026-11-19): each test fails without its fix.

/// The fixture's output with every line that starts with `prefix` and
/// contains `needle` replaced by `f(line)`.
fn edited(prefix: &str, needle: &str, f: impl Fn(&str) -> String) -> String {
    read("layers/truth.tsv")
        .lines()
        .map(|l| if l.starts_with(prefix) && l.contains(needle) { f(l) } else { l.to_string() })
        .collect::<Vec<_>>()
        .join("\n")
}

fn records(tsv: &str) -> LayerReport {
    layer_records("Rules", "base", &base(), tsv).unwrap().2
}

fn pair<'a>(r: &'a LayerReport, t: &str, ty: &str) -> Option<&'a sophia_ingest::layers::Pair> {
    r.pairs.iter().find(|p| p.target == t && p.argtype == ty)
}

#[test]
fn another_labels_rows_are_not_used() {
    // Finding 5: a second variant's Enzyme row and points for g at Float32.
    let mut tsv = read("layers/truth.tsv");
    tsv.push_str("\nenzyme\tother\tg\tFloat32\trefused:X\t0\t0\t0\t-\t0\t0\t0\t-\n");
    tsv.push_str("point\tother\tg\tFloat32\t0.7f0\t1.0f0\t99.0f0\t1.0\n");
    let r = records(&tsv);
    let p = pair(&r, "g", "Float32").expect("base's g at Float32 is still paired");
    assert_eq!(p.agreement.points, 10);
    assert!(p.agreement.tested);
    assert!(r.skipped.iter().all(|s| s.0 == "dyn"), "{:?}", r.skipped);
}

#[test]
fn an_unreadable_point_fails_the_test() {
    // Finding 6: f2 at Float32, one reference unreadable.
    let mut done = false;
    let tsv = read("layers/truth.tsv")
        .lines()
        .map(|l| {
            if !done && l.starts_with("point\tbase\tf2\tFloat32\t") {
                done = true;
                let mut f: Vec<&str> = l.split('\t').collect();
                f[7] = "garbage";
                f.join("\t")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let r = records(&tsv);
    let p = pair(&r, "f2", "Float32").unwrap();
    assert_eq!(p.agreement.points, 10);
    assert!(!p.agreement.tested);
    assert_eq!(p.annotation.status, "failed");
}

#[test]
fn float32_values_are_read_as_float32() {
    // Finding 7: 1.0000005f0 is the Float32 1 + 4 eps(Float32), exactly at
    // the tolerance; read as a Float64 decimal it is 4.19 eps away.
    let l = parse_layers("point\tbase\tf\tFloat32\t1.0f0\t1.0f0\t1.0000005f0\t1.0\n");
    assert_eq!(l.points[0].enzyme, Some(1.0000005f32 as f64));
    assert!(agreement(&[&l.points[0]]).tested);
}

#[test]
fn a_function_with_several_methods_gets_no_fact() {
    // Finding 9: `method_g32` has `g(x)` and `g(x::Float32)`; which one a
    // gradient starts from is dispatch, which the ids do not say.
    let dir = Path::new(DIR).join("rules/variants/method_g32");
    let (_, _, r) = layer_records("Rules", "base", &dir, &read("layers/truth.tsv")).unwrap();
    assert!(pair(&r, "g", "Float64").is_none() && pair(&r, "g", "Float32").is_none());
    assert!(r.skipped.iter().any(|s| s.0 == "g" && s.2.contains("methods")), "{:?}", r.skipped);
    assert!(pair(&r, "f1", "Float64").is_some());
}

#[test]
fn a_module_text_must_match_its_digest() {
    // Finding 10: text substituted under an unchanged digest.
    let tsv = edited("module\t", "diffejulia_g_", |l| l.replacen("fmul", "fadd", 1));
    let r = records(&tsv);
    assert!(pair(&r, "g", "Float64").is_none() || pair(&r, "g", "Float32").is_none());
    assert!(r.skipped.iter().any(|s| s.2.contains("digest")), "{:?}", r.skipped);
    // The fixture's texts all match.
    assert_eq!(records(&read("layers/truth.tsv")).pairs.len(), 18);
}

#[test]
fn a_failed_test_is_stored_as_failed() {
    // Finding 11: Enzyme's value for h at Float64 far off at one point.
    let mut done = false;
    let tsv = read("layers/truth.tsv")
        .lines()
        .map(|l| {
            if !done && l.starts_with("point\tbase\th\tFloat64\t") {
                done = true;
                let mut f: Vec<&str> = l.split('\t').collect();
                f[6] = "1.0e10";
                f.join("\t")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let r = records(&tsv);
    let h = pair(&r, "h", "Float64").unwrap().clone();
    assert_eq!((h.annotation.kind.as_str(), h.annotation.status.as_str()), (sophia_ingest::layers::EQUIVALENT_FAILED, "failed"));
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("s.sqlite")).unwrap());
    let (_, root) = sophia_ingest::module::package_root(&base()).unwrap();
    sophia_ingest::store_module(&store, "Rules", &root).unwrap();
    store_layers(&store, &r).unwrap();
    assert_eq!(store.read(|tx| tx.annotations_of(&h.enzyme.id)).unwrap(), vec![h.annotation.clone()]);
    let tested = ids(&store, "query tested()\n select p where annotation(_, \"equivalent-tested\", p)\nend");
    assert_eq!(tested.len(), 34);
    assert!(!tested.contains(&h.zygote.id) && !tested.contains(&h.enzyme.id));
}
