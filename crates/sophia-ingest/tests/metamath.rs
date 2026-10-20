//! The Metamath frontend: identity rules, proofs, storing.

use sophia_ingest::metamath::{name_for, parse};
use sophia_store::{Store, Target};

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

fn ids(text: &str) -> Vec<(String, sophia_hash::Digest, String)> {
    parse(text).unwrap().into_iter().map(|a| (a.label, a.record.id, a.record.kind)).collect()
}

#[test]
fn demo_parses_with_kinds_and_references() {
    let a = parse(DEMO).unwrap();
    let kinds: Vec<(&str, &str)> = a.iter().map(|x| (x.label.as_str(), x.role.as_str())).collect();
    assert_eq!(kinds, vec![
        ("tze", "syntax"), ("tpl", "syntax"), ("weq", "syntax"), ("wim", "syntax"),
        ("a1", "axiom"), ("a2", "axiom"), ("mp", "axiom"), ("th1", "theorem"),
    ]);
    let th1 = a.last().unwrap();
    let proof = th1.proof.as_ref().expect("a theorem has a proof record");
    assert_eq!(proof.kind, "proof");
    let by = |l: &str| a.iter().find(|x| x.label == l).unwrap().record.id;
    assert!(!proof.refs.contains(&th1.record.id), "a proof uses, it does not reference, what it proves");
    for l in ["tze", "tpl", "weq", "wim", "a1", "a2", "mp"] {
        assert!(proof.refs.contains(&by(l)), "th1's proof uses {l}");
    }
    // A theorem used by a later proof is referenced through its proof.
    let more = format!("{DEMO}\nth3 $p |- t = t $= tt th1 $.\n");
    let b = parse(&more).unwrap();
    let th3 = b.last().unwrap().proof.as_ref().unwrap();
    assert_eq!(th3.refs, vec![proof.id]);
    assert!(th1.record.refs.is_empty(), "a statement references nothing");
    assert!(sophia_hash::Record::verify(&[&th1.record]));
    assert!(sophia_hash::Record::verify(&[proof]));
}

#[test]
fn labels_and_variable_names_do_not_change_ids() {
    let renamed = DEMO
        .replace("th1", "reflexivity")
        .replace("a2", "addid")
        .replace(" t ", " x ")
        .replace("$v t r", "$v x r")
        .replace("( t ", "( x ")
        .replace(" t)", " x)");
    let a = ids(DEMO);
    let b = ids(&renamed);
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.1, y.1, "{} vs {}", x.0, y.0);
    }
}

#[test]
fn a_changed_statement_changes_its_id_and_its_dependants() {
    let changed = DEMO.replace("a2 $a |- ( t + 0 ) = t $.", "a2 $a |- ( 0 + t ) = t $.");
    let a = ids(DEMO);
    let b = ids(&changed);
    let _ = (&a[0].2, &b[0].2);
    let get = |v: &[(String, sophia_hash::Digest, String)], l: &str| v.iter().find(|x| x.0 == l).unwrap().1;
    assert_eq!(get(&a, "a1"), get(&b, "a1"));
    assert_ne!(get(&a, "a2"), get(&b, "a2"));
    assert_eq!(get(&a, "th1"), get(&b, "th1"), "th1's statement does not mention a2");
    let proof = |text: &str| parse(text).unwrap().last().unwrap().proof.as_ref().unwrap().id;
    assert_ne!(proof(DEMO), proof(&changed), "Merkle: th1's proof uses a2");
}

#[test]
fn another_proof_of_the_same_statement_shares_the_statement() {
    // Same statement, different (here: normal vs compressed) proof.
    let compressed = DEMO.replace(
        "  tt tze tpl tt weq tt tt weq tt a2 tt tze tpl tt weq tt tze tpl tt weq tt tt\n  weq wim tt a2 tt tze tpl tt tt a1 mp mp $.",
        "( tze tpl weq a2 wim a1 mp ) AABCZADZAADZAEZJJKFLIAAGHH $.",
    );
    let a = parse(DEMO).unwrap();
    let b = parse(&compressed).unwrap();
    assert_eq!(a.last().unwrap().record.id, b.last().unwrap().record.id);
    assert_ne!(a.last().unwrap().proof.as_ref().unwrap().id, b.last().unwrap().proof.as_ref().unwrap().id);
}

#[test]
fn repeated_theorems_are_one_record() {
    let twice = format!("{DEMO}\nth2 $p |- t = t $=\n  tt tze tpl tt weq tt tt weq tt a2 tt tze tpl tt weq tt tze tpl tt weq tt tt\n  weq wim tt a2 tt tze tpl tt tt a1 mp mp $.\n");
    let a = parse(&twice).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("m.sqlite")).unwrap());
    let r = sophia_ingest::store_metamath(&s, "demo", &a, 1000).unwrap();
    assert_eq!(r.assertions, 9);
    assert_eq!(r.proofs, 2);
    assert_eq!(r.new_records, 9, "8 statements and 1 proof: th2 and its proof are th1's");
    let t1 = s.resolve(&name_for("demo", "th1")).unwrap().unwrap().target;
    let t2 = s.resolve(&name_for("demo", "th2")).unwrap().unwrap().target;
    assert_eq!(t1, t2);
    assert!(matches!(t1, Target::Content(_)));
    // The proof is attached to its statement by a `proof` annotation.
    let anns = s.read(|tx| tx.annotations_of(&t1.digest())).unwrap();
    assert_eq!(anns.len(), 1, "th1 is proved, not assumed");
    assert_eq!(anns[0].kind, "proof");
    let p = s.resolve(&sophia_ingest::metamath::proof_name_for("demo", "th1")).unwrap().unwrap().target.digest();
    assert_eq!(anns[0].subjects, vec![t1.digest(), p]);
    assert_eq!(s.read(|tx| tx.annotation(&anns[0].id)).unwrap().as_ref(), Some(&anns[0]));
    // The library is three collections.
    let coll = |w: &str| s.resolve(&sophia_ingest::library_collection("demo", w)).unwrap().unwrap().target.digest();
    assert_eq!(s.methods(&coll("axioms")).unwrap().len(), 3, "a1, a2, mp");
    assert_eq!(s.methods(&coll("proofs")).unwrap(), vec![p], "th1 and th2 share one proof");
    assert!(s.methods(&coll("definitions")).unwrap().is_empty());
}

#[test]
fn compressed_proofs_decode() {
    // th1 again, in compressed form (as `metamath` would write it).
    let compressed = DEMO.replace(
        "  tt tze tpl tt weq tt tt weq tt a2 tt tze tpl tt weq tt tze tpl tt weq tt tt\n  weq wim tt a2 tt tze tpl tt tt a1 mp mp $.",
        "( tze tpl weq a2 wim a1 mp ) AABCZADZAADZAEZJJKFLIAAGHH $.",
    );
    let a = parse(&compressed).unwrap();
    let th1 = a.last().unwrap();
    assert_eq!(th1.role, "theorem");
    assert_eq!(th1.record.kind, "statement");
    assert_eq!(th1.proof.as_ref().unwrap().refs.len(), 7, "the 7 used assertions");
}

#[test]
fn names_quote_labels_that_are_not_identifiers() {
    assert_eq!(name_for("set", "ax-mp"), "mm.set.`ax-mp`");
    assert_eq!(name_for("set", "2p2e4"), "mm.set.`2p2e4`");
    assert_eq!(name_for("set", "mpbi"), "mm.set.mpbi");
    sophia_store::validate_path(&name_for("set", "df-an")).unwrap();
    sophia_store::validate_path(&name_for("set", "2p2e4")).unwrap();
}

/// The real libraries, if fetched (`~/.cache/sophia-corpora`). Run with
/// `cargo test --release -p sophia-ingest -- --ignored`.
#[test]
#[ignore]
fn set_mm_and_iset_mm_parse() {
    let home = std::env::var("HOME").unwrap();
    for db in ["set.mm", "iset.mm"] {
        let path = format!("{home}/.cache/sophia-corpora/set.mm/{db}");
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let t = std::time::Instant::now();
        let a = parse(&text).unwrap();
        let distinct: std::collections::HashSet<_> = a.iter().map(|x| x.record.id).collect();
        eprintln!("{db}: {} assertions, {} distinct, parsed in {:.2?}", a.len(), distinct.len(), t.elapsed());
        assert!(a.len() > 10_000);
    }
}
