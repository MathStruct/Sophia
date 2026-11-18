//! Review probes for B-016 (SP-010 step 1), written by Mistral's review of the
//! change (2026-11-18) and kept as regression tests: every op and the folder
//! paths invalidate the readers of what they change, eviction leaves only
//! current entries, and a store without epochs never gives a stale hit.

use std::collections::BTreeSet;

use sophia_hash::{Digest, NominalId, Record, Status};
use sophia_query::{Cache, Limits, Strategy, Val, prepare_text, run_cached, snapshot};
use sophia_store::{Backend, Store, Target, Visibility};

fn id(i: u64) -> Digest {
    Digest::of(&i.to_le_bytes())
}

fn fake(i: u64, kind: &str, refs: Vec<Digest>) -> Record {
    Record {
        id: id(i),
        lang: "test".into(),
        normaliser: "test@0".into(),
        kind: kind.into(),
        component: id(i),
        index: 0,
        component_size: 1,
        bytes: i.to_le_bytes().to_vec(),
        display: Vec::new(),
        status: Status::Complete,
        refs,
        nominal_refs: Vec::new(),
    }
}

fn store() -> (Store<sophia_store::sqlite::Sqlite>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("s.sqlite")).unwrap()), dir)
}

/// populate: n records with a few refs, names on every third, one generic with methods.
fn populate(s: &Store<sophia_store::sqlite::Sqlite>, n: u64) -> NominalId {
    let records: Vec<Record> = (0..n)
        .map(|i| {
            let mut refs = vec![id((i + 1) % n), id((i + 7) % n)];
            refs.retain(|d| *d != id(i));
            refs.sort();
            refs.dedup();
            fake(i, "method", refs)
        })
        .collect();
    let names: Vec<(String, usize, Visibility)> = (0..n as usize).step_by(3).map(|i| (format!("g.n{i}"), i, Visibility::Public)).collect();
    s.add(&records, &names).unwrap();
    let g = s.new_nominal("test", "generic", "g.f").unwrap();
    for i in (0..n).step_by(5) {
        s.add_method(&g, &id(i)).unwrap();
    }
    g
}

fn run(s: &Store<sophia_store::sqlite::Sqlite>, p: &sophia_query::Prepared, params: &[Val]) -> (sophia_query::Rows, Cache) {
    let (rows, _rid, cache) = run_cached(s, p, params, Limits::default()).unwrap();
    (rows, cache)
}

fn fresh(s: &Store<sophia_store::sqlite::Sqlite>, p: &sophia_query::Prepared, params: &[Val]) -> sophia_query::Rows {
    s.read(|tx| p.run(tx, params, Limits::default(), Strategy::Auto).map_err(|e| sophia_store::StoreError::Backend(e.to_string())))
        .unwrap()
        .sorted()
}

fn same_rows(a: &sophia_query::Rows, b: &sophia_query::Rows) -> bool {
    a.clone().sorted().rows == b.clone().sorted().rows
}

fn epoch(s: &Store<sophia_store::sqlite::Sqlite>, r: &str) -> u64 {
    s.read(|tx| sophia_store::logic::relation_epoch(tx, r)).unwrap()
}

type Op = Box<dyn Fn(&Store<sophia_store::sqlite::Sqlite>)>;

/// Every mutating op must invalidate (Miss) the queries that read the relation
/// it changes, and the rows must equal a fresh evaluation — a Hit with old rows
/// would be the stale-hit defect.
#[test]
fn every_op_invalidates_its_readers_without_stale_rows() {
    let cases: Vec<(&str, &str, Op)> = vec![
        (
            "select a, b where refs(a, b)",
            "add-definition",
            Box::new(|s| {
                s.add(&[fake(5000, "method", vec![id(1)])], &[]).unwrap();
            }),
        ),
        (
            "select d, k where def(d, k)",
            "add-definition",
            Box::new(|s| {
                s.add(&[fake(5001, "data", vec![])], &[]).unwrap();
            }),
        ),
        (
            "select d, n where uses(d, n)",
            "add-definition-with-nominal-ref",
            Box::new(|s| {
                let g = s.resolve("g.f").unwrap().unwrap();
                let Target::Nominal(g) = g.target else { panic!() };
                let mut r = fake(5002, "method", vec![]);
                r.nominal_refs = vec![g];
                s.add(&[r], &[]).unwrap();
            }),
        ),
        (
            "select p, d where name(p, d)",
            "alias",
            Box::new(|s| {
                s.alias("g.alias", Target::Content(id(1)), Visibility::Public).unwrap();
            }),
        ),
        (
            "select p, d where name(p, d)",
            "remove-name",
            Box::new(|s| {
                s.remove("g.n0", None).unwrap();
            }),
        ),
        (
            "select p, d where name(p, d)",
            "rename",
            Box::new(|s| {
                s.rename("g.n3", "g.n3moved", None).unwrap();
            }),
        ),
        (
            "select p, d where name(p, d)",
            "update-move-name",
            Box::new(|s| {
                let seq = s.resolve("g.n6").unwrap().unwrap().seq;
                s.update("g.n6", &[fake(5003, "method", vec![])], 0, seq).unwrap();
            }),
        ),
        (
            "select n, d where member(n, d)",
            "add-method",
            Box::new(|s| {
                let g = s.resolve("g.f").unwrap().unwrap();
                let Target::Nominal(g) = g.target else { panic!() };
                s.add_method(&g, &id(3)).unwrap();
            }),
        ),
        (
            "select n, d where member(n, d)",
            "remove-method",
            Box::new(|s| {
                let g = s.resolve("g.f").unwrap().unwrap();
                let Target::Nominal(g) = g.target else { panic!() };
                s.remove_method(&g, &id(0)).unwrap();
            }),
        ),
        (
            "select n, k where nominal(n, k)",
            "new-nominal",
            Box::new(|s| {
                s.new_nominal("test", "generic", "g.g2").unwrap();
            }),
        ),
        (
            "select a, k, s where annotation(a, k, s)",
            "annotate",
            Box::new(|s| {
                s.annotate(&sophia_store::Annotation::new("note", vec![id(1)], b"hi".to_vec(), None)).unwrap();
            }),
        ),
        (
            "select f, k, s where fact(f, k, s)",
            "fact",
            Box::new(|s| {
                s.record_fact(&sophia_store::Fact::new("typing", id(1), b"ok".to_vec(), ("test", "0"), vec![], 0)).unwrap();
            }),
        ),
    ];
    for (query, op, do_op) in cases {
        let (s, _d) = store();
        populate(&s, 12);
        let p = prepare_text(query).unwrap();
        let (r1, c1) = run(&s, &p, &[]);
        assert_eq!(c1, Cache::Miss, "{query}/{op} first run");
        let (_, c2) = run(&s, &p, &[]);
        assert_eq!(c2, Cache::Hit, "{query}/{op} second run");
        assert!(same_rows(&r1, &fresh(&s, &p, &[])), "{query}/{op} cached rows == fresh");
        do_op(&s);
        let (r3, c3) = run(&s, &p, &[]);
        assert_eq!(c3, Cache::Miss, "{query}: after {op} the run must miss (got {c3:?})");
        assert!(same_rows(&r3, &fresh(&s, &p, &[])), "{query}: after {op} rows must equal a fresh run");
    }
}

/// A method change rewrites the nominal row's version, but `nominal(n, kind)`
/// does not show versions (SPEC-007 §2), so it bumps only `member`, and a
/// `nominal` reader keeps a result that is still correct. If `nominal` ever
/// exposes the version, `LogOp::relations` must add it for method ops.
#[test]
fn method_changes_keep_nominal_results() {
    let (s, _d) = store();
    let g = populate(&s, 12);
    let p = prepare_text("select n, k where nominal(n, k)").unwrap();
    let (_, c1) = run(&s, &p, &[]);
    assert_eq!(c1, Cache::Miss);
    let (_, c2) = run(&s, &p, &[]);
    assert_eq!(c2, Cache::Hit);
    let epoch_before = epoch(&s, "nominal");
    let version_before = s.relation_version(&g).unwrap();
    s.add_method(&g, &id(3)).unwrap();
    let epoch_after = epoch(&s, "nominal");
    let version_after = s.relation_version(&g).unwrap();
    assert_ne!(version_before, version_after, "add_method rewrote the nominal row");
    assert_eq!(epoch_after, epoch_before, "a method change does not bump the nominal epoch");
    let (rows, c3) = run(&s, &p, &[]);
    assert_eq!(c3, Cache::Hit, "a nominal-reading query is not invalidated by a method change");
    assert!(same_rows(&rows, &fresh(&s, &p, &[])), "nominal(n, kind) is unchanged by a version bump, so the hit is not stale today");
}

/// A magic-set query's read set is exactly the base relations of the original
/// query; a change to a relation it does not read keeps its cached result.
#[test]
fn magic_query_reads_only_its_own_relations() {
    let (s, _d) = store();
    populate(&s, 30);
    let p =
        prepare_text("query reach(t::Id)\n dep(a, b) = refs(a, b)\n dep(a, c) = refs(a, b) && dep(b, c)\n select a where dep(a, t)\nend")
            .unwrap();
    assert!(p.magic.is_some());
    assert_eq!(p.reads(), BTreeSet::from([sophia_query::ir::Base::Refs]), "reach reads only refs");
    let params = [Val::Id(id(2))];
    let (_, c1) = run(&s, &p, &params);
    assert_eq!(c1, Cache::Miss);
    let (_, c2) = run(&s, &p, &params);
    assert_eq!(c2, Cache::Hit);
    // A name change: kept.
    s.alias("g.other", Target::Content(id(4)), Visibility::Public).unwrap();
    let (r3, c3) = run(&s, &p, &params);
    assert_eq!(c3, Cache::Hit, "refs-only query keeps its cache across a name change");
    assert!(same_rows(&r3, &fresh(&s, &p, &params)));
    // A definition change: invalidated, correct rows.
    s.add(&[fake(6000, "method", vec![id(2)])], &[]).unwrap();
    let (r4, c4) = run(&s, &p, &params);
    assert_eq!(c4, Cache::Miss, "refs-only query misses after a definition change");
    assert!(same_rows(&r4, &fresh(&s, &p, &params)));
    assert!(r4.rows.contains(&vec![Val::Id(id(6000))]), "the new dependant is in the rows");
}

/// After a miss that writes, every remaining `query-result/` key must be in the
/// current layout and carry the current snapshot for its read set; keys in
/// older or malformed layouts must be gone.
#[test]
fn eviction_leaves_only_current_layout_and_snapshots() {
    let (s, _d) = store();
    populate(&s, 12);
    let refs = prepare_text("select a, b where refs(a, b)").unwrap();
    let names = prepare_text("select p, d where name(p, d)").unwrap();
    run(&s, &refs, &[]);
    run(&s, &names, &[]);
    assert_eq!(s.meta_prefix("query-result/").unwrap().len(), 2);
    // Plant keys of older and malformed layouts.
    let bogus64 = "ab".repeat(32);
    let snap_refs = s.read(|tx| snapshot(tx, &refs.reads()).map_err(|e| sophia_store::StoreError::Backend(e.to_string()))).unwrap();
    let tag_refs = sophia_query::read_set_tag(&refs.reads());
    s.set_meta(&format!("query-result/{bogus64}/{bogus64}"), b"legacy-v1").unwrap(); // old 2-segment layout
    s.set_meta(&format!("query-result/{bogus64}"), b"legacy-v0").unwrap(); // old 1-segment layout
    s.set_meta(&format!("query-result/zz/{}/{bogus64}", snap_refs.to_hex()), b"bad-tag").unwrap();
    s.set_meta(&format!("query-result/{tag_refs}/{bogus64}/{bogus64}"), b"wrong-snap").unwrap();
    s.set_meta(&format!("query-result/{tag_refs}/{}/{bogus64}/extra", snap_refs.to_hex()), b"extra-seg").unwrap();
    s.set_meta("query-result/", b"empty-rest").unwrap();
    // A miss that writes: a definition changes refs.
    s.add(&[fake(7000, "method", vec![id(1)])], &[]).unwrap();
    let (_, c) = run(&s, &refs, &[]);
    assert_eq!(c, Cache::Miss);
    let entries = s.meta_prefix("query-result/").unwrap();
    let mut tags = BTreeSet::new();
    for (k, v) in &entries {
        assert!(k.starts_with("query-result/"), "{k}");
        assert_ne!(v.as_slice(), b"legacy-v1");
        let rest = &k["query-result/".len()..];
        let seg: Vec<&str> = rest.split('/').collect();
        assert_eq!(seg.len(), 3, "key {k} is not in the 3-segment layout");
        assert_eq!(seg[0].len(), 2, "tag {k}");
        u8::from_str_radix(seg[0], 16).unwrap();
        let bits = u8::from_str_radix(seg[0], 16).unwrap();
        let reads: BTreeSet<sophia_query::ir::Base> =
            sophia_query::ir::Base::ALL.iter().enumerate().filter(|(i, _)| bits & 1 << i != 0).map(|(_, b)| *b).collect();
        let current = s.read(|tx| snapshot(tx, &reads).map_err(|e| sophia_store::StoreError::Backend(e.to_string()))).unwrap();
        assert_eq!(seg[1], current.to_hex(), "key {k} keeps a non-current snapshot");
        tags.insert(seg[0].to_string());
    }
    assert_eq!(entries.len(), 2, "refs + names entries: {:?}", entries.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>());
    assert!(tags.contains(&tag_refs));
    // The name entry survived (name unchanged) and is still a hit.
    let (_, c) = run(&s, &names, &[]);
    assert_eq!(c, Cache::Hit);
}

/// A store written before the change (log entries, no `relation-epoch/` keys,
/// cached results in an older layout) must never give a stale hit.
#[test]
fn pre_epoch_store_never_stale_hits() {
    let (s, _d) = store();
    populate(&s, 12);
    // Make it a pre-change store: strip every epoch key.
    for (k, _) in s.meta_prefix(sophia_store::logic::EPOCH_PREFIX).unwrap() {
        let k = k.clone();
        s.backend().write(|tx| tx.remove_meta(&k)).unwrap();
    }
    assert!(s.meta_prefix(sophia_store::logic::EPOCH_PREFIX).unwrap().is_empty());
    let refs = prepare_text("select a, b where refs(a, b)").unwrap();
    let notes = prepare_text("select a, k, s where annotation(a, k, s)").unwrap();
    // Plant old-layout cache entries with garbage payloads under the ids the old
    // (v0/v1) layouts would have used.
    sophia_query::ensure_store_id(&s).unwrap();
    let store_id = s.meta("store-id").unwrap().unwrap();
    let seq = s.last_seq().unwrap();
    let mut w = sophia_hash::canonical::Writer::new();
    w.str("sophia", "snapshot-v0");
    w.field("store", &store_id);
    w.u64("seq", seq);
    let snap_v0 = Digest::of(&w.finish());
    let rid = sophia_query::canon::result_id(&refs.id, &snap_v0, &[]);
    s.set_meta(&format!("query-result/{}/{}", snap_v0.to_hex(), rid.to_hex()), b"garbage-v1").unwrap();
    s.set_meta(&format!("query-result/{}", rid.to_hex()), b"garbage-v0").unwrap();
    // A run must miss (not read the garbage) and be correct.
    let (r1, c1) = run(&s, &refs, &[]);
    assert_eq!(c1, Cache::Miss, "an old-layout key must not be hit");
    assert!(same_rows(&r1, &fresh(&s, &refs, &[])));
    // The miss's eviction removed the old-layout keys.
    for (k, v) in s.meta_prefix("query-result/").unwrap() {
        assert_ne!(v, b"garbage-v1");
        assert_ne!(v, b"garbage-v0");
        assert_eq!(k.split('/').count(), 4, "key {k}");
    }
    // Coarse but never stale: a name change (logged now, initializing epochs) keeps
    // the refs result, whose rows are still correct.
    s.alias("g.late", Target::Content(id(2)), Visibility::Public).unwrap();
    let (r2, c2) = run(&s, &refs, &[]);
    assert_eq!(c2, Cache::Hit, "refs unchanged by a name change, in a pre-epoch store");
    assert!(same_rows(&r2, &fresh(&s, &refs, &[])));
    // An annotation change invalidates the annotation reader even in a pre-epoch store.
    let (_, cn1) = run(&s, &notes, &[]);
    assert_eq!(cn1, Cache::Miss);
    let a = sophia_store::Annotation::new("note", vec![id(1)], b"hi".to_vec(), None);
    s.annotate(&a).unwrap();
    let (r3, cn2) = run(&s, &notes, &[]);
    assert_eq!(cn2, Cache::Miss, "annotation changed, so the annotation query must miss");
    assert!(same_rows(&r3, &fresh(&s, &notes, &[])));
    assert!(r3.rows.contains(&vec![Val::Id(a.id), Val::Text("note".into()), Val::Id(id(1))]));
}

/// Two queries with the same read set and snapshot but different parameters
/// coexist under one read-set/snapshot prefix.
#[test]
fn same_read_set_different_params_coexist() {
    let (s, _d) = store();
    populate(&s, 12);
    let p =
        prepare_text("query reach(t::Id)\n dep(a, b) = refs(a, b)\n dep(a, c) = refs(a, b) && dep(b, c)\n select a where dep(a, t)\nend")
            .unwrap();
    for t in [0u64, 1, 2] {
        let (r, c) = run(&s, &p, &[Val::Id(id(t))]);
        assert_eq!(c, Cache::Miss, "t={t}");
        let (_, c2) = run(&s, &p, &[Val::Id(id(t))]);
        assert_eq!(c2, Cache::Hit, "t={t}");
        assert!(same_rows(&r, &fresh(&s, &p, &[Val::Id(id(t))])));
    }
    assert_eq!(s.meta_prefix("query-result/").unwrap().len(), 3);
}

/// The folder-ingest path (`replace_files`): every base-data change it makes is
/// logged under an op that covers the relation, so cached queries reading those
/// relations miss and return fresh rows.
#[test]
fn folder_replace_files_invalidates_its_readers() {
    let (s, _d) = store();
    let r0 = fake(100, "method", vec![]);
    let r1 = fake(101, "method", vec![]);
    let f1 = sophia_store::FileIngest {
        path: "a.jl".into(),
        text: Some("v1".into()),
        normaliser: "test@0".into(),
        records: vec![r0, r1.clone()],
        names: vec![("f.a".into(), 0)],
        generics: vec![sophia_store::FileGeneric { path: "g.f".into(), lang: "test".into(), members: vec![1] }],
        ..Default::default()
    };
    s.replace_files(&[f1]).unwrap();
    let names = prepare_text("select p, d where name(p, d)").unwrap();
    let members = prepare_text("select n, d where member(n, d)").unwrap();
    let defs = prepare_text("select d, k where def(d, k)").unwrap();
    let refs = prepare_text("select a, b where refs(a, b)").unwrap();
    for p in [&names, &members, &defs, &refs] {
        let (_, c1) = run(&s, p, &[]);
        assert_eq!(c1, Cache::Miss);
        let (_, c2) = run(&s, p, &[]);
        assert_eq!(c2, Cache::Hit, "{}", p.print(sophia_query::Profile::Julia));
    }
    // Replace the file: the name moves, the generic's member changes, a new
    // record with a ref appears.
    let r2 = fake(102, "method", vec![id(101)]);
    let f2 = sophia_store::FileIngest {
        path: "a.jl".into(),
        text: Some("v2".into()),
        normaliser: "test@0".into(),
        records: vec![r2, r1],
        names: vec![("f.b".into(), 0)],
        generics: vec![sophia_store::FileGeneric { path: "g.f".into(), lang: "test".into(), members: vec![0] }],
        ..Default::default()
    };
    s.replace_files(&[f2]).unwrap();
    let g = s.resolve("g.f").unwrap().unwrap();
    let Target::Nominal(g) = g.target else { panic!("g.f is not a nominal") };
    let (rn, cn) = run(&s, &names, &[]);
    assert_eq!(cn, Cache::Miss, "name changed in the folder");
    assert!(same_rows(&rn, &fresh(&s, &names, &[])));
    assert!(rn.rows.contains(&vec![Val::Text("f.b".into()), Val::Id(id(102))]));
    assert!(!rn.rows.iter().any(|r| r[0] == Val::Text("f.a".into())));
    let (rm, cm) = run(&s, &members, &[]);
    assert_eq!(cm, Cache::Miss, "member changed in the folder");
    assert!(same_rows(&rm, &fresh(&s, &members, &[])));
    assert!(rm.rows.contains(&vec![Val::Id(g), Val::Id(id(102))]));
    assert!(!rm.rows.iter().any(|r| r == &vec![Val::Id(g), Val::Id(id(101))]));
    let (rd, cd) = run(&s, &defs, &[]);
    assert_eq!(cd, Cache::Miss, "def changed in the folder");
    assert!(same_rows(&rd, &fresh(&s, &defs, &[])));
    assert!(rd.rows.contains(&vec![Val::Id(id(102)), Val::Text("method".into())]));
    let (rr, cr) = run(&s, &refs, &[]);
    assert_eq!(cr, Cache::Miss, "refs changed in the folder");
    assert!(same_rows(&rr, &fresh(&s, &refs, &[])));
    assert!(rr.rows.contains(&vec![Val::Id(id(102)), Val::Id(id(101))]));
}

/// The collection path (`RemoveDefinition`, logged from `folder.rs::collect`):
/// records a derived store no longer reaches are removed, and cached queries
/// reading def/refs/uses must miss and return fresh rows.
#[test]
fn collected_definitions_invalidate_their_readers() {
    let (s, _d) = store();
    s.set_derived().unwrap();
    let r0 = fake(200, "method", vec![id(201)]);
    let r1 = fake(201, "method", vec![]);
    let f1 = sophia_store::FileIngest {
        path: "a.jl".into(),
        text: Some("v1".into()),
        normaliser: "test@0".into(),
        records: vec![r0, r1],
        names: vec![("f.a".into(), 0)],
        ..Default::default()
    };
    s.replace_files(&[f1]).unwrap();
    let defs = prepare_text("select d, k where def(d, k)").unwrap();
    let refs = prepare_text("select a, b where refs(a, b)").unwrap();
    for p in [&defs, &refs] {
        let (_, c1) = run(&s, p, &[]);
        assert_eq!(c1, Cache::Miss);
        let (_, c2) = run(&s, p, &[]);
        assert_eq!(c2, Cache::Hit);
    }
    // Replace with a file whose record no longer references 201, and drop the
    // name's target too: collection removes the unreached records.
    let r2 = fake(202, "method", vec![]);
    let f2 = sophia_store::FileIngest {
        path: "a.jl".into(),
        text: Some("v2".into()),
        normaliser: "test@0".into(),
        records: vec![r2],
        names: vec![("f.a".into(), 0)],
        ..Default::default()
    };
    s.replace_files(&[f2]).unwrap();
    // In derived mode `replace_in` collects what the files no longer reach,
    // logging `RemoveDefinition` per record.
    assert!(s.record(&id(200)).unwrap().is_none(), "the old target was collected");
    assert!(s.record(&id(201)).unwrap().is_none(), "the unreferenced record was collected");
    let (rd, cd) = run(&s, &defs, &[]);
    assert_eq!(cd, Cache::Miss, "def changed by collection");
    assert!(same_rows(&rd, &fresh(&s, &defs, &[])));
    assert!(!rd.rows.iter().any(|r| r[0] == Val::Id(id(200)) || r[0] == Val::Id(id(201))));
    let (rr, cr) = run(&s, &refs, &[]);
    assert_eq!(cr, Cache::Miss, "refs changed by collection");
    assert!(same_rows(&rr, &fresh(&s, &refs, &[])));
    assert!(!rr.rows.iter().any(|r| r.contains(&Val::Id(id(201)))));
}
