//! Checks Sophia's proof-dependency answers against `metamath.exe`, the
//! reference Metamath program, which Sophia shares no code with. For a sample
//! of theorems it asks both "which axioms and which definitions does this
//! theorem rest on?" and compares the sets. Acceptance test for B-16
//! ([[Real Code Corpora]] rung 1; the external review asked for it).
//!
//! Usage: sophia-mm-check --exe PATH/metamath FILE.mm [--db NAME] [--sample N] [--seed S] [LABEL ...]
//!
//! Sophia's side is the query the benchmark times: the transitive closure
//! of a proof's references through proofs, filtered to the statements in
//! the library's `axioms` (or `definitions`) collection. `metamath.exe`'s
//! side is `show trace_back LABEL /axioms /essential`, whose list mixes
//! axioms and definitions; they are split by Sophia's own rule (`df-`).
//! Two labels whose statements are identical have one id in Sophia, so the
//! comparison is by statement id, not label.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

use sophia_hash::Digest;
use sophia_query::{Limits, Strategy, Val, prepare_text};
use sophia_store::Store;

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn main() {
    let exe = arg("--exe").expect("--exe PATH to metamath.exe");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i].starts_with("--") {
            i += 2;
        } else {
            positional.push(args[i].clone());
            i += 1;
        }
    }
    let file = positional.first().expect("FILE.mm").clone();
    let db = arg("--db").unwrap_or_else(|| "set".into());
    let sample: usize = arg("--sample").map(|s| s.parse().unwrap()).unwrap_or(200);
    let seed: u64 = arg("--seed").map(|s| s.parse().unwrap()).unwrap_or(1);

    let text = std::fs::read_to_string(&file).unwrap();
    let parsed = sophia_ingest::metamath::parse(&text).unwrap();
    let id_of: HashMap<String, Digest> = parsed.iter().map(|a| (a.label.clone(), a.record.id)).collect();
    let role: HashMap<String, String> = parsed.iter().map(|a| (a.label.clone(), a.role.clone())).collect();
    let theorems: Vec<String> = parsed.iter().filter(|a| a.proof.is_some()).map(|a| a.label.clone()).collect();

    // The sample: named labels, then a seeded draw (xorshift) over theorems.
    let mut labels: Vec<String> = positional[1..].to_vec();
    let mut x = seed.max(1);
    let mut picked = BTreeSet::new();
    while picked.len() < sample.min(theorems.len()) {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        picked.insert((x % theorems.len() as u64) as usize);
    }
    labels.extend(picked.into_iter().map(|i| theorems[i].clone()));

    // Sophia.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(sophia_store::lmdb_store::Lmdb::open(dir.path().join("mm")).unwrap());
    let t = Instant::now();
    sophia_ingest::store_metamath(&store, &db, &parsed, 1000).unwrap();
    eprintln!("ingested {} assertions in {:.1?}", parsed.len(), t.elapsed());
    let rests = prepare_text(
        "query rests_on(proof::Id, coll::Text)\n below(x) = refs(proof, x)\n below(y) = below(x) && def(x, :proof) && refs(x, y)\n select s where below(s) && def(s, :statement) && member(c, s) && name(coll, c)\nend",
    )
    .unwrap();
    let limits = Limits { max_rows: usize::MAX, ..Limits::default() };
    let ask = |label: &str, which: &str| -> BTreeSet<Digest> {
        let proof = store.resolve(&sophia_ingest::metamath::proof_name_for(&db, label)).unwrap().unwrap().target.digest();
        let args = vec![Val::Id(proof), Val::Text(sophia_ingest::library_collection(&db, which))];
        let rows = store.read(|tx| Ok(rests.run(tx, &args, limits, Strategy::Auto).unwrap())).unwrap();
        rows.rows
            .into_iter()
            .map(|r| match &r[0] {
                Val::Id(d) => *d,
                v => panic!("{v:?}"),
            })
            .collect()
    };
    let t = Instant::now();
    let sophia: Vec<(BTreeSet<Digest>, BTreeSet<Digest>)> = labels.iter().map(|l| (ask(l, "axioms"), ask(l, "definitions"))).collect();
    let sophia_time = t.elapsed();

    // metamath.exe, one session for all labels.
    let mm_dir = std::path::Path::new(&file).parent().unwrap();
    let mut child = Command::new(&exe)
        .current_dir(mm_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // Commands are written from a thread: metamath.exe's output fills its
    // pipe long before thousands of commands are written, so writing
    // everything before reading would deadlock.
    let mut stdin = child.stdin.take().unwrap();
    let name = std::path::Path::new(&file).file_name().unwrap().to_string_lossy().to_string();
    let commands = labels.clone();
    let writer = std::thread::spawn(move || {
        writeln!(stdin, "set scroll continuous\nset width 100000\nread \"{name}\"").unwrap();
        for l in &commands {
            writeln!(stdin, "show trace_back {l} /axioms /essential").unwrap();
        }
        writeln!(stdin, "exit").unwrap();
    });
    let t = Instant::now();
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap();
    let mm_time = t.elapsed();
    let out = String::from_utf8_lossy(&out.stdout);
    // `Statement "X" assumes the following axioms ($a statements):` then
    // the labels, up to the next prompt.
    let mut mm: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in out.lines() {
        // Commands come from stdin, so they are not echoed: output follows
        // the prompt on the same line.
        let mut line = line;
        if let Some(rest) = line.strip_prefix("MM>") {
            current = None;
            line = rest.trim_start();
        }
        if let Some(rest) = line.strip_prefix("Statement \"") {
            let l = rest.split('"').next().unwrap().to_string();
            mm.entry(l.clone()).or_default();
            current = Some(l);
        } else if let Some(l) = &current {
            // `(None)`: the theorem assumes no axioms.
            mm.get_mut(l).unwrap().extend(line.split_whitespace().filter(|w| *w != "(None)").map(String::from));
        }
    }

    // Compare by statement id.
    let mut agree = 0;
    let (mut ax_total, mut df_total) = (0, 0);
    for (l, (ax, df)) in labels.iter().zip(&sophia) {
        let Some(theirs) = mm.get(l) else {
            println!("{l}: no answer from metamath.exe");
            continue;
        };
        let pick = |r: &str| -> BTreeSet<Digest> {
            theirs.iter().filter(|x| role.get(*x).map(String::as_str) == Some(r)).map(|x| id_of[x]).collect()
        };
        let (mm_ax, mm_df) = (pick("axiom"), pick("definition"));
        // A theorem that proves syntax (`$p wff …`: weq, wel, bj-0 in
        // set.mm) has syntax constructors among its essential steps, and
        // metamath.exe lists them; Sophia's question is about axioms and
        // definitions only.
        let others: Vec<&String> = theirs
            .iter()
            .filter(|x| !matches!(role.get(*x).map(String::as_str), Some("axiom" | "definition" | "syntax")))
            .collect();
        ax_total += mm_ax.len();
        df_total += mm_df.len();
        if *ax == mm_ax && *df == mm_df && others.is_empty() {
            agree += 1;
        } else {
            let name = |d: &Digest| {
                let mut n: Vec<&String> = id_of.iter().filter(|(_, v)| *v == d).map(|(k, _)| k).collect();
                n.sort();
                n.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("=")
            };
            let diff = |a: &BTreeSet<Digest>, b: &BTreeSet<Digest>| a.difference(b).map(name).collect::<Vec<_>>().join(" ");
            println!(
                "{l}: DIFFER axioms only-sophia [{}] only-mm [{}]; definitions only-sophia [{}] only-mm [{}]; unclassified {others:?}",
                diff(ax, &mm_ax),
                diff(&mm_ax, ax),
                diff(df, &mm_df),
                diff(&mm_df, df)
            );
        }
    }
    println!(
        "{agree} of {} theorems agree with metamath.exe ({} axiom and {} definition dependencies compared); Sophia {:.1?} for {} queries, metamath.exe {:.1?} including reading the file",
        labels.len(),
        ax_total,
        df_total,
        sophia_time,
        2 * labels.len(),
        mm_time
    );
}
