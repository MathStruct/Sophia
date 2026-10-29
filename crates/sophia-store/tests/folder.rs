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

/// A record no name, span or generic membership reaches, directly or through
/// references, is refused: the manifest would not reach it, so collection
/// could never remove it and the store would drift from a fresh build (RQ-002).
fn unreached_records_are_refused<B: Backend>(s: Store<B>) {
    let mut a = file("a.jl", &[("a", "1")]);
    a.records.push(rec("hidden", "const_statement"));
    assert!(matches!(s.replace_files(&[a]), Err(StoreError::UnreachedRecord { .. })));
    assert!(s.file_manifest("a.jl").unwrap().is_none(), "nothing written");
    // Reached through a reference from a named record: accepted.
    let mut b = file("b.jl", &[("b", "2")]);
    let helper = rec("helper", "const_statement");
    b.records[0].refs.push(helper.id);
    b.records.push(helper);
    s.replace_files(&[b]).unwrap();
}

fn last_seq<B: Backend>(s: &Store<B>) -> u64 {
    s.read(|tx| tx.last_seq()).unwrap()
}

/// SPEC-010 §2: in a store-truth folder a file is written with the token
/// read when it was last printed; a stale token writes nothing.
fn checked_replacement_refuses_stale_tokens<B: Backend>(s: Store<B>) {
    assert_eq!(s.file_token("a.jl").unwrap(), 0, "no such file");
    s.replace_files_checked(&[(file("a.jl", &[("a", "1"), ("g:1", "g(x) = 0")]), 0)]).unwrap();
    let t1 = s.file_token("a.jl").unwrap();
    assert_ne!(t1, 0);
    assert_eq!(s.laid_out("jl.P.a").unwrap(), Some(("a.jl".to_string(), t1)));
    assert_eq!(s.laid_out("jl.P.nothing").unwrap(), None);
    // Another writer replaces the file: the first writer's token is stale.
    s.replace_files_checked(&[(file("a.jl", &[("a", "2"), ("g:1", "g(x) = 0")]), t1)]).unwrap();
    let t2 = s.file_token("a.jl").unwrap();
    assert_ne!(t1, t2);
    let before = (all_records(&s), state(&s), last_seq(&s));
    let r = s.replace_files_checked(&[(file("a.jl", &[("a", "3")]), t1), (file("b.jl", &[("b", "4")]), 0)]);
    match r {
        Err(StoreError::FileConflict { conflicts }) => assert_eq!(conflicts, vec![("a.jl".to_string(), t1, t2)]),
        other => panic!("expected a conflict: {other:?}"),
    }
    assert_eq!((all_records(&s), state(&s), last_seq(&s)), before, "nothing written");
    // A name of the file moving changes its token, as an edit does.
    s.replace_files_checked(&[(file("a.jl", &[("a", "2")]), t2), (file("b.jl", &[("b", "4")]), 0)]).unwrap();
    let t3 = s.file_token("a.jl").unwrap();
    s.replace_files_checked(&[(file("a.jl", &[]), t3), (file("b.jl", &[("b", "4"), ("a", "2")]), s.file_token("b.jl").unwrap())]).unwrap();
    assert_eq!(s.laid_out("jl.P.a").unwrap().map(|(p, _)| p), Some("b.jl".to_string()));
    // A derived store's files are the truth: nothing to check against.
    s.set_derived().unwrap();
    assert!(matches!(s.replace_files_checked(&[(file("c.jl", &[("c", "5")]), 0)]), Err(StoreError::Derived)));
    assert!(matches!(s.propose_files(&[(file("c.jl", &[("c", "5")]), 0)]), Err(StoreError::Derived)));
}

/// SPEC-010 §5: a proposal writes nothing, says what would change, and
/// accepting it equals a plain replacement.
fn a_proposal_writes_nothing_and_accepting_it_equals_replacing<B: Backend>(s: Store<B>, fresh: impl Fn() -> (Store<B>, tempfile::TempDir)) {
    let (t, _d) = fresh();
    let first = vec![file("a.jl", &[("a", "1"), ("b", "2"), ("g:1", "g(x) = 0")]), file("b.jl", &[("c", "3")])];
    s.replace_files(&first).unwrap();
    t.replace_files(&first).unwrap();
    let edit = file("a.jl", &[("a", "10"), ("d", "4"), ("g:2", "g(x) = 1")]);
    let token = s.file_token("a.jl").unwrap();
    let before = (all_records(&s), state(&s), last_seq(&s), s.file_manifest("a.jl").unwrap());
    let p = s.propose_files(&[(edit.clone(), token)]).unwrap();
    assert_eq!((all_records(&s), state(&s), last_seq(&s), s.file_manifest("a.jl").unwrap()), before, "nothing written");
    assert!(p.conflicts.is_empty() && p.error.is_none());
    let f = &p.files[0];
    let id = |b: &str| rec(b, "const_statement").id;
    let m = |b: &str| rec(b, "function_definition").id;
    assert!(f.text_changed);
    assert_eq!(f.names_added, vec![("jl.P.d".to_string(), id("4"))]);
    assert_eq!(f.names_removed, vec![("jl.P.b".to_string(), id("2"))]);
    assert_eq!(f.names_retargeted, vec![("jl.P.a".to_string(), id("1"), id("10"))]);
    assert_eq!(f.members_added, vec![("jl.P.g".to_string(), m("g(x) = 1"))]);
    assert_eq!(f.members_removed, vec![("jl.P.g".to_string(), m("g(x) = 0"))]);
    let report = p.report.clone().expect("would succeed");
    // Accept: the same as replacing.
    assert_eq!(s.replace_files_checked(&[(edit.clone(), token)]).unwrap(), report);
    t.replace_files(&[edit]).unwrap();
    assert_eq!((all_records(&s), state(&s)), (all_records(&t), state(&t)));
    // Conflicts and errors are reported, not thrown.
    let p = s.propose_files(&[(file("a.jl", &[("a", "11")]), token)]).unwrap();
    assert_eq!(p.conflicts.len(), 1);
    let p = s.propose_files(&[(file("c.jl", &[("c", "3")]), 0)]).unwrap();
    assert!(matches!(p.error, Some(StoreError::NameClaimed { .. })), "{:?}", p.error);
    let mut hidden = file("c.jl", &[("e", "5")]);
    hidden.records.push(rec("hidden", "const_statement"));
    assert!(matches!(s.propose_files(&[(hidden, 0)]).unwrap().error, Some(StoreError::UnreachedRecord { .. })));
}

/// SPEC-010 §3: the truth moves into the store in place, the store keeps
/// every later version, and giving the truth back leaves a fresh build.
fn the_truth_moves_into_the_store_and_back<B: Backend>(s: Store<B>, fresh: impl Fn() -> (Store<B>, tempfile::TempDir)) {
    s.set_derived().unwrap();
    assert!(matches!(s.give_truth(&[]), Err(StoreError::Derived)));
    let mut tree = Tree::new();
    let commits = history();
    let apply = |tree: &mut Tree, c: &[FileIngest]| {
        for f in c {
            if f.text.is_some() { tree.insert(f.path.clone(), f.clone()); } else { tree.remove(&f.path); }
        }
    };
    s.replace_files(&commits[0]).unwrap();
    apply(&mut tree, &commits[0]);
    // Into the store, with the files as they are now.
    s.take_truth(&commits[1]).unwrap();
    apply(&mut tree, &commits[1]);
    assert!(!s.is_derived().unwrap());
    assert!(s.meta(sophia_store::folder::TRUTH_SINCE_KEY).unwrap().is_some());
    assert!(matches!(s.take_truth(&[]), Err(StoreError::NotDerived)));
    // Store-truth edits keep every version.
    let mut seen = std::collections::BTreeSet::new();
    for c in &commits[2..4] {
        let checked: Vec<(FileIngest, u64)> = c.iter().map(|f| (f.clone(), s.file_token(&f.path).unwrap())).collect();
        assert_eq!(s.replace_files_checked(&checked).unwrap().records_removed, 0);
        seen.extend(c.iter().flat_map(|f| f.records.iter().map(|r| r.id)));
        apply(&mut tree, c);
    }
    let kept: std::collections::BTreeSet<Digest> = all_records(&s).into_iter().collect();
    assert!(seen.is_subset(&kept), "every version stays");
    // Back to the files: every current file, with the text written.
    let exported: Vec<(String, Digest)> = tree.values().map(|f| (f.path.clone(), source_record(&f.path, f.text.as_ref().unwrap()).id)).collect();
    assert!(matches!(s.give_truth(&exported[1..]), Err(StoreError::ExportMismatch { .. })), "a file not exported");
    let mut wrong = exported.clone();
    wrong[0].1 = source_record(&wrong[0].0, "edited elsewhere").id;
    assert!(matches!(s.give_truth(&wrong), Err(StoreError::ExportMismatch { .. })), "a text not as stored");
    assert!(s.give_truth(&exported).unwrap() > 0);
    assert!(s.is_derived().unwrap());
    let (t, _d) = fresh();
    t.set_derived().unwrap();
    t.replace_files(&tree.values().cloned().collect::<Vec<_>>()).unwrap();
    assert_eq!((all_records(&s), state(&s)), (all_records(&t), state(&t)));
}

/// Codex review of SPEC-010 (2026-10-28): a generic function's name is laid
/// out by the files that contribute its methods, though it has no owner row.
fn generic_names_are_laid_out<B: Backend>(s: Store<B>) {
    s.replace_files_checked(&[(file("b.jl", &[("g:1", "g(x) = 0")]), 0), (file("a.jl", &[("g:2", "g(x) = 1")]), 0)]).unwrap();
    let token = s.file_token("a.jl").unwrap();
    assert_eq!(s.laid_out("jl.P.g").unwrap(), Some(("a.jl".to_string(), token)), "the first contributing file");
}

/// Codex review of SPEC-010: the files get the truth back only when they
/// hold every name; a name no file has would outlive its export.
fn give_truth_refuses_names_no_file_has<B: Backend>(s: Store<B>) {
    s.set_derived().unwrap();
    let a = file("a.jl", &[("a", "1"), ("g:1", "g(x) = 0")]);
    s.replace_files(std::slice::from_ref(&a)).unwrap();
    s.take_truth(&[]).unwrap();
    s.alias("jl.P.extra", Target::Content(rec("1", "const_statement").id), Visibility::Public).unwrap();
    let exported = vec![("a.jl".to_string(), source_record("a.jl", a.text.as_ref().unwrap()).id)];
    match s.give_truth(&exported) {
        Err(StoreError::NotInFiles { name }) => assert_eq!(name, "jl.P.extra"),
        other => panic!("expected NotInFiles: {other:?}"),
    }
    assert!(!s.is_derived().unwrap(), "nothing changed");
    s.remove("jl.P.extra", None).unwrap();
    // A file's name moved by a store call: the files say otherwise (second
    // Codex review).
    let seq = s.resolve("jl.P.a").unwrap().unwrap().seq;
    s.update("jl.P.a", &[rec("9", "const_statement")], 0, seq).unwrap();
    assert!(matches!(s.give_truth(&exported), Err(StoreError::ExportMismatch { path }) if path == "a.jl"));
    let seq = s.resolve("jl.P.a").unwrap().unwrap().seq;
    s.update("jl.P.a", &[rec("1", "const_statement")], 0, seq).unwrap();
    assert!(s.give_truth(&exported).is_ok());
}

/// Codex review of SPEC-010: a proposal names the generic functions an
/// accept would create or remove, which no single file's diff shows.
fn proposals_name_generic_functions_created_and_removed<B: Backend>(s: Store<B>) {
    s.replace_files(&[file("b.jl", &[("b", "1")])]).unwrap();
    let a = file("a.jl", &[("g:1", "g(x) = 0")]);
    let p = s.propose_files(&[(a.clone(), 0)]).unwrap();
    assert_eq!((p.generics_added.clone(), p.generics_removed.clone()), (vec!["jl.P.g".to_string()], vec![]));
    s.replace_files_checked(&[(a, 0)]).unwrap();
    let p = s.propose_files(&[(deleted("a.jl"), s.file_token("a.jl").unwrap())]).unwrap();
    assert_eq!((p.generics_added, p.generics_removed), (vec![], vec!["jl.P.g".to_string()]));
    // The second Codex review's case, `g` a constant of a.jl while b.jl
    // still has a method of it, cannot arise: one batch cannot hold both,
    // and a later batch with only a.jl is claimed by b.jl's method.
    let t = |p: &str| s.file_token(p).unwrap();
    let b = (file("b.jl", &[("b", "1"), ("g:1", "g(x) = 0")]), t("b.jl"));
    s.replace_files_checked(&[(file("a.jl", &[("g:1", "g(x) = 0")]), t("a.jl")), b.clone()]).unwrap();
    let b = (b.0, t("b.jl"));
    let claimed = |r: std::result::Result<FileReport, StoreError>| assert!(matches!(r, Err(StoreError::NameClaimed { .. })), "{r:?}");
    claimed(s.replace_files_checked(&[(file("a.jl", &[("g", "5")]), t("a.jl")), b]));
    claimed(s.replace_files_checked(&[(file("a.jl", &[("g", "5")]), t("a.jl"))]));
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
            fn unreached_records_are_refused() {
                let (s, _d) = store();
                super::unreached_records_are_refused(s);
            }
            #[test]
            fn checked_replacement_refuses_stale_tokens() {
                let (s, _d) = store();
                super::checked_replacement_refuses_stale_tokens(s);
            }
            #[test]
            fn a_proposal_writes_nothing_and_accepting_it_equals_replacing() {
                let (s, _d) = store();
                super::a_proposal_writes_nothing_and_accepting_it_equals_replacing(s, store);
            }
            #[test]
            fn the_truth_moves_into_the_store_and_back() {
                let (s, _d) = store();
                super::the_truth_moves_into_the_store_and_back(s, store);
            }
            #[test]
            fn generic_names_are_laid_out() {
                let (s, _d) = store();
                super::generic_names_are_laid_out(s);
            }
            #[test]
            fn give_truth_refuses_names_no_file_has() {
                let (s, _d) = store();
                super::give_truth_refuses_names_no_file_has(s);
            }
            #[test]
            fn proposals_name_generic_functions_created_and_removed() {
                let (s, _d) = store();
                super::proposals_name_generic_functions_created_and_removed(s);
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
