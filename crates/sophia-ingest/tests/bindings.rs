//! SPEC-009 §8, requirement 4: the names Sophia gives Julia definitions are
//! the ones Moonkale's golden `bindings.tsv` records. Sophia finds the
//! definitions itself (`code::file`, `module::ingest`) and names them with
//! the shared `binding` / `entity_path` of `moonkale-content-id`; a drift
//! on either side fails here.

use std::collections::BTreeMap;
use std::path::Path;

use sophia_ingest::{binding, entity_path};

const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden-moonkale");

/// (file, start, end) → (name, binding, entity_path), as written in the TSV.
fn rows() -> BTreeMap<(String, usize, usize), (String, String, String)> {
    let tsv = std::fs::read_to_string(format!("{GOLDEN}/bindings.tsv")).unwrap();
    tsv.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            ((c[0].to_string(), c[1].parse().unwrap(), c[2].parse().unwrap()), (c[3].into(), c[4].into(), c[5].into()))
        })
        .collect()
}

/// Sophia's row for one definition, in the TSV's spelling.
fn ours(d: &sophia_ingest::code::Def, pkg: &str) -> (String, String, String) {
    let b = binding(d, pkg);
    let path = b.as_ref().map(|b| match b {
        sophia_ingest::Binding::Method(k) | sophia_ingest::Binding::Named(k) => entity_path("jl", k),
    });
    (format!("{:?}", d.name), format!("{b:?}"), format!("{path:?}"))
}

fn check(file: &str, defs: &[sophia_ingest::code::Def], pkg: &str, golden: &BTreeMap<(String, usize, usize), (String, String, String)>) -> usize {
    for d in defs {
        let key = (file.to_string(), d.start, d.end);
        let want = golden.get(&key).unwrap_or_else(|| panic!("{file} {}..{} ({}) is not in bindings.tsv", d.start, d.end, d.name));
        assert_eq!(&ours(d, pkg), want, "{file} {}..{}", d.start, d.end);
    }
    defs.len()
}

#[test]
fn julia_names_match_moonkales_golden_bindings() {
    let golden = rows();
    let mut checked = 0;
    let mut seen = std::collections::HashSet::new();
    // Single files, named under the package `Fixture`, as Moonkale's golden run does.
    for f in ["geometry.jl", "scopes.jl"] {
        let src = std::fs::read_to_string(format!("{GOLDEN}/{f}")).unwrap();
        let file = sophia_ingest::code::file("julia", &src).unwrap();
        let defs: Vec<_> = file.defs.iter().cloned().chain(file.refused.iter().map(|(d, _)| d.clone())).collect();
        checked += check(f, &defs, "Fixture", &golden);
    }
    // Packages: their files read through `module::discover`, named under the root module.
    for (dir, root, pkg) in [("module", "Geo.jl", "Geo"), ("module", "Callable.jl", "Callable"), ("module", "Macros.jl", "Macros"), ("module", "Dyn.jl", "Dyn"), ("generics", "Gen.jl", "Gen")] {
        let d = sophia_ingest::module::discover(&Path::new(GOLDEN).join(dir).join(root)).unwrap();
        let r = sophia_ingest::module::ingest(&d).unwrap();
        for (path, items) in &r.files {
            let name = format!("{dir}/{}", Path::new(path).file_name().unwrap().to_string_lossy());
            // A file included by several roots (`Dyn.jl` includes `types.jl`
            // too) is recorded under the first, as in Moonkale's run.
            if !seen.insert(name.clone()) {
                continue;
            }
            let defs: Vec<_> = items.iter().map(|(d, _)| d.clone()).collect();
            checked += check(&name, &defs, pkg, &golden);
        }
    }
    let julia_rows = golden.keys().filter(|(f, _, _)| f.ends_with(".jl")).count();
    assert_eq!(checked, julia_rows, "every Julia row of bindings.tsv is checked");
}
