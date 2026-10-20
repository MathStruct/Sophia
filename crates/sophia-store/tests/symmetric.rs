//! Review P0 (2026-10-20): symmetric recursive components, through both backends.

use sophia_hash::Record;
use sophia_store::*;
use sophialang_core::samples::builtin;
use sophialang_core::*;

/// f() = g(), g() = f(): structurally identical members.
fn symmetric() -> Vec<Definition> {
    let int = || Term::Nominal(builtin("Int64"));
    let body = |other: u32| Term::lam("x", Term::app(Term::Draft(other), [Term::Var(0)]));
    vec![Definition::method(Term::arrow(int(), int()), body(1)), Definition::method(Term::arrow(int(), int()), body(0))]
}

fn check<B: Backend>(s: Store<B>) {
    let recs = normalise_batch(&symmetric(), &mut Default::default()).unwrap();
    s.add(&recs, &[("f".into(), 0, Visibility::Public), ("g".into(), 1, Visibility::Public)]).unwrap();
    for r in &recs {
        assert!(s.verify(&r.id).unwrap(), "verify failed for {:?} (component size {})", r.id, r.component_size);
        let back = s.record(&r.id).unwrap().unwrap();
        let members: Vec<Record> = s.read(|tx| tx.component(&back.component)).unwrap();
        assert_eq!(members.len() as u32, back.component_size, "stored members must reconstruct the component");
    }
}

#[test]
fn sqlite() {
    let d = tempfile::tempdir().unwrap();
    check(Store::new(sophia_store::sqlite::Sqlite::open(d.path().join("s")).unwrap()));
}

#[test]
fn redb() {
    let d = tempfile::tempdir().unwrap();
    check(Store::new(sophia_store::redb_store::Redb::open(d.path().join("r")).unwrap()));
}

#[cfg(feature = "lmdb")]
#[test]
fn lmdb() {
    let d = tempfile::tempdir().unwrap();
    check(Store::new(sophia_store::lmdb_store::Lmdb::open_with_map_size(d.path().join("l"), 1 << 30).unwrap()));
}

#[cfg(feature = "duckdb")]
#[test]
fn duckdb() {
    let d = tempfile::tempdir().unwrap();
    check(Store::new(sophia_store::duckdb_store::DuckDb::open(d.path().join("d")).unwrap()));
}

#[test]
fn symmetric_members_are_one_definition_stable_under_permutation_and_renaming() {
    let int = || Term::Nominal(builtin("Int64"));
    let a = normalise_batch(&symmetric(), &mut Default::default()).unwrap();
    assert_eq!(a[0].id, a[1].id, "f and g are the same definition");
    assert_eq!(a[0].component_size, 1);
    // Same as h() = h().
    let h = Definition::method(Term::arrow(int(), int()), Term::lam("y", Term::app(Term::Draft(0), [Term::Var(0)])));
    assert_eq!(normalise_batch(&[h], &mut Default::default()).unwrap()[0].id, a[0].id);
    // Swapping the input order changes nothing.
    let mut swapped = symmetric();
    swapped.reverse();
    let b = normalise_batch(&swapped, &mut Default::default()).unwrap();
    assert_eq!(b[0].id, a[0].id);
    // An asymmetric pair (different bodies) stays two members.
    let mut asym = symmetric();
    asym[1] = Definition::method(
        Term::arrow(int(), int()),
        Term::lam("x", Term::app(Term::Draft(0), [Term::app(Term::Nominal(builtin("+")), [Term::Var(0), Term::Lit(Lit::Int64(1))])])),
    );
    let c = normalise_batch(&asym, &mut Default::default()).unwrap();
    assert_ne!(c[0].id, c[1].id);
    assert_eq!(c[0].component_size, 2);
}
