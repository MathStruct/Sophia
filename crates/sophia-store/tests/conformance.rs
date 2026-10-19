//! The conformance suite (SPEC-001 §13): the same tests on every backend.

use sophia_hash::Record;
use sophia_store::*;
use sophialang_core::samples::{builtin, program};
use sophialang_core::{Definition, Term, normalise_batch, normalise_one};

fn sample() -> (Vec<&'static str>, Vec<Record>) {
    let (names, defs): (Vec<_>, Vec<Definition>) = program().into_iter().unzip();
    let records = normalise_batch(&defs, &mut Default::default()).unwrap();
    (names, records)
}

fn populated<B: Backend>(store: &Store<B>) -> (Vec<&'static str>, Vec<Record>) {
    let (names, records) = sample();
    let named: Vec<(String, usize, Visibility)> =
        names.iter().enumerate().map(|(i, n)| (n.to_string(), i, Visibility::Public)).collect();
    store.add(&records, &named).unwrap();
    (names, records)
}

fn add_one_more(n: i64) -> Record {
    normalise_one(&Definition::constant(
        Term::Nominal(builtin("Int64")),
        Term::Lit(sophialang_core::Lit::Int64(n)),
    ))
    .unwrap()
}

fn re_putting_writes_nothing<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let seq = s.last_seq().unwrap();
    assert_eq!(s.put_records(&records).unwrap(), 0);
    assert_eq!(s.last_seq().unwrap(), seq);
    let count = s.read(|tx| tx.record_count()).unwrap();
    // The two relation signatures share content.
    assert_eq!(count as usize, records.len() - 1);
}

fn records_round_trip_and_verify<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    for r in &records {
        assert_eq!(s.record(&r.id).unwrap().as_ref(), Some(r));
        assert!(s.verify(&r.id).unwrap(), "{}", r.kind);
    }
}

fn names_resolve_and_list<B: Backend>(s: Store<B>) {
    let (names, records) = populated(&s);
    let e = s.resolve("sample.add").unwrap().unwrap();
    assert_eq!(e.target, Target::Content(records[1].id));
    let all = s.read(|tx| tx.names_under("sample", 100)).unwrap();
    assert_eq!(all.len(), names.len());
    assert!(all.windows(2).all(|w| w[0].path < w[1].path));
    let none = s.read(|tx| tx.names_under("samp", 100)).unwrap();
    assert!(none.is_empty(), "prefix is by segment, not by characters");
    let of = s.read(|tx| tx.names_of(&records[6].id)).unwrap();
    assert_eq!(of.len(), 2, "the two relation signatures are one definition");
}

fn duplicate_names_are_refused<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let r = s.add(&records[..1], &[("sample.add".into(), 0, Visibility::Public)]);
    assert!(matches!(r, Err(StoreError::NameExists(_))));
    assert!(matches!(s.add(&records[..1], &[("bad path".into(), 0, Visibility::Public)]), Err(StoreError::InvalidPath { .. })));
}

fn rename_keeps_content_and_lineage<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let before = s.resolve("sample.double").unwrap().unwrap();
    let seq = s.rename("sample.double", "sample.twice", Some(before.seq)).unwrap();
    assert!(s.resolve("sample.double").unwrap().is_none());
    let after = s.resolve("sample.twice").unwrap().unwrap();
    assert_eq!(after.target, Target::Content(records[11].id));
    assert_eq!(after.seq, seq);
    let lineages = s.read(|tx| tx.lineages_of(&records[11].id)).unwrap();
    assert_eq!(lineages, vec![records[11].id]);
}

fn update_moves_name_extends_lineage_reports_dependants<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let name = s.resolve("sample.two").unwrap().unwrap();
    let new = add_one_more(2);
    let rep = s.update("sample.two", std::slice::from_ref(&new), 0, name.seq).unwrap();
    assert!(!rep.unchanged);
    assert_eq!(rep.old, Target::Content(records[15].id));
    assert_eq!(s.resolve("sample.two").unwrap().unwrap().target, Target::Content(new.id));
    let lin = s.read(|tx| tx.lineage(&records[15].id)).unwrap();
    assert_eq!(lin.len(), 2);
    assert_eq!(lin[1].id, new.id);
    assert_eq!(lin[1].supersedes, Some(records[15].id));

    // Updating add reports its dependants (the test) as still using the old add.
    let add = s.resolve("sample.add").unwrap().unwrap();
    let rep = s.update("sample.add", &[add_one_more(7)], 0, add.seq).unwrap();
    assert_eq!(rep.dependants_of_old, vec![records[10].id]);
}

fn update_is_optimistic_and_reingest_is_a_noop<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let name = s.resolve("sample.two").unwrap().unwrap();
    let r = s.update("sample.two", &[add_one_more(1)], 0, name.seq + 99);
    assert!(matches!(r, Err(StoreError::Conflict { .. })));
    let seq = s.last_seq().unwrap();
    let rep = s.update("sample.two", &records[15..16], 0, name.seq).unwrap();
    assert!(rep.unchanged);
    assert_eq!(rep.seq, name.seq);
    assert_eq!(s.last_seq().unwrap(), seq, "a no-op writes nothing");
}

fn recursive_groups_share_a_component<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let comp = s.read(|tx| tx.component(&records[2].component)).unwrap();
    assert_eq!(comp.len(), 2);
    assert!(comp[0].index <= comp[1].index);
}

fn incomplete_definitions_store<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let r = s.record(&records[4].id).unwrap().unwrap();
    assert_eq!(r.status, sophia_hash::Status::Incomplete);
}

fn methods_change_no_ids_and_stale_facts<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let plus = s.new_nominal("sophialang", "generic", "sample.plus").unwrap();
    assert_eq!(s.resolve("sample.plus").unwrap().unwrap().target, Target::Nominal(plus));
    let v0 = s.relation_version(&plus).unwrap();
    s.add_method(&plus, &records[11].id).unwrap();
    let v1 = s.relation_version(&plus).unwrap();
    assert!(v1 > v0);
    assert_eq!(s.methods(&plus).unwrap(), vec![records[11].id]);

    let fact = Fact::new(
        "test-result",
        records[10].id,
        b"pass".to_vec(),
        ("sophia-test", "0"),
        vec![Input::Content(records[10].id), Input::Relation { nominal: plus, version: v1 }],
        1,
    );
    s.record_fact(&fact).unwrap();
    assert!(!s.is_stale(&fact).unwrap(), "a cached result is reused while nothing changed");
    assert_eq!(s.read(|tx| tx.facts_of(&records[10].id)).unwrap(), vec![fact.clone()]);

    s.add_method(&plus, &records[1].id).unwrap();
    assert!(s.is_stale(&fact).unwrap(), "a new method makes the fact stale");
    assert_eq!(s.record(&records[11].id).unwrap().unwrap().id, records[11].id, "no definition changed");

    s.remove_method(&plus, &records[1].id).unwrap();
    assert_eq!(s.methods(&plus).unwrap(), vec![records[11].id]);
}

fn annotations_attach_to_subjects<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let a = Annotation::new("doc", vec![records[1].id], b"Adds two naturals.".to_vec(), None);
    s.annotate(&a).unwrap();
    let p = Annotation::new("proves:memory", vec![records[12].id], b"evidence".to_vec(), Some(("sophia-own", "0")));
    s.annotate(&p).unwrap();
    assert_eq!(s.read(|tx| tx.annotations_of(&records[1].id)).unwrap(), vec![a]);
    assert_eq!(s.read(|tx| tx.annotations_of(&records[12].id)).unwrap(), vec![p]);
}

fn changes_since_pages_through_the_log<B: Backend>(s: Store<B>) {
    let start = s.last_seq().unwrap();
    populated(&s);
    let mut seen = Vec::new();
    let mut at = start;
    loop {
        let c = s.changes_since(at, 7).unwrap();
        seen.extend(c.entries.iter().map(|e| e.seq));
        at = c.next;
        if !c.more {
            break;
        }
    }
    let last = s.last_seq().unwrap();
    assert_eq!(seen, ((start + 1)..=last).collect::<Vec<_>>());
    let ops: Vec<LogOp> = s.changes_since(start, 1000).unwrap().entries.iter().map(|e| e.op).collect();
    assert!(ops.contains(&LogOp::AddDefinition) && ops.contains(&LogOp::AddName));
}

fn meta_round_trips<B: Backend>(s: Store<B>) {
    assert_eq!(s.meta("k").unwrap(), None);
    s.set_meta("k", b"v1").unwrap();
    s.set_meta("k", b"v2").unwrap();
    assert_eq!(s.meta("k").unwrap(), Some(b"v2".to_vec()));
}

fn failed_writes_roll_back<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let seq = s.last_seq().unwrap();
    // The second name collides: the whole call must leave no trace.
    let r = s.add(
        &[add_one_more(42)],
        &[("fresh.name".into(), 0, Visibility::Public), ("sample.add".into(), 0, Visibility::Public)],
    );
    assert!(r.is_err());
    assert_eq!(s.last_seq().unwrap(), seq);
    assert!(s.resolve("fresh.name").unwrap().is_none());
    assert!(s.record(&add_one_more(42).id).unwrap().is_none());
    let _ = records;
}

macro_rules! suite {
    ($module:ident, $make:expr) => {
        mod $module {
            use super::*;
            #[allow(unused)]
            fn store() -> (Store<impl Backend>, tempfile::TempDir) {
                let dir = tempfile::tempdir().unwrap();
                let make = $make;
                (Store::new(make(dir.path())), dir)
            }
            suite!(@tests re_putting_writes_nothing, records_round_trip_and_verify, names_resolve_and_list,
                duplicate_names_are_refused, rename_keeps_content_and_lineage,
                update_moves_name_extends_lineage_reports_dependants, update_is_optimistic_and_reingest_is_a_noop,
                recursive_groups_share_a_component, incomplete_definitions_store, methods_change_no_ids_and_stale_facts,
                annotations_attach_to_subjects, changes_since_pages_through_the_log, meta_round_trips,
                failed_writes_roll_back);
        }
    };
    (@tests $($t:ident),*) => {
        $(#[test] fn $t() { let (s, _d) = store(); super::$t(s); })*
    };
}

suite!(sqlite, |p: &std::path::Path| sophia_store::sqlite::Sqlite::open(p.join("store.sqlite")).unwrap());
suite!(redb, |p: &std::path::Path| sophia_store::redb_store::Redb::open(p.join("store.redb")).unwrap());

#[test]
fn sql_queries_are_read_only_and_capped() {
    use sophia_store::query::{QueryLimits, Value};
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("q.sqlite")).unwrap());
    populated(&s);
    let db = s.backend();

    let t = db.query_readonly("SELECT path, target_id FROM names ORDER BY path", QueryLimits::default()).unwrap();
    assert_eq!(t.columns, vec!["path", "target_id"]);
    assert_eq!(t.rows[0][0], Value::Text("sample.CircleLayout".into()));

    let t = db
        .query_readonly(
            "SELECT d.kind, count(*) FROM definitions d JOIN refs r ON r.from_id = d.id GROUP BY d.kind ORDER BY 1",
            QueryLimits::default(),
        )
        .unwrap();
    assert!(!t.rows.is_empty());

    let capped = db.query_readonly("SELECT * FROM log", QueryLimits { max_rows: 3, ..Default::default() }).unwrap();
    assert_eq!(capped.rows.len(), 3);
    assert!(capped.truncated);

    for bad in [
        "DELETE FROM t_name",
        "SELECT * FROM t_definition",
        "INSERT INTO t_meta VALUES ('a', x'00')",
        "PRAGMA journal_mode",
        "ATTACH DATABASE 'x' AS y",
        "CREATE TABLE z(a)",
    ] {
        assert!(matches!(db.query_readonly(bad, QueryLimits::default()), Err(StoreError::QueryRefused(_))), "{bad}");
    }

    let long = db.query_readonly(
        "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c) SELECT count(*) FROM c",
        QueryLimits { max_steps_k: 10, ..Default::default() },
    );
    assert!(long.is_err(), "the step budget aborts runaway queries");

    // The store is still usable afterwards.
    assert!(s.resolve("sample.add").unwrap().is_some());
}

fn settings_are_listed_and_logged<B: Backend>(s: Store<B>) {
    let before = s.last_seq().unwrap();
    let seq = s
        .set_settings("profile/mine", &[("profile/mine/kw.function".into(), Some("fn".into())), ("profile/mine/blocks".into(), Some("braces".into()))])
        .unwrap();
    assert_eq!(seq, before + 1);
    let rows = s.meta_prefix("profile/mine/").unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], ("profile/mine/blocks".to_string(), b"braces".to_vec()));
    s.set_settings("profile/mine", &[("profile/mine/blocks".into(), None)]).unwrap();
    assert_eq!(s.meta_prefix("profile/mine/").unwrap().len(), 1);
    assert_eq!(s.changes_since(before, 10).unwrap().entries.iter().filter(|e| e.op == LogOp::Setting).count(), 2);
}

mod settings {
    use super::*;
    #[test]
    fn sqlite() {
        let d = tempfile::tempdir().unwrap();
        settings_are_listed_and_logged(Store::new(sophia_store::sqlite::Sqlite::open(d.path().join("s")).unwrap()));
    }
    #[test]
    fn redb() {
        let d = tempfile::tempdir().unwrap();
        settings_are_listed_and_logged(Store::new(sophia_store::redb_store::Redb::open(d.path().join("r")).unwrap()));
    }
}

/// Review demonstration 3 (store level): a method table changes, and an
/// earlier world snapshot still gives the original members.
fn worlds_are_reproducible<B: Backend>(s: Store<B>) {
    let (_, records) = populated(&s);
    let g = s.new_nominal("sophialang", "generic", "sample.area").unwrap();
    s.add_method(&g, &records[11].id).unwrap();
    let w1 = s.snapshot_world().unwrap();
    assert_eq!(s.snapshot_world().unwrap(), w1, "same tables, same world");
    s.add_method(&g, &records[1].id).unwrap();
    let w2 = s.snapshot_world().unwrap();
    assert_ne!(w1, w2);
    assert_eq!(s.world_members(&w1, &g).unwrap(), vec![records[11].id]);
    let mut now = s.world_members(&w2, &g).unwrap();
    now.sort();
    let mut expect = vec![records[11].id, records[1].id];
    expect.sort();
    assert_eq!(now, expect);
    s.remove_method(&g, &records[1].id).unwrap();
    assert_eq!(s.snapshot_world().unwrap(), w1, "back to the same tables, back to the same world");
}

mod worlds {
    use super::*;
    #[test]
    fn sqlite() {
        let d = tempfile::tempdir().unwrap();
        worlds_are_reproducible(Store::new(sophia_store::sqlite::Sqlite::open(d.path().join("s")).unwrap()));
    }
    #[test]
    fn redb() {
        let d = tempfile::tempdir().unwrap();
        worlds_are_reproducible(Store::new(sophia_store::redb_store::Redb::open(d.path().join("r")).unwrap()));
    }
}
