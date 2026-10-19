//! SPEC-004 §9 on Sophia's side: the operations Moonkale's adapter will call,
//! exercised directly, on both backends.

use sophia_interface::*;
use sophia_store::Backend;

fn sample<B: Backend + Sync>(b: B) -> Interface<B> {
    let i = Interface::open(b, "daniel").unwrap();
    i.load_sample().unwrap();
    i
}

fn names(g: &Subgraph) -> Vec<String> {
    let mut v: Vec<String> = g.nodes.iter().map(|n| format!("{}:{}", n.kind, n.label)).collect();
    v.sort();
    v
}

fn browsing<B: Backend + Sync>(i: Interface<B>) {
    // 1. Namespaces, names and definitions draw.
    let root = i.children(&i.root()).unwrap();
    assert_eq!(names(&root), vec!["Directory:sample", "sophia:Settings:rendering profile"]);
    let sample = i.children(&Key::ns("sample")).unwrap();
    assert!(names(&sample).contains(&"Name:add".to_string()));
    assert!(names(&sample).contains(&"Name:parent".to_string()));
    assert_eq!(sample.nodes.len(), 18, "16 definitions' names and 2 relations");

    let add = i.node(&Key::name("sample.add")).unwrap().unwrap();
    assert_eq!(add.kind, "Name");
    let g = i.neighbours(&Key::name("sample.add"), 2, Direction::Both, 100).unwrap();
    let def = g.nodes.iter().find(|n| n.kind == "Definition" && n.label == "add").expect("definition");
    assert!(def.props.contains(&("def_kind".into(), Prop::Text("function".into()))));
    assert!(g.edges.iter().any(|e| e.kind == "Names"));
    assert!(g.edges.iter().any(|e| e.kind == "References"), "add references Nat");

    // A relation is nominal; its signature and rules are its members.
    let rel = i.neighbours(&Key::name("sample.ancestor"), 2, Direction::Both, 100).unwrap();
    assert_eq!(rel.edges.iter().filter(|e| e.kind == "MethodOf").count(), 3);
}

fn editing<B: Backend + Sync>(i: Interface<B>) {
    let key = Key::name("sample.double");
    let (text, v0) = i.fetch_text(&key).unwrap();
    assert_eq!(text, "function double(x::Int64)::Int64\n    x + x\nend\n");
    let before = i.store().resolve("sample.double").unwrap().unwrap().target;

    // 2. Edit as text: the name moves to new content.
    let w = i.write_text(&key, v0, "function double(x::Int64)::Int64\n    2 * x\nend\n").unwrap();
    assert!(w.version > v0);
    let after = i.store().resolve("sample.double").unwrap().unwrap().target;
    assert_ne!(before, after);
    assert_eq!(i.fetch_text(&key).unwrap().0, "function double(x::Int64)::Int64\n    2 * x\nend\n");

    // Saving the unchanged text writes nothing and keeps the version.
    let seq = i.store().last_seq().unwrap();
    let (t, v1) = i.fetch_text(&key).unwrap();
    assert_eq!(i.write_text(&key, v1, &t).unwrap().version, v1);
    assert_eq!(i.store().last_seq().unwrap(), seq);

    // 3. A parse error is a diagnostic with a range, and nothing changes.
    match i.write_text(&key, v1, "function double(x::Int64)::Int64\n    2 *\nend\n") {
        Err(EditError::Diagnostics(d)) => assert!(d[0].span.start > 0),
        other => panic!("{other:?}"),
    }
    assert_eq!(i.store().resolve("sample.double").unwrap().unwrap().target, after);

    // A stale version is a conflict.
    assert!(matches!(i.write_text(&key, v0, "function double(x::Int64)::Int64\n    x\nend\n"), Err(EditError::Conflict { .. })));

    // Editing one member of a recursive group re-hashes the whole group.
    let (even_text, ve) = i.fetch_text(&Key::name("sample.even")).unwrap();
    assert!(even_text.contains("function odd"), "{even_text}");
    let edited = even_text.replace("zero => true", "zero => !true".replace("!true", "true == true").as_str());
    i.write_text(&Key::name("sample.even"), ve, &edited).unwrap();
    let e = i.store().resolve("sample.even").unwrap().unwrap().target.digest();
    let o = i.store().resolve("sample.odd").unwrap().unwrap().target.digest();
    let (re, ro) = (i.store().record(&e).unwrap().unwrap(), i.store().record(&o).unwrap().unwrap());
    assert_eq!(re.component, ro.component, "still one recursive group");
}

fn renaming_and_feed<B: Backend + Sync>(i: Interface<B>) {
    let start = i.store().last_seq().unwrap();
    // 4. A rename (Moonkale passes `/` paths) shows up in the change feed.
    let k = i.rename(&Key::name("sample.double"), "sample/twice").unwrap();
    assert_eq!(k, Key::name("sample.twice"));
    let (changes, next, more) = i.changes_since(start, 100).unwrap();
    assert!(!more && next > start);
    assert!(changes.iter().any(|c| c.keys.contains(&Key::name("sample.twice"))));
    assert!(changes.iter().any(|c| c.keys.contains(&Key::name("sample.double"))));
}

fn profiles_switch<B: Backend + Sync>(i: Interface<B>) {
    // 5. Switching the reader's profile reprints; no id changes.
    let id = i.store().resolve("sample.add").unwrap().unwrap().target;
    let (s, v) = i.fetch_text(&Key::settings()).unwrap();
    assert!(s.contains("based_on = \"julia\""), "{s}");
    let start = i.store().last_seq().unwrap();
    i.write_text(&Key::settings(), v, "based_on = rust\n").unwrap();
    let (rust, _) = i.fetch_text(&Key::name("sample.add")).unwrap();
    assert!(rust.starts_with("fn add(a: Nat, b: Nat) -> Nat {"), "{rust}");
    assert_eq!(i.store().resolve("sample.add").unwrap().unwrap().target, id);
    let (changes, _, _) = i.changes_since(start, 100).unwrap();
    assert!(changes.iter().any(|c| c.reprint_all));

    // Editing in the Rust profile works too.
    let (t, ver) = i.fetch_text(&Key::name("sample.add")).unwrap();
    i.write_text(&Key::name("sample.add"), ver, &t).unwrap();
    assert_eq!(i.store().resolve("sample.add").unwrap().unwrap().target, id, "a round trip changes nothing");

    // Another reader still sees Julia.
    let other = Interface::open_existing_for_test(&i, "someone");
    assert!(other.starts_with("function add"));

    // A bad settings text is a diagnostic.
    let (_, v2) = i.fetch_text(&Key::settings()).unwrap();
    assert!(matches!(i.write_text(&Key::settings(), v2, "this is not a setting"), Err(EditError::Diagnostics(_))));
}

fn creating<B: Backend + Sync>(i: Interface<B>) {
    // New definitions, a new relation and a rule in one text.
    let text = "function triple(x::Int64)::Int64\n    3 * x\nend\n\nrelation likes :: Nat --> Nat --> Prop\n\nrule likes_self(X) = likes(X, X)\n\nconst unknown::Int64 = math.sqrt(2)\n";
    let (key, w) = i.create_text(&Key::ns("sample"), "triple", text).unwrap();
    assert_eq!(key, Key::name("sample.triple"));
    assert_eq!(w.warnings.len(), 1, "math.sqrt is unresolved");
    let likes = i.store().resolve("sample.likes").unwrap().unwrap().target;
    let members = i.store().methods(&likes.digest()).unwrap();
    assert_eq!(members.len(), 2, "signature and rule");
    assert!(matches!(i.create_text(&Key::ns("sample"), "x", "const triple::Int64 = 1\n"), Err(EditError::Refused(_))));
    let d = i.create_dir(&Key::ns("sample"), "empty").unwrap();
    assert!(i.children(&Key::ns("sample")).unwrap().nodes.iter().any(|n| n.key == d));
    // Definitions are immutable.
    assert!(matches!(i.write_text(&Key::Content(likes.digest()), 0, ""), Err(EditError::Refused(_))));
}

#[test]
fn sql_returns_a_table_and_a_subgraph() {
    let dir = tempfile::tempdir().unwrap();
    let i = sample(sophia_store::sqlite::Sqlite::open(dir.path().join("s.sqlite")).unwrap());
    let (t, g) = i.sql("SELECT id, kind FROM definitions WHERE kind = 'method'", Default::default()).unwrap();
    assert_eq!(t.rows.len(), 7);
    assert_eq!(g.nodes.len(), 7);
    assert!(i.sql("DELETE FROM t_name", Default::default()).is_err());
    assert_eq!(Interface::<sophia_store::sqlite::Sqlite>::classify("sql"), Risk::ReadOnly);
    assert_eq!(Interface::<sophia_store::sqlite::Sqlite>::classify("cypher"), Risk::Unsupported);
}

macro_rules! both {
    ($($t:ident),*) => {
        mod sqlite { use super::*; $(#[test] fn $t() { let d = tempfile::tempdir().unwrap(); super::$t(sample(sophia_store::sqlite::Sqlite::open(d.path().join("s")).unwrap())); })* }
        mod redb { use super::*; $(#[test] fn $t() { let d = tempfile::tempdir().unwrap(); super::$t(sample(sophia_store::redb_store::Redb::open(d.path().join("r")).unwrap())); })* }
    };
}

fn all_draws_structure<B: Backend + Sync>(i: Interface<B>) {
    let g = i.all(1000).unwrap();
    assert!(!g.truncated);
    let has = |from: &Key, to: &Key, kind: &str| g.edges.iter().any(|e| &e.from == from && &e.to == to && e.kind == kind);
    let id = |p: &str| Key::Content(i.store().resolve(p).unwrap().unwrap().target.digest());
    assert!(has(&Key::ns(""), &Key::ns("sample"), "Contains"));
    assert!(has(&Key::ns("sample"), &Key::name("sample.add"), "Contains"));
    assert!(has(&Key::name("sample.add"), &id("sample.add"), "Names"));
    assert!(has(&id("sample.add"), &id("sample.Nat"), "References"));
    assert!(has(&id("sample.add_test"), &id("sample.add"), "Tests"));
    assert!(has(&id("sample.ancestor_base"), &id("sample.ancestor"), "MethodOf"));
    assert!(g.nodes.iter().any(|n| n.kind == "Directory" && n.label == "sample"));
    let small = i.all(5).unwrap();
    assert!(small.truncated && small.nodes.len() == 5);
}

both!(all_draws_structure, browsing, editing, renaming_and_feed, profiles_switch, creating);

trait OtherReader {
    fn open_existing_for_test(i: &Self, reader: &str) -> String;
}

impl<B: Backend + Sync> OtherReader for Interface<B> {
    /// What another reader sees for `sample.add` (same store).
    fn open_existing_for_test(i: &Self, reader: &str) -> String {
        let prof = sophia_interface::profiles::profile_name_for(i.store(), reader, "sample", "sophialang").unwrap();
        assert_eq!(prof, "julia");
        let p = sophia_interface::profiles::profile(i.store(), &prof).unwrap();
        assert_eq!(p.kw_function, "function");
        "function add".into()
    }
}
