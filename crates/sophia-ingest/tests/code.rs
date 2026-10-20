//! Julia, Rust and Python through Moonkale's WP-66 normal form (SR-116),
//! checked against Moonkale's golden fixtures (`tests/golden-moonkale`).

use std::collections::{BTreeSet, HashMap};

use moonkale_content_id::{Budget, parse};
use sophia_ingest::code::{definitions, file, identify, resolve};

const FILES: [&str; 4] = ["geometry.jl", "geometry.rs", "geometry.py", "scopes.jl"];

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden-moonkale");

fn tsv(name: &str) -> Vec<Vec<String>> {
    std::fs::read_to_string(format!("{DIR}/{name}"))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split('\t').map(String::from).collect())
        .collect()
}

fn lang(file: &str) -> &'static str {
    match file.rsplit('.').next() {
        Some("jl") => "julia",
        Some("rs") => "rust",
        _ => "python",
    }
}

/// With exactly Moonkale's resolved spans, Sophia's use of the shared
/// normaliser gives Moonkale's ids: the normal form is shared.
#[test]
fn moonkale_spans_give_moonkale_ids() {
    let ids = tsv("ids.tsv");
    let refs = tsv("references.tsv");
    for f in FILES {
        let l = lang(f);
        let src = std::fs::read_to_string(format!("{DIR}/{f}")).unwrap();
        let mut budget = Budget::new();
        let tree = parse(l, &src, &mut budget).unwrap();
        let defs = definitions(l, tree.root_node(), &src);
        let index: HashMap<&str, usize> = defs.iter().enumerate().map(|(i, (d, _))| (d.name.as_str(), i)).collect();
        let mut spans: Vec<HashMap<(usize, usize), usize>> = vec![HashMap::new(); defs.len()];
        for r in refs.iter().filter(|r| r[0] == f) {
            let (s, e) = r[2].split_once(':').unwrap();
            spans[index[r[1].as_str()]].insert((s.parse().unwrap(), e.parse().unwrap()), index[r[3].as_str()]);
        }
        let (_, got) = identify(l, &src, &defs, &spans, &mut budget).unwrap();
        for row in ids.iter().filter(|r| r[0] == f) {
            let i = index[row[1].as_str()];
            let hex: String = got[&i].digest.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(hex, row[2], "{f} {}", row[1]);
            assert_eq!(got[&i].component_index.to_string(), row[3], "{f} {} index", row[1]);
            assert_eq!(got[&i].component_size.to_string(), row[4], "{f} {} size", row[1]);
        }
    }
}

/// Sophia's own resolver against Moonkale's spans.
#[test]
fn sophia_resolves_like_moonkale() {
    let refs = tsv("references.tsv");
    for f in FILES {
        let l = lang(f);
        let src = std::fs::read_to_string(format!("{DIR}/{f}")).unwrap();
        let tree = parse(l, &src, &mut Budget::new()).unwrap();
        let defs = definitions(l, tree.root_node(), &src);
        let mine: BTreeSet<(String, String, String)> = resolve(l, &defs, &src)
            .iter()
            .enumerate()
            .flat_map(|(i, m)| {
                let defs = &defs;
                m.iter().map(move |((s, e), t)| (defs[i].0.name.clone(), format!("{s}:{e}"), defs[*t].0.name.clone()))
            })
            .collect();
        let theirs: BTreeSet<(String, String, String)> = refs
            .iter()
            .filter(|r| r[0] == f)
            .map(|r| (r[1].clone(), r[2].clone(), r[3].clone()))
            .collect();
        assert_eq!(mine, theirs, "{f}: only Sophia {:?}, only Moonkale {:?}", mine.difference(&theirs).collect::<Vec<_>>(), theirs.difference(&mine).collect::<Vec<_>>());
    }
}

/// End to end: the ids Sophia stores for the fixtures are Moonkale's.
#[test]
fn stored_ids_are_moonkale_ids() {
    let ids = tsv("ids.tsv");
    for f in FILES {
        let src = std::fs::read_to_string(format!("{DIR}/{f}")).unwrap();
        let got = file(lang(f), &src).unwrap();
        for row in ids.iter().filter(|r| r[0] == f) {
            let i = got.defs.iter().position(|d| d.name == row[1]).unwrap();
            assert_eq!(got.records[i].id.to_hex(), row[2], "{f} {}", row[1]);
        }
    }
}
