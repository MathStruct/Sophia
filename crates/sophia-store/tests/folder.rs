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

fn a_new_normaliser_needs_a_rebuild<B: Backend>(s: Store<B>) {
    s.replace_files(&[file("a.jl", &[("a", "1")])]).unwrap();
    let mut f = file("a.jl", &[("a", "1")]);
    f.normaliser = "moonkale-wp66@8".into();
    assert!(matches!(s.replace_files(&[f]), Err(StoreError::NormaliserChanged { .. })));
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
