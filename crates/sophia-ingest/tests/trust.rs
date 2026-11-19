//! Proof trust records (B-021, SPEC-006 §5): a failed check taints every
//! proof that rests on the failed one, and only those; the check
//! annotations carry the checker and its status; metamath.exe's error lines
//! give the failed labels.

use std::collections::BTreeSet;

use sophia_hash::Digest;
use sophia_ingest::metamath::{parse, proof_name_for};
use sophia_ingest::trust::{CHECK_FAILED, CHECKED, Checker, RESTS_ON_CHECKED, RESTS_ON_UNCHECKED, metamath_verify_failures, proof_checks};
use sophia_query::{Limits, Strategy, Val, prepare_text};
use sophia_store::Store;

/// th2 uses th1; th3 uses only an axiom; th4 uses th2 (two levels up).
const LIB: &str = r#"
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
th2 $p |- r = r $= tr th1 $.
th3 $p |- ( s + 0 ) = s $= ts a2 $.
th4 $p |- s = s $= ts th2 $.
"#;

fn checker() -> Checker {
    Checker { name: "metamath.exe".into(), version: "test".into(), kernel: "metamath.exe verify proof".into() }
}

fn proofs(store: &Store<sophia_store::sqlite::Sqlite>, text: &str) -> BTreeSet<Digest> {
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

#[test]
fn a_failed_proof_taints_what_rests_on_it() {
    let parsed = parse(LIB).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("s.sqlite")).unwrap());
    sophia_ingest::store_metamath(&store, "lib", &parsed, 100).unwrap();
    let proof = |l: &str| store.resolve(&proof_name_for("lib", l)).unwrap().unwrap().target.digest();
    let (th1, th2, th3, th4) = (proof("th1"), proof("th2"), proof("th3"), proof("th4"));

    // Before any check, nothing rests only on checked proofs.
    assert!(proofs(&store, RESTS_ON_CHECKED).is_empty());

    let failed: BTreeSet<String> = ["th2".to_string(), "nosuch".to_string()].into();
    let input = Digest::of(b"the source manifest");
    let (anns, unknown) = proof_checks(&checker(), &parsed, &failed, &[input]);
    assert_eq!(unknown, vec!["nosuch".to_string()]);
    assert_eq!(anns.len(), 4);
    for a in &anns {
        assert_eq!(a.subjects.len(), 1);
        assert_eq!(a.checker.as_deref(), Some("metamath.exe"));
        assert_eq!(a.checker_version.as_deref(), Some("test"));
        assert_eq!(a.inputs, vec![input]);
        let bad = a.subjects[0] == th2;
        assert_eq!((a.kind.as_str(), a.status.as_str()), if bad { (CHECK_FAILED, "failed") } else { (CHECKED, "checked") });
    }
    store.annotate_many(&anns).unwrap();
    let stored = store.read(|tx| tx.annotations_of(&th2)).unwrap();
    assert!(stored.iter().any(|a| a.kind == CHECK_FAILED && a.status == "failed"), "{stored:?}");

    assert_eq!(proofs(&store, RESTS_ON_CHECKED), [th1, th3].into());
    assert_eq!(proofs(&store, RESTS_ON_UNCHECKED), [th2, th4].into());
}

/// Checking again with nothing failed adds a `checked` annotation beside the
/// earlier `check-failed` one (the kind is part of the id, so neither
/// replaces the other); a proof is tainted only while it has no `checked`.
#[test]
fn a_recheck_that_passes_untaints() {
    let parsed = parse(LIB).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("s.sqlite")).unwrap());
    sophia_ingest::store_metamath(&store, "lib", &parsed, 100).unwrap();
    let failed: BTreeSet<String> = ["th1".to_string()].into();
    store.annotate_many(&proof_checks(&checker(), &parsed, &failed, &[]).0).unwrap();
    assert_eq!(proofs(&store, RESTS_ON_UNCHECKED).len(), 3);
    store.annotate_many(&proof_checks(&checker(), &parsed, &BTreeSet::new(), &[]).0).unwrap();
    assert!(proofs(&store, RESTS_ON_UNCHECKED).is_empty());
    assert_eq!(proofs(&store, RESTS_ON_CHECKED).len(), 4);
}

#[test]
fn metamath_error_lines_give_labels() {
    let out = "MM> verify proof *\n\
               0 10%  20%\n\
               ?Error on line 17 of file \"lib.mm\" at statement 23, label \"th2\", type \"$p\":\n\
               th2 $p |- r = r $= tr th1 $.\n\
               The proof does not prove the statement.\n\
               ?Error on line 19 of file \"lib.mm\" at statement 25, label \"th4\", type \"$p\":\n\
               MM> exit\n";
    assert_eq!(metamath_verify_failures(out), ["th2".to_string(), "th4".to_string()].into());
    assert!(metamath_verify_failures("MM> verify proof *\nAll proofs in the database were verified in 2.85 s.\n").is_empty());
}
