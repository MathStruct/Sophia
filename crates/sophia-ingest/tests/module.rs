//! Module-level Julia resolution (SPEC-008), on a multi-file fixture.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use sophia_ingest::module::{Module, discover, ingest, resolve};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/module-fixture");

/// (definition, referenced name, referenced file) for every resolved span.
type Edges = BTreeSet<(String, String, String)>;

fn edges(root: &str) -> (Edges, HashMap<String, BTreeSet<String>>) {
    let d = discover(&Path::new(DIR).join(root)).unwrap();
    let m = Module::new(&d).unwrap();
    let defs = m.definitions();
    let r = resolve(&m, &defs, &d.dynamic);
    let base = |f: usize| Path::new(&m.files[f].path).file_name().unwrap().to_string_lossy().to_string();
    let mut out = BTreeSet::new();
    let mut deps = HashMap::new();
    for (f, fr) in r.refs.iter().enumerate() {
        for (i, map) in fr.iter().enumerate() {
            let from = defs[f][i].0.name.clone();
            for t in map.values() {
                out.insert((from.clone(), defs[t.0][t.1].0.name.clone(), base(t.0)));
            }
            deps.insert(from, r.depends_on[&(f, i)].iter().map(|&x| base(x)).collect());
        }
    }
    (out, deps)
}

fn has(e: &BTreeSet<(String, String, String)>, from: &str, to: &str) -> bool {
    e.iter().any(|(f, t, _)| f == from && t == to)
}

#[test]
fn includes_are_followed_in_order() {
    let d = discover(&Path::new(DIR).join("Geo.jl")).unwrap();
    let names: Vec<String> = d.files.iter().map(|f| Path::new(&f.path).file_name().unwrap().to_string_lossy().to_string()).collect();
    assert_eq!(names, ["Geo.jl", "types.jl", "ops.jl"]);
    assert_eq!(d.files[1].base_module, ["Geo"]);
    assert!(d.dynamic.is_empty());
}

#[test]
fn references_resolve_across_files_conservatively() {
    let (e, deps) = edges("Geo.jl");
    // Across files: ops.jl uses the struct from types.jl.
    assert!(e.contains(&("dist".into(), "Point".into(), "types.jl".into())));
    assert!(has(&e, "dist", "norm2"));
    assert!(has(&e, "origin", "Point"));
    // Imported (`using LinearAlgebra: norm`): never resolved to a local.
    assert!(!has(&e, "len", "norm"));
    assert!(has(&e, "len", "Point"));
    // `helper` lives in the submodule Inner: invisible from Geo.
    assert!(!has(&e, "area", "helper"));
    // ...but visible inside Inner.
    assert!(has(&e, "scale", "helper"));
    // Two methods: the call is to the generic function, unresolved.
    assert!(!has(&e, "use_twice", "twice"));
    // `x.y` fields and `Base.show` are not module-level names.
    assert!(!e.iter().any(|(f, _, _)| f == "show2"));
    // The files an id depends on (Moonkale's cache key).
    assert_eq!(deps["dist"], BTreeSet::from(["ops.jl".to_string(), "types.jl".to_string()]));
    assert_eq!(deps["twice"], BTreeSet::from(["ops.jl".to_string()]));
}

#[test]
fn a_dynamic_include_keeps_functions_open() {
    let d = discover(&Path::new(DIR).join("Dyn.jl")).unwrap();
    assert_eq!(d.dynamic.len(), 1, "include(joinpath(…)) is dynamic");
    let (e, _) = edges("Dyn.jl");
    assert!(!has(&e, "f", "origin"), "a function could gain methods from the dynamic include");
    assert!(has(&e, "g", "Point"), "a type cannot be redefined: still resolved");
}

#[test]
fn module_ingest_identifies_every_file() {
    let d = discover(&Path::new(DIR).join("Geo.jl")).unwrap();
    let r = ingest(&d).unwrap();
    assert!(r.cross_file_references >= 4, "{}", r.cross_file_references);
    let all: Vec<_> = r.files.iter().flat_map(|(_, v)| v).collect();
    let dist = all.iter().find(|(d, _)| d.name == "dist").unwrap();
    let point = all.iter().find(|(d, _)| d.name == "Point").unwrap();
    assert!(dist.1.refs.contains(&point.1.id), "dist's id depends on Point's (Merkle, across files)");
}
