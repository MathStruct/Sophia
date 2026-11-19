//! E-2 (B-023): do two gemm implementations share structure that a query
//! can find without names? See `gemm_structure.md`.
//!
//! ```sh
//! cargo run --release -p sophia-ingest --example gemm_structure -- \
//!     OUT_DIR crates/sophia-ingest/examples/gemm_structure.tsv \
//!     rust:matrixmultiply:DIR/src julia:Octavian:DIR/src/Octavian.jl julia:matmul:DIR/matmul.jl
//! ```
//!
//! For every definition of every corpus, refused or not, it computes
//! name-free features from the syntax tree (loop-nest depth, loop keywords
//! hidden in macro text, calls), and scores two structural predicates
//! against hand-made role labels twice: on the source (calls resolved by
//! name, every definition) and in the store (loop-depth facts on the stored
//! definitions, calls as `refs` or `uses`+`member`, the predicates as Sophia
//! queries).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use moonkale_content_id::{Budget, Syntax, parse};
use sophia_hash::Digest;
use sophia_query::{Limits, Strategy, Val, prepare_text};
use sophia_store::{Fact, Input, Store, sqlite::Sqlite};

/// One definition found in the source.
struct D {
    corpus: String,
    file: String,
    name: String,
    kind: String,
    /// The record id, when the normal form accepted it.
    id: Option<Digest>,
    /// `ok`, or why it has no id.
    status: String,
    /// Deepest loop nest (Julia: one level per `for` binding).
    depth: usize,
    loops: usize,
    /// Loop keywords inside macro token trees (Rust): loops the tree does not show.
    macro_loops: usize,
    calls: BTreeSet<String>,
    roles: Vec<String>,
}

fn loop_level(lang: &str, n: Syntax) -> usize {
    match (lang, n.kind()) {
        ("rust", "for_expression" | "while_expression" | "loop_expression") => 1,
        ("julia", "while_statement") => 1,
        ("julia", "for_statement") => {
            let mut c = n.walk();
            n.named_children(&mut c).filter(|k| k.kind() == "for_binding").count().max(1)
        }
        _ => 0,
    }
}

/// The name a call's callee ends in, if it has one.
fn callee(lang: &str, n: Syntax, src: &str) -> Option<String> {
    let text = |m: Syntax| src[m.start_byte()..m.end_byte()].to_string();
    let f = match lang {
        "rust" => n.child_by_field_name("function")?,
        _ => n.named_child(0)?,
    };
    let mut f = f;
    loop {
        match f.kind() {
            "identifier" | "field_identifier" | "operator" => return Some(text(f)),
            "scoped_identifier" => f = f.child_by_field_name("name")?,
            "field_expression" if lang == "rust" => f = f.child_by_field_name("field")?,
            "field_expression" => f = f.named_child(u32::try_from(f.named_child_count().checked_sub(1)?).ok()?)?,
            "generic_function" => f = f.child_by_field_name("function")?,
            "parametrized_type_expression" => f = f.named_child(0)?,
            _ => return None,
        }
    }
}

fn features(lang: &str, n: Syntax, src: &str, depth: usize, d: &mut D) {
    let l = loop_level(lang, n);
    let depth = depth + l;
    if l > 0 {
        d.depth = d.depth.max(depth);
        d.loops += 1;
    }
    match (lang, n.kind()) {
        (_, "call_expression") => {
            if let Some(c) = callee(lang, n, src) {
                d.calls.insert(c);
            }
        }
        ("rust", "macro_invocation") => {
            if let Some(m) = n.child_by_field_name("macro") {
                d.calls.insert(format!("{}!", &src[m.start_byte()..m.end_byte()]));
            }
        }
        ("rust", "token_tree") => {
            let mut stack = vec![n];
            while let Some(t) = stack.pop() {
                if matches!(t.kind(), "for" | "while" | "loop") {
                    d.macro_loops += 1;
                }
                let mut c = t.walk();
                stack.extend(t.children(&mut c));
            }
            return;
        }
        _ => {}
    }
    let mut c = n.walk();
    for ch in n.children(&mut c) {
        features(lang, ch, src, depth, d);
    }
}

fn short(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

/// (path, text) of each source file.
type Files = Vec<(String, String)>;

/// Every definition of one corpus, with the ids the store will give them.
fn read_corpus(lang: &str, name: &str, path: &Path) -> Result<(Vec<D>, Files), Box<dyn std::error::Error>> {
    let mut out = Vec::new();
    // (file, start, end) -> id or refusal
    let mut status: HashMap<(String, usize, usize), Result<Digest, String>> = HashMap::new();
    let files: Vec<(String, String)> = if lang == "julia" {
        let disc = sophia_ingest::module::discover(path)?;
        let ing = sophia_ingest::module::ingest(&disc)?;
        for (p, v) in &ing.files {
            for (def, rec) in v {
                status.insert((p.clone(), def.start, def.end), Ok(rec.id));
            }
        }
        disc.files.iter().map(|f| (f.path.clone(), f.text.clone())).collect()
    } else {
        let mut files = Vec::new();
        let mut stack = vec![path.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d)? {
                let p = e?.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    files.push((p.display().to_string(), std::fs::read_to_string(&p)?));
                }
            }
        }
        files.sort();
        files
    };
    for (p, src) in &files {
        // Per-file refusal reasons (Rust ids too: the store ingests Rust per file).
        if let Ok(f) = sophia_ingest::code::file(lang, src) {
            if lang == "rust" {
                for (def, rec) in f.defs.iter().zip(&f.records) {
                    status.insert((p.clone(), def.start, def.end), Ok(rec.id));
                }
            }
            for (def, why) in f.refused {
                status.entry((p.clone(), def.start, def.end)).or_insert(Err(why));
            }
        }
        let tree = parse(lang, src, &mut Budget::new())?;
        for (def, node) in sophia_ingest::code::definitions(lang, tree.root_node(), src) {
            let st = status.get(&(p.clone(), def.start, def.end)).cloned().unwrap_or(Err("no id in module mode".into()));
            let mut d = D {
                corpus: name.into(),
                file: short(p),
                name: def.name.clone(),
                kind: def.kind.clone(),
                id: st.as_ref().ok().copied(),
                status: match &st {
                    Ok(_) => "ok".into(),
                    Err(e) => e.clone(),
                },
                depth: 0,
                loops: 0,
                macro_loops: 0,
                calls: BTreeSet::new(),
                roles: Vec::new(),
            };
            features(lang, node, src, 0, &mut d);
            d.calls.remove(&d.name);
            out.push(d);
        }
    }
    Ok((out, files))
}

/// Source level: does `i` reach (through calls resolved by name within its
/// corpus) a definition with a loop?
fn reaches_loop(defs: &[D], by_name: &HashMap<(&str, &str), Vec<usize>>, i: usize) -> bool {
    let mut seen = BTreeSet::from([i]);
    let mut stack = vec![i];
    while let Some(j) = stack.pop() {
        for c in &defs[j].calls {
            for &k in by_name.get(&(defs[j].corpus.as_str(), c.as_str())).into_iter().flatten() {
                if seen.insert(k) {
                    if defs[k].depth >= 1 {
                        return true;
                    }
                    stack.push(k);
                }
            }
        }
    }
    false
}

/// Definitions that contain others: their loops are their contents' loops.
/// Not candidates of a predicate (Julia modules are not stored records).
const CONTAINERS: [&str; 2] = ["module_definition", "mod_item"];

const CALLS: &str = "    calls(a, b) = refs(a, b)
    calls(a, b) = uses(a, n) && member(n, b)
    reach(a, b) = calls(a, b)
    reach(a, c) = calls(a, b) && reach(b, c)
    below(a) = reach(a, b) && fact(_, :e2_depth_ge1, b)
";

fn run(store: &Store<Sqlite>, text: &str) -> Result<BTreeSet<Digest>, Box<dyn std::error::Error>> {
    let q = prepare_text(text)?;
    let rows = store.read(|tx| Ok(q.run(tx, &[], Limits { max_rows: usize::MAX, ..Limits::default() }, Strategy::Auto)))??;
    Ok(rows
        .rows
        .into_iter()
        .filter_map(|r| match &r[0] {
            Val::Id(d) => Some(*d),
            _ => None,
        })
        .collect())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [out, labels, corpora @ ..] = &args[..] else {
        return Err("usage: gemm_structure OUT_DIR LABELS.tsv lang:name:path…".into());
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out)?;
    let db = out.join("e2.sqlite");
    for ext in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{ext}", db.display()));
    }
    let store = Store::new(Sqlite::open(&db)?);

    let mut defs: Vec<D> = Vec::new();
    for spec in corpora {
        let [lang, name, path] = spec.splitn(3, ':').collect::<Vec<_>>()[..] else { return Err(format!("bad corpus {spec}").into()) };
        let (ds, files) = read_corpus(lang, name, Path::new(path))?;
        defs.extend(ds);
        let r = if lang == "julia" {
            sophia_ingest::store_module(&store, name, Path::new(path))?.0
        } else {
            sophia_ingest::store_code(&store, lang, name, &files)?
        };
        println!(
            "ingest {name}: {} files, {} definitions stored, {} refused",
            r.files,
            r.definitions,
            r.refused_definitions.values().sum::<usize>()
        );
    }

    // Labels: corpus, file, name, role (a definition may have several roles).
    let mut labelled: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for line in std::fs::read_to_string(labels)?.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let [corpus, file, name, role] = f[..] else { return Err(format!("bad label line {line:?}").into()) };
        let hit = defs.iter_mut().filter(|d| d.corpus == corpus && d.file == file && d.name == name).fold(0, |n, d| {
            d.roles.push(role.into());
            n + 1
        });
        if hit == 0 {
            return Err(format!("label matches no definition: {line}").into());
        }
        labelled.entry((corpus.into(), role.into())).or_default().insert(format!("{file}:{name}"));
    }

    // Name-free loop facts on the stored definitions.
    let mut facts = Vec::new();
    for d in &defs {
        // Containers get no facts, so the store query excludes them as the
        // source predicate does (Mistral's F3 on B-023).
        let Some(id) = d.id.filter(|_| !CONTAINERS.contains(&d.kind.as_str())) else { continue };
        for k in 1..=d.depth.min(4) {
            facts.push(Fact::new(&format!("e2_depth_ge{k}"), id, Vec::new(), ("e2-gemm-structure", "0"), vec![Input::Content(id)], 0));
        }
    }
    store.record_facts(&facts)?;

    // Per-definition table.
    let mut tsv = String::from("corpus\tfile\tname\tkind\tstatus\tdepth\tloops\tmacro_loops\troles\tcalls\n");
    for d in &defs {
        tsv += &format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            d.corpus,
            d.file,
            d.name,
            d.kind,
            d.status,
            d.depth,
            d.loops,
            d.macro_loops,
            d.roles.join(","),
            d.calls.iter().cloned().collect::<Vec<_>>().join(" ")
        );
    }
    std::fs::write(out.join("definitions.tsv"), tsv)?;

    // Coverage of functions with and without a loop, per corpus.
    println!("\n## Functions by loop and status (corpus, has a loop, status: count)");
    let mut cov: BTreeMap<(&str, bool, &str), usize> = BTreeMap::new();
    for d in &defs {
        if matches!(d.kind.as_str(), "function_item" | "function_definition") {
            *cov.entry((d.corpus.as_str(), d.depth > 0, d.status.as_str())).or_default() += 1;
        }
    }
    for ((c, l, s), n) in &cov {
        println!("{c}\t{}\t{s}\t{n}", if *l { "loop" } else { "no loop" });
    }

    // Coverage of the labelled definitions.
    println!("\n## Labelled definitions (corpus, role: in store / labelled; depth, macro loops)");
    for ((corpus, role), names) in &labelled {
        let ds: Vec<&D> = defs.iter().filter(|d| &d.corpus == corpus && d.roles.contains(role)).collect();
        let stored = ds.iter().filter(|d| d.id.is_some()).count();
        let detail: Vec<String> =
            ds.iter().map(|d| format!("{}:{} {} d={} m={}", d.file, d.name, d.status, d.depth, d.macro_loops)).collect();
        println!("{corpus}\t{role}\t{stored}/{} ({} names)\t{}", ds.len(), names.len(), detail.join("; "));
    }

    // The two predicates, on the source and in the store.
    let mut by_name: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (i, d) in defs.iter().enumerate() {
        by_name.entry((d.corpus.as_str(), d.name.as_str())).or_default().push(i);
    }
    let below: Vec<bool> = (0..defs.len()).map(|i| reaches_loop(&defs, &by_name, i)).collect();
    let preds: [(&str, &[&str], String); 2] = [
        (
            "nest-over-nest",
            &["driver"],
            format!("query e2_nest_over_nest()\n{CALLS}    select a where fact(_, :e2_depth_ge2, a) && below(a)\nend"),
        ),
        (
            "leaf-nest",
            &["pack", "macrokernel", "microkernel", "naive"],
            format!("query e2_leaf_nest()\n{CALLS}    select a where fact(_, :e2_depth_ge2, a) && !below(a)\nend"),
        ),
    ];
    let mut by_id: HashMap<Digest, Vec<usize>> = HashMap::new();
    for (i, d) in defs.iter().enumerate() {
        if let Some(id) = d.id {
            by_id.entry(id).or_default().push(i);
        }
    }
    println!("\n## Predicates (per corpus: matched, of them labelled with an expected role; expected roles found / labelled)");
    let corpora_names: BTreeSet<&str> = defs.iter().map(|d| d.corpus.as_str()).collect();
    for (pname, expected, query) in &preds {
        let source: BTreeSet<usize> = (0..defs.len())
            .filter(|&i| !CONTAINERS.contains(&defs[i].kind.as_str()) && defs[i].depth >= 2 && (below[i] == (*pname == "nest-over-nest")))
            .collect();
        let t = std::time::Instant::now();
        let ids = run(&store, query)?;
        let elapsed = t.elapsed();
        let in_store: BTreeSet<usize> = ids.iter().flat_map(|id| by_id.get(id).cloned().unwrap_or_default()).collect();
        println!("{pname} (query {:.1?}):", elapsed);
        for c in &corpora_names {
            for (level, set) in [("source", &source), ("store", &in_store)] {
                let m: Vec<usize> = set.iter().copied().filter(|&i| defs[i].corpus == *c).collect();
                let good = m.iter().filter(|&&i| defs[i].roles.iter().any(|r| expected.contains(&r.as_str()))).count();
                let want: Vec<usize> = (0..defs.len())
                    .filter(|&i| defs[i].corpus == *c && defs[i].roles.iter().any(|r| expected.contains(&r.as_str())))
                    .collect();
                let found = want.iter().filter(|i| m.contains(i)).count();
                let names: Vec<String> =
                    m.iter().map(|&i| format!("{}{}", defs[i].name, if defs[i].roles.is_empty() { "" } else { "*" })).collect();
                println!(
                    "  {c}\t{level}\tmatched {}\tlabelled-expected {good}\trecall {found}/{}\t{}",
                    m.len(),
                    want.len(),
                    names.join(" ")
                );
            }
        }
    }
    println!("\nwrote {}", out.join("definitions.tsv").display());
    Ok(())
}
