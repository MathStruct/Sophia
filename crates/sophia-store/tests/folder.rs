//! SPEC-009: a folder store, on every backend. `replace_files` replaces
//! everything a set of files contributed in one transaction.

use sophia_hash::merkle::{component_digest, member_digest};
use sophia_hash::{Digest, Record, Status};
use sophia_store::folder::source_record;
use sophia_store::*;

const NORM: &str = "moonkale-wp66@7";

fn rec(body: &str, kind: &str) -> Record {
    let component = component_digest("julia", NORM, body.as_bytes());
    Record {
        id: member_digest(&component, 0),
        lang: "julia".into(),
        normaliser: NORM.into(),
        kind: kind.into(),
        component,
        index: 0,
        component_size: 1,
        bytes: body.as_bytes().to_vec(),
        display: Vec::new(),
        status: Status::Complete,
        refs: Vec::new(),
        nominal_refs: Vec::new(),
    }
}

/// A file of definitions `name = body` (one per line); names starting with
/// `g:` are methods of the generic function `jl.P.g`.
fn file(path: &str, defs: &[(&str, &str)]) -> FileIngest {
    let mut text = String::new();
    let mut f = FileIngest { path: path.into(), normaliser: NORM.into(), ..FileIngest::default() };
    let mut methods = Vec::new();
    for (name, body) in defs {
        let start = text.len() as u32;
        text.push_str(&format!("{name} = {body}\n"));
        let i = f.records.len();
        f.records.push(rec(body, if name.starts_with("g:") { "function_definition" } else { "const_statement" }));
        f.spans.push((i, start, text.len() as u32 - 1));
        if name.starts_with("g:") {
            methods.push(i);
        } else {
            f.names.push((format!("jl.P.{name}"), i));
        }
    }
    if !methods.is_empty() {
        f.generics.push(FileGeneric { path: "jl.P.g".into(), lang: "julia".into(), members: methods });
    }
    f.text = Some(text);
    f
}

fn deleted(path: &str) -> FileIngest {
    FileIngest { path: path.into(), normaliser: NORM.into(), ..FileIngest::default() }
}

fn target<B: Backend>(s: &Store<B>, n: &str) -> Option<Digest> {
    s.resolve(n).unwrap().map(|e| e.target.digest())
}

fn methods<B: Backend>(s: &Store<B>) -> Vec<Digest> {
    match s.resolve("jl.P.g").unwrap().map(|e| e.target) {
        Some(Target::Nominal(g)) => {
            let mut v = s.methods(&g).unwrap();
            v.sort();
            v
        }
        _ => Vec::new(),
    }
}

fn sorted(mut v: Vec<Digest>) -> Vec<Digest> {
    v.sort();
    v
}

fn files_replace_names_generics_and_spans<B: Backend>(s: Store<B>) {
    s.set_derived().unwrap();
    assert!(s.is_derived().unwrap());
    let a = file("src/a.jl", &[("a", "1"), ("b", "2"), ("g:1", "g(x::Int) = 1")]);
    let b = file("src/b.jl", &[("c", "3"), ("g:2", "g(x::Float64) = 2")]);
    let r = s.replace_files(&[a.clone(), b.clone()]).unwrap();
    assert_eq!((r.names_added, r.members_added), (4, 2), "{r:?}");
    assert_eq!(target(&s, "jl.P.a"), Some(rec("1", "const_statement").id));
    assert_eq!(methods(&s), sorted(vec![rec("g(x::Int) = 1", "function_definition").id, rec("g(x::Float64) = 2", "function_definition").id]));
    let m = s.file_manifest("src/a.jl").unwrap().unwrap();
    assert_eq!(m.source, Some(source_record("src/a.jl", a.text.as_ref().unwrap()).id));
    let ts = s.text_source(&rec("2", "const_statement").id, Some("b")).unwrap().unwrap();
    let src = s.record(&ts.source).unwrap().unwrap();
    assert_eq!(&src.bytes[ts.start as usize..ts.end as usize], b"b = 2");
    assert_eq!(ts.path, "src/a.jl");
    assert_eq!(sorted_files(&s), ["src/a.jl", "src/b.jl"]);

    // Re-sending the same files changes nothing.
    let seq = s.last_seq().unwrap();
    let r = s.replace_files(&[a, b.clone()]).unwrap();
    assert_eq!((r.names_added, r.names_moved, r.names_removed, r.members_added, r.members_removed), (0, 0, 0, 0, 0));
    assert_eq!(s.last_seq().unwrap(), seq, "an unchanged batch logs nothing");

    // Edit a.jl: a goes, b changes, d is new, g's first method changes.
    let a2 = file("src/a.jl", &[("b", "20"), ("d", "4"), ("g:1", "g(x::Int) = 10")]);
    let r = s.replace_files(&[a2]).unwrap();
    assert_eq!((r.names_added, r.names_moved, r.names_removed), (1, 1, 1), "{r:?}");
    assert_eq!((r.members_added, r.members_removed), (1, 1));
    assert_eq!(target(&s, "jl.P.a"), None);
    assert_eq!(target(&s, "jl.P.b"), Some(rec("20", "const_statement").id));
    let lineage = s.read(|tx| tx.lineages_of(&rec("20", "const_statement").id)).unwrap();
    assert_eq!(lineage.len(), 1, "the moved name extends its lineage");
    assert_eq!(methods(&s).len(), 2);

    // c moves from b.jl to a.jl: both files in one batch.
    let a3 = file("src/a.jl", &[("b", "20"), ("d", "4"), ("c", "3"), ("g:1", "g(x::Int) = 10")]);
    let b3 = file("src/b.jl", &[("g:2", "g(x::Float64) = 2")]);
    s.replace_files(&[a3.clone(), b3]).unwrap();
    assert_eq!(target(&s, "jl.P.c"), Some(rec("3", "const_statement").id));
    assert_eq!(s.text_source(&rec("3", "const_statement").id, None).unwrap().unwrap().path, "src/a.jl");

    // Claiming a name another file owns, without that file: refused, nothing written.
    let seq = s.last_seq().unwrap();
    let b4 = file("src/b.jl", &[("d", "40"), ("g:2", "g(x::Float64) = 2")]);
    match s.replace_files(&[b4]) {
        Err(StoreError::NameClaimed { name, owner }) => assert_eq!((name.as_str(), owner.as_str()), ("jl.P.d", "src/a.jl")),
        other => panic!("{other:?}"),
    }
    assert_eq!(s.last_seq().unwrap(), seq, "the batch rolled back");
    assert_eq!(target(&s, "jl.P.d"), Some(rec("4", "const_statement").id));

    // Two files of one batch claiming one name: refused before any write.
    let x = file("src/x.jl", &[("e", "5")]);
    let y = file("src/y.jl", &[("e", "6")]);
    assert!(matches!(s.replace_files(&[x, y]), Err(StoreError::NameClaimed { .. })));

    // Deleting b.jl removes its method; deleting a.jl removes the rest.
    s.replace_files(&[deleted("src/b.jl")]).unwrap();
    assert_eq!(methods(&s), vec![rec("g(x::Int) = 10", "function_definition").id]);
    assert_eq!(sorted_files(&s), ["src/a.jl"]);
    s.replace_files(&[deleted("src/a.jl")]).unwrap();
    for n in ["jl.P.b", "jl.P.c", "jl.P.d", "jl.P.g"] {
        assert_eq!(target(&s, n), None, "{n}");
    }
    assert!(sorted_files(&s).is_empty());
}

fn sorted_files<B: Backend>(s: &Store<B>) -> Vec<String> {
    let mut v = s.files().unwrap();
    v.sort();
    v
}

/// The same method text in two files is one record contributed twice: it
/// stays a member until neither file has it.
fn shared_methods_are_counted<B: Backend>(s: Store<B>) {
    let a = file("a.jl", &[("g:1", "g(x) = 0")]);
    let b = file("b.jl", &[("g:1", "g(x) = 0")]);
    s.replace_files(&[a, b]).unwrap();
    assert_eq!(methods(&s).len(), 1);
    s.replace_files(&[deleted("a.jl")]).unwrap();
    assert_eq!(methods(&s).len(), 1, "b.jl still has it");
    s.replace_files(&[deleted("b.jl")]).unwrap();
    assert!(methods(&s).is_empty());
    assert_eq!(target(&s, "jl.P.g"), None);
}

/// What a folder store says about `jl.P.g` and the names of `jl.P`: the
/// state a history replay must agree on with a store built fresh (P-231).
fn state<B: Backend>(s: &Store<B>) -> (Option<Digest>, Vec<Digest>, Vec<(String, Option<Digest>)>) {
    let content = match s.resolve("jl.P.g").unwrap().map(|e| e.target) {
        Some(Target::Content(id)) => Some(id),
        _ => None,
    };
    let names = ["jl.P.a", "jl.P.b"].iter().map(|n| (n.to_string(), target(s, n))).collect();
    (content, methods(s), names)
}

type Tree = std::collections::BTreeMap<String, FileIngest>;

/// Replays `commits` (each a batch of files) into `s` and, after each, checks
/// it against a store built fresh from `tree`, the files so far.
fn replay<B: Backend>(s: &Store<B>, fresh: impl Fn() -> (Store<B>, tempfile::TempDir), tree: &mut Tree, commits: &[Vec<FileIngest>]) {
    for (i, c) in commits.iter().enumerate() {
        s.replace_files(c).unwrap();
        for f in c {
            if f.text.is_some() {
                tree.insert(f.path.clone(), f.clone());
            } else {
                tree.remove(&f.path);
            }
        }
        let (t, _d) = fresh();
        t.replace_files(&tree.values().cloned().collect::<Vec<_>>()).unwrap();
        assert_eq!(state(s), state(&t), "after commit {i}");
    }
}

/// Two methods with one normalised id in one file are one member: a
/// refresh of the file subtracts it once, not twice.
fn repeated_methods_survive_a_refresh<B: Backend>(s: Store<B>, fresh: impl Fn() -> (Store<B>, tempfile::TempDir)) {
    let twice = || file("a.jl", &[("g:1", "g(x) = 0"), ("g:2", "g(x) = 0")]);
    let mut grown = twice();
    grown.records.push(rec("2", "const_statement"));
    grown.names.push(("jl.P.a".into(), 2));
    replay(&s, &fresh, &mut Tree::new(), &[vec![twice()], vec![grown], vec![twice()]]);
    assert_eq!(methods(&s).len(), 1);
}

/// A generic function's name that becomes a definition's loses its
/// methods; when the definition goes, the methods files still contribute
/// come back, from files in the batch and outside it.
fn names_changing_kind_keep_memberships_right<B: Backend>(s: Store<B>, fresh: impl Fn() -> (Store<B>, tempfile::TempDir)) {
    let a = |with_const: bool| {
        let mut defs = vec![("g:1", "g(x) = 0")];
        if with_const {
            defs.push(("g", "1"));
        }
        file("a.jl", &defs)
    };
    let b = file("b.jl", &[("g:2", "g(x, y) = 0")]);
    let tree = &mut Tree::new();
    replay(&s, &fresh, tree, &[vec![a(false), b.clone()]]);
    let Some(Target::Nominal(old)) = s.resolve("jl.P.g").unwrap().map(|e| e.target) else { panic!("a generic function") };
    replay(&s, &fresh, tree, &[vec![a(true), b.clone()]]);
    assert!(s.methods(&old).unwrap().is_empty(), "no orphan memberships");
    replay(&s, &fresh, tree, &[vec![a(false)]]);
    assert_eq!(methods(&s).len(), 2, "a.jl's and b.jl's methods are back");
    // The definition comes back, and goes in a file the batch refreshes.
    replay(&s, &fresh, tree, &[vec![a(true), b.clone()], vec![a(false), b.clone()], vec![a(true), b.clone()], vec![a(false)]]);
}

fn all_records<B: Backend>(s: &Store<B>) -> Vec<Digest> {
    sorted(s.read(|tx| tx.records_of_kind(None, usize::MAX)).unwrap())
}

/// A file whose two definitions form one component (mutual recursion).
fn pair(path: &str, a: &str, b: &str) -> FileIngest {
    let text = format!("{a}\n{b}\n");
    let mut f = FileIngest { path: path.into(), normaliser: NORM.into(), text: Some(text.clone()), ..FileIngest::default() };
    let component = component_digest("julia", NORM, text.as_bytes());
    for (i, body) in [a, b].iter().enumerate() {
        let mut r = rec(body, "function_definition");
        r.component = component;
        r.index = i as u32;
        r.component_size = 2;
        r.id = member_digest(&component, i as u32);
        f.records.push(r);
        f.names.push((format!("jl.P.{}", &body[..1]), i));
    }
    f.spans = vec![(0, 0, a.len() as u32), (1, a.len() as u32 + 1, text.len() as u32 - 1)];
    f
}

/// A history: edits, a move between files, a method replaced, identical
/// text at two paths, a component, deletions.
fn history() -> Vec<Vec<FileIngest>> {
    vec![
        vec![file("a.jl", &[("a", "1"), ("b", "2"), ("g:1", "g(x) = 0")]), file("b.jl", &[("c", "3")]), pair("p.jl", "e() = f()", "f() = e()")],
        vec![file("a.jl", &[("a", "10"), ("b", "2"), ("g:1", "g(x) = 1")])],
        vec![file("a.jl", &[("a", "10")]), file("b.jl", &[("c", "3"), ("b", "2")])],
        vec![file("c.jl", &[("c", "3")]), file("b.jl", &[("b", "2")])],
        vec![deleted("p.jl"), file("c.jl", &[("d", "3")])],
        vec![deleted("a.jl"), deleted("b.jl")],
    ]
}

/// Files are the truth: what no current file reaches is collected as the
/// files change (SR-146), so a replay ends with a fresh build's records.
fn files_truth_collects_what_files_no_longer_reach<B: Backend>(s: Store<B>, fresh: impl Fn() -> (Store<B>, tempfile::TempDir)) {
    s.set_derived().unwrap();
    let mut tree = Tree::new();
    let mut removed = 0;
    for (i, c) in history().into_iter().enumerate() {
        removed += s.replace_files(&c).unwrap().records_removed;
        for f in c {
            if f.text.is_some() { tree.insert(f.path.clone(), f); } else { tree.remove(&f.path); }
        }
        let (t, _d) = fresh();
        t.set_derived().unwrap();
        t.replace_files(&tree.values().cloned().collect::<Vec<_>>()).unwrap();
        assert_eq!(all_records(&s), all_records(&t), "after commit {i}");
        assert_eq!(state(&s), state(&t), "after commit {i}");
    }
    assert!(removed > 0);
    assert_eq!(s.collect_folder().unwrap(), 0, "nothing left to sweep");
}

/// The store is the truth: nothing is removed; the graph keeps every
/// version. Marking it derived later and sweeping once leaves what a fresh
/// build has.
fn store_truth_keeps_history_until_swept<B: Backend>(s: Store<B>, fresh: impl Fn() -> (Store<B>, tempfile::TempDir)) {
    let mut tree = Tree::new();
    let mut seen = std::collections::BTreeSet::new();
    for c in history() {
        assert_eq!(s.replace_files(&c).unwrap().records_removed, 0);
        for f in c {
            seen.extend(f.records.iter().map(|r| r.id));
            if f.text.is_some() { tree.insert(f.path.clone(), f); } else { tree.remove(&f.path); }
        }
    }
    let kept: std::collections::BTreeSet<Digest> = all_records(&s).into_iter().collect();
    assert!(seen.is_subset(&kept), "every version stays");
    assert!(matches!(s.collect_folder(), Err(StoreError::NotDerived)));
    s.set_derived().unwrap();
    assert!(s.collect_folder().unwrap() > 0);
    let (t, _d) = fresh();
    t.replace_files(&tree.values().cloned().collect::<Vec<_>>()).unwrap();
    assert_eq!(all_records(&s), all_records(&t));
    assert_eq!(state(&s), state(&t));
}

/// Applies `commits` to `s` (derived) and after each compares its records
/// with a fresh derived build of the tree.
fn replay_records<B: Backend>(s: &Store<B>, fresh: impl Fn() -> (Store<B>, tempfile::TempDir), commits: Vec<Vec<FileIngest>>) {
    s.set_derived().unwrap();
    let mut tree = Tree::new();
    for (i, c) in commits.into_iter().enumerate() {
        s.replace_files(&c).unwrap();
        for f in c {
            if f.text.is_some() { tree.insert(f.path.clone(), f); } else { tree.remove(&f.path); }
        }
        let (t, _d) = fresh();
        t.set_derived().unwrap();
        t.replace_files(&tree.values().cloned().collect::<Vec<_>>()).unwrap();
        assert_eq!(all_records(s), all_records(&t), "after commit {i}");
    }
}

/// Facts do not keep records: a record no file has any more is collected
/// even while a fact is about it or uses it, as a fresh build would not
/// have it either (Codex's review of SR-146; the fact dangles in both).
fn facts_do_not_keep_records<B: Backend>(s: Store<B>, fresh: impl Fn() -> (Store<B>, tempfile::TempDir)) {
    let a_id = rec("1", "const_statement").id;
    let b_id = rec("2", "const_statement").id;
    for (subject, inputs) in [(b_id, vec![Input::Content(a_id)]), (a_id, vec![])] {
        let (s, _d) = fresh();
        let mut b = file("b.jl", &[("b", "2")]);
        b.facts.push(Fact::new("uses", subject, vec![], ("test", "0"), inputs, 0));
        replay_records(&s, &fresh, vec![vec![file("a.jl", &[("a", "1")]), b.clone()], vec![deleted("a.jl")], vec![file("b.jl", &[("b", "2")])]]);
        assert!(s.record(&a_id).unwrap().is_none());
    }
    drop(s);
}

/// One file listing a generic function twice is one contributor: it cannot
/// outvote another file's method when its name becomes a definition's.
fn a_file_listing_a_generic_twice_is_one_contributor<B: Backend>(s: Store<B>) {
    let mut a = file("a.jl", &[("g:1", "g(x) = 0")]);
    a.generics.push(a.generics[0].clone());
    s.replace_files(&[a, file("b.jl", &[("g:1", "g(x) = 0")])]).unwrap();
    let r = s.replace_files(&[file("a.jl", &[("g", "1")])]);
    assert!(matches!(r, Err(StoreError::NameClaimed { .. })), "b.jl still contributes: {r:?}");
    assert_eq!(methods(&s).len(), 1);
}

fn a_new_normaliser_needs_a_rebuild<B: Backend>(s: Store<B>) {
    s.replace_files(&[file("a.jl", &[("a", "1")])]).unwrap();
    let mut f = file("a.jl", &[("a", "1")]);
    f.normaliser = "moonkale-wp66@8".into();
    assert!(matches!(s.replace_files(&[f]), Err(StoreError::NormaliserChanged { .. })));
}

/// A generic-call fact about `subject` (as Moonkale's index would hand over).
fn call_fact(subject: Digest, callee: &str) -> Fact {
    Fact::new("calls-generic", subject, callee.as_bytes().to_vec(), ("moonkale-index", "0"), vec![], 0)
}

fn current_facts<B: Backend>(s: &Store<B>, subject: &Digest) -> Vec<Digest> {
    s.read(|tx| tx.facts_of(subject)).unwrap().into_iter().map(|f| f.id).collect()
}

/// Facts follow their file (SPEC-009 §2): replaced, removed on deletion,
/// counted across files, rolled back with the batch, and facts stored
/// outside the folder writer are left alone.
fn facts_follow_their_files<B: Backend>(s: Store<B>) {
    let x = rec("x()", "function_definition").id;
    let with_fact = |path: &str, callee: Option<&str>| {
        let mut f = file(path, &[("x", "x()")]);
        f.names.clear(); // the same record in two files: no name clash
        f.facts = callee.map(|c| vec![call_fact(x, c)]).unwrap_or_default();
        f
    };
    // Replacement with fewer facts removes the dropped one.
    let r = s.replace_files(&[with_fact("a.jl", Some("jl.P.f"))]).unwrap();
    assert_eq!(r.facts_added, 1);
    assert_eq!(current_facts(&s, &x), vec![call_fact(x, "jl.P.f").id]);
    assert_eq!(s.file_manifest("a.jl").unwrap().unwrap().facts, vec![call_fact(x, "jl.P.f").id]);
    let r = s.replace_files(&[with_fact("a.jl", None)]).unwrap();
    assert_eq!(r.facts_removed, 1);
    assert!(current_facts(&s, &x).is_empty(), "a replacement without the fact removes it");
    // Resolution changes, the body does not: the fact is replaced.
    s.replace_files(&[with_fact("a.jl", Some("jl.P.f"))]).unwrap();
    s.replace_files(&[with_fact("a.jl", Some("jl.Q.f"))]).unwrap();
    assert_eq!(current_facts(&s, &x), vec![call_fact(x, "jl.Q.f").id]);

    // One fact from two files stays until both drop it; deleting a file
    // removes its contribution though the record lives on in another file.
    s.replace_files(&[with_fact("b.jl", Some("jl.Q.f"))]).unwrap();
    s.replace_files(&[deleted("a.jl")]).unwrap();
    assert_eq!(current_facts(&s, &x), vec![call_fact(x, "jl.Q.f").id], "b.jl still contributes it");
    s.replace_files(&[with_fact("b.jl", Some("jl.R.f"))]).unwrap();
    assert_eq!(current_facts(&s, &x), vec![call_fact(x, "jl.R.f").id]);
    s.replace_files(&[with_fact("c.jl", None)]).unwrap();
    s.replace_files(&[deleted("b.jl")]).unwrap();
    assert!(current_facts(&s, &x).is_empty(), "deleting the last contributor removes the fact");
    assert!(s.record(&x).unwrap().is_some(), "the record stays (c.jl has it)");

    // A fact stored outside the folder writer survives files that contribute it too.
    let external = call_fact(x, "jl.S.f");
    s.record_fact(&external).unwrap();
    s.replace_files(&[with_fact("d.jl", Some("jl.S.f"))]).unwrap();
    s.replace_files(&[deleted("d.jl")]).unwrap();
    assert_eq!(current_facts(&s, &x), vec![external.id], "facts not from files stay");

    // A failed batch changes no fact and no manifest.
    s.replace_files(&[with_fact("e.jl", Some("jl.T.f")), file("owner.jl", &[("taken", "7")])]).unwrap();
    let before = (current_facts(&s, &x), s.file_manifest("e.jl").unwrap());
    let mut bad = with_fact("e.jl", None);
    bad.names.push(("jl.P.taken".into(), 0)); // owned by owner.jl, which is not in the batch
    assert!(matches!(s.replace_files(&[bad]), Err(StoreError::NameClaimed { .. })));
    assert_eq!((current_facts(&s, &x), s.file_manifest("e.jl").unwrap()), before, "rolled back");
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
            #[test]
            fn files_replace_names_generics_and_spans() {
                let (s, _d) = store();
                super::files_replace_names_generics_and_spans(s);
            }
            #[test]
            fn shared_methods_are_counted() {
                let (s, _d) = store();
                super::shared_methods_are_counted(s);
            }
            #[test]
            fn facts_follow_their_files() {
                let (s, _d) = store();
                super::facts_follow_their_files(s);
            }
            #[test]
            fn repeated_methods_survive_a_refresh() {
                let (s, _d) = store();
                super::repeated_methods_survive_a_refresh(s, store);
            }
            #[test]
            fn names_changing_kind_keep_memberships_right() {
                let (s, _d) = store();
                super::names_changing_kind_keep_memberships_right(s, store);
            }
            #[test]
            fn files_truth_collects_what_files_no_longer_reach() {
                let (s, _d) = store();
                super::files_truth_collects_what_files_no_longer_reach(s, store);
            }
            #[test]
            fn store_truth_keeps_history_until_swept() {
                let (s, _d) = store();
                super::store_truth_keeps_history_until_swept(s, store);
            }
            #[test]
            fn facts_do_not_keep_records() {
                let (s, _d) = store();
                super::facts_do_not_keep_records(s, store);
            }
            #[test]
            fn a_file_listing_a_generic_twice_is_one_contributor() {
                let (s, _d) = store();
                super::a_file_listing_a_generic_twice_is_one_contributor(s);
            }
            #[test]
            fn a_new_normaliser_needs_a_rebuild() {
                let (s, _d) = store();
                super::a_new_normaliser_needs_a_rebuild(s);
            }
        }
    };
}

suite!(sqlite, |p: &std::path::Path| sophia_store::sqlite::Sqlite::open(p.join("store.sqlite")).unwrap());
suite!(redb, |p: &std::path::Path| sophia_store::redb_store::Redb::open(p.join("store.redb")).unwrap());
#[cfg(feature = "lmdb")]
suite!(lmdb, |p: &std::path::Path| sophia_store::lmdb_store::Lmdb::open_with_map_size(p.join("store.lmdb"), 1 << 30).unwrap());
#[cfg(feature = "duckdb")]
suite!(duckdb, |p: &std::path::Path| sophia_store::duckdb_store::DuckDb::open(p.join("store.duckdb")).unwrap());
