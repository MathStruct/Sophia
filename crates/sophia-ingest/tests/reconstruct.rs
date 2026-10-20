//! Every record an ingest stores can be rebuilt from the store alone (the
//! source records and the ingest manifest) and is identical to the stored
//! one, on every backend. The precondition for dropping derived bytes
//! ([[Current Inventory]]); see `src/source.md`.

use std::path::Path;

use sophia_ingest::source::{self, Manifest, reconstruct};
use sophia_store::{Backend, Store};

const MODULES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden-moonkale/module");

const DEMO: &str = r#"
$c 0 + = -> ( ) term wff |- $.
$v t r s P Q $.
tt $f term t $.  tr $f term r $.  ts $f term s $.  wp $f wff P $.  wq $f wff Q $.
tze $a term 0 $.
tpl $a term ( t + r ) $.
weq $a wff t = r $.
wim $a wff ( P -> Q ) $.
a1 $a |- ( t = r -> ( t = s -> r = s ) ) $.
a2 $a |- ( t + 0 ) = t $.
${ min $e |- P $.  maj $e |- ( P -> Q ) $.  mp $a |- Q $. $}
th1 $p |- t = t $=
  tt tze tpl tt weq tt tt weq tt a2 tt tze tpl tt weq tt tze tpl tt weq tt tt
  weq wim tt a2 tt tze tpl tt tt a1 mp mp $.
"#;

fn manifest_of<B: Backend>(s: &Store<B>, name: &str) -> sophia_hash::Digest {
    s.resolve(name).unwrap().unwrap_or_else(|| panic!("no {name}")).target.digest()
}

/// Julia modules (the golden fixtures): every record rebuilt and identical,
/// every stored Julia record among the rebuilt ones, and the sources verify
/// under Sophia's own hashing.
fn julia_modules_rebuild<B: Backend>(s: Store<B>) {
    let mut rebuilt = std::collections::BTreeSet::new();
    for root in ["Geo", "Callable", "Macros"] {
        sophia_ingest::store_module(&s, root, &Path::new(MODULES).join(format!("{root}.jl"))).unwrap();
        let m = manifest_of(&s, &format!("ingest.jl.{root}"));
        let rec = reconstruct(&s, &m).unwrap();
        assert!(rec.ok(), "{root}: {rec:?}");
        rebuilt.extend(rec.ids.iter().copied());
        let manifest = Manifest::decode(&s.record(&m).unwrap().unwrap().bytes).unwrap();
        assert_eq!(rec.sources, manifest.files.len());
        for f in &manifest.files {
            let src = s.record(&f.source).unwrap().unwrap();
            assert!(sophia_hash::Record::verify(&[&src]), "{}", f.path);
        }
        assert!(sophia_hash::Record::verify(&[&s.record(&m).unwrap().unwrap()]));
    }
    let stored: std::collections::BTreeSet<_> = s
        .read(|tx| {
            let mut v = Vec::new();
            for id in tx.records_of_kind(None, usize::MAX)? {
                if tx.record(&id)?.is_some_and(|r| r.lang == "julia") {
                    v.push(id);
                }
            }
            Ok(v)
        })
        .unwrap()
        .into_iter()
        .collect();
    assert!(!stored.is_empty());
    assert_eq!(stored, rebuilt, "every stored Julia record is derivable from stored sources");
}

/// Metamath: statements and proofs rebuilt and identical.
fn metamath_rebuilds<B: Backend>(s: Store<B>) {
    let (r, m) = sophia_ingest::store_metamath_source(&s, "demo", DEMO, 100).unwrap();
    let rec = reconstruct(&s, &m).unwrap();
    assert!(rec.ok(), "{rec:?}");
    assert_eq!(rec.rebuilt, r.new_records, "statements and proofs, each once");
}

/// The check is not vacuous: a manifest whose source differs from what was
/// ingested rebuilds records the store does not have.
fn changed_source_is_detected<B: Backend>(s: Store<B>) {
    let (_, m) = sophia_ingest::store_metamath_source(&s, "demo", DEMO, 100).unwrap();
    let mut manifest = Manifest::decode(&s.record(&m).unwrap().unwrap().bytes).unwrap();
    let other = DEMO.replace("( t + 0 ) = t", "( 0 + t ) = t");
    manifest.files[0].source = source::source_record("demo.mm", &other).id;
    let m2 = source::store_manifest(&s, "ingest.mm.tampered", &manifest, &[&other]).unwrap();
    let rec = reconstruct(&s, &m2).unwrap();
    assert!(!rec.ok());
    assert!(!rec.missing.is_empty(), "{rec:?}");
}

/// Re-ingesting the same sources stores nothing new, and the manifest name
/// keeps pointing at the same manifest.
fn reingest_is_a_noop<B: Backend>(s: Store<B>) {
    let root = Path::new(MODULES).join("Geo.jl");
    sophia_ingest::store_module(&s, "Geo", &root).unwrap();
    let m = manifest_of(&s, "ingest.jl.Geo");
    let seq = s.last_seq().unwrap();
    let (r, _, _) = sophia_ingest::store_module(&s, "Geo", &root).unwrap();
    assert_eq!(r.new_records, 0);
    assert_eq!(manifest_of(&s, "ingest.jl.Geo"), m);
    assert_eq!(s.last_seq().unwrap(), seq);
}

macro_rules! suite {
    ($module:ident, $make:expr) => {
        mod $module {
            use super::*;
            fn store() -> (Store<impl Backend>, tempfile::TempDir) {
                let dir = tempfile::tempdir().unwrap();
                let make = $make;
                (Store::new(make(dir.path())), dir)
            }
            #[test] fn julia_modules_rebuild() { let (s, _d) = store(); super::julia_modules_rebuild(s); }
            #[test] fn metamath_rebuilds() { let (s, _d) = store(); super::metamath_rebuilds(s); }
            #[test] fn changed_source_is_detected() { let (s, _d) = store(); super::changed_source_is_detected(s); }
            #[test] fn reingest_is_a_noop() { let (s, _d) = store(); super::reingest_is_a_noop(s); }
        }
    };
}

suite!(sqlite, |p: &Path| sophia_store::sqlite::Sqlite::open(p.join("s.sqlite")).unwrap());
suite!(redb, |p: &Path| sophia_store::redb_store::Redb::open(p.join("s.redb")).unwrap());
suite!(lmdb, |p: &Path| sophia_store::lmdb_store::Lmdb::open_with_map_size(p.join("s.lmdb"), 1 << 30).unwrap());
suite!(duckdb, |p: &Path| sophia_store::duckdb_store::DuckDb::open(p.join("s.duckdb")).unwrap());
