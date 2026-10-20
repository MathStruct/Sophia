//! Julia modules through the crate's module-level resolver (normaliser
//! v5), checked against Moonkale's golden data (`golden-moonkale/module.tsv`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sophia_ingest::module::{discover, ingest, package_root};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden-moonkale/module");

fn base(p: &str) -> String {
    Path::new(p).file_name().unwrap().to_string_lossy().to_string()
}

#[test]
fn includes_are_followed_in_order() {
    let d = discover(&Path::new(DIR).join("Geo.jl")).unwrap();
    let names: Vec<String> = d.files.iter().map(|f| base(&f.path)).collect();
    assert_eq!(names, ["Geo.jl", "types.jl", "ops.jl"]);
    assert_eq!(d.files[1].base_module, ["Geo"]);
    let dy = discover(&Path::new(DIR).join("Dyn.jl")).unwrap();
    assert_eq!(dy.dynamic.len(), 1, "include(joinpath(…)) is dynamic");
}

/// Every row of module.tsv: Sophia, reading the files itself and calling
/// the crate, gets Moonkale's ids, depends_on, references and generic calls,
/// and its stored records carry those ids.
#[test]
fn sophia_reproduces_moonkale_module_ids() {
    let tsv = std::fs::read_to_string(format!("{DIR}/../module.tsv")).unwrap();
    let mut rows: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    for l in tsv.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let cols: Vec<String> = l.split('\t').map(String::from).collect();
        rows.entry(cols[0].clone()).or_default().push(cols);
    }
    assert!(rows.len() >= 4);
    for (root, rows) in rows {
        let d = discover(&Path::new(DIR).join(&root)).unwrap();
        let r = ingest(&d).unwrap();
        let fname = |i: u32| base(&d.files[i as usize].path);
        let stored: BTreeMap<(String, String), String> = r
            .files
            .iter()
            .flat_map(|(p, v)| v.iter().map(move |(def, rec)| ((base(p), format!("{}:{}", def.start, def.end)), rec.id.to_hex())))
            .collect();
        for row in &rows {
            if row[2] == "calls_generic" {
                let span = row[3].clone();
                assert!(
                    r.calls_generic.iter().any(|(f, s, n)| fname(*f) == row[1] && format!("{}:{}", s.0, s.1) == span && *n == row[4]),
                    "{root}: calls_generic {row:?} missing from {:?}",
                    r.calls_generic
                );
                continue;
            }
            let x = r
                .module
                .definitions
                .iter()
                .find(|x| fname(x.key.0) == row[1] && format!("{}:{}", x.key.1, x.key.2) == row[3])
                .unwrap_or_else(|| panic!("{root}: no definition {row:?}"));
            let got = match &x.identity {
                Ok(i) => i.digest.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                Err(e) => format!("{e:?}"),
            };
            assert_eq!(got, row[4], "{root}: {} {}", row[1], row[2]);
            let deps: BTreeSet<String> = x.depends_on.iter().map(|&f| fname(f)).collect();
            let want: BTreeSet<String> = row[5].split(',').filter(|s| !s.is_empty()).map(String::from).collect();
            assert_eq!(deps, want, "{root}: depends_on of {}", row[2]);
            let refs: BTreeSet<String> =
                x.references.iter().map(|(s, t)| format!("{}:{}>{}:{}:{}", s.0, s.1, fname(t.0), t.1, t.2)).collect();
            let want: BTreeSet<String> = row.get(6).map(|c| c.split(',').filter(|s| !s.is_empty()).map(String::from).collect()).unwrap_or_default();
            assert_eq!(refs, want, "{root}: references of {}", row[2]);
            if x.identity.is_ok() {
                assert_eq!(stored[&(row[1].clone(), row[3].clone())], row[4], "{root}: stored record id of {}", row[2]);
            }
        }
    }
}

#[test]
fn package_root_reads_project_toml() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("Project.toml"), "name = \"Geo\"\nuuid = \"x\"\n").unwrap();
    std::fs::write(dir.path().join("src/Geo.jl"), "module Geo end\n").unwrap();
    let (name, root) = package_root(dir.path()).unwrap();
    assert_eq!(name, "Geo");
    assert!(root.ends_with("src/Geo.jl"));
}
