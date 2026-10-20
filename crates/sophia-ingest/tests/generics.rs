//! Generic functions are (module, name), never a name alone: same-named
//! functions in different modules stay apart, and methods written
//! `Base.show(…)`, `A.f(…)` or after `import M: f` join the function they
//! extend. Fixture: `julia-semantics/generics` ([[Julia Name Resolution]]).

use std::path::Path;

use sophia_store::Store;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/julia-semantics/generics");

#[test]
fn generic_functions_are_module_qualified() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::new(sophia_store::sqlite::Sqlite::open(dir.path().join("g.sqlite")).unwrap());
    let (_, root) = sophia_ingest::module::package_root(Path::new(DIR)).unwrap();
    let (r, _, _) = sophia_ingest::store_module(&s, "Gen", &root).unwrap();
    assert!(r.refused_definitions.is_empty(), "{:?}", r.refused_definitions);
    let methods = |p: &str| {
        let g = s.resolve(p).unwrap().unwrap_or_else(|| panic!("no {p}")).target.digest();
        s.methods(&g).unwrap().len()
    };
    // A.f: its two methods, `A.f(p::Point)` from Gen, `f(::String)` from C
    // after `import ..A: f`.
    assert_eq!(methods("jl.Gen.A.f"), 4);
    // B.f is another function, and so is Gen's own f.
    assert_eq!(methods("jl.Gen.B.f"), 1);
    assert_eq!(methods("jl.Gen.f"), 2);
    // Constructors: the type's name is the struct; its methods are `.new`.
    assert_eq!(methods("jl.Gen.Point.new"), 2);
    assert!(s.resolve("jl.Gen.Point").unwrap().is_some());
    // Methods of Base's functions: by `import Base: show`, `import
    // Base.length`, and qualified heads.
    for p in ["jl.Base.show", "jl.Base.length", "jl.Base.`+`", "jl.Base.getindex"] {
        assert_eq!(methods(p), 1, "{p}");
    }
    assert_eq!(r.extended, 4);
    assert!(s.resolve("jl.Gen.show").unwrap().is_none(), "show is Base's, not Gen's");
    // `use` calls Gen's f, not A's or B's.
    let gen_f = s.resolve("jl.Gen.f").unwrap().unwrap().target.digest();
    let a_f = s.resolve("jl.Gen.A.f").unwrap().unwrap().target.digest();
    let users: Vec<_> = s
        .read(|tx| {
            let mut v = Vec::new();
            for id in tx.records_of_kind(None, usize::MAX)? {
                v.extend(tx.record(&id)?);
            }
            Ok(v)
        })
        .unwrap()
        .into_iter()
        .filter(|rec| !rec.nominal_refs.is_empty())
        .collect();
    assert_eq!(users.len(), 1, "only `use` calls a generic function");
    assert_eq!(users[0].nominal_refs, vec![gen_f]);
    assert!(!users[0].nominal_refs.contains(&a_f));
}
