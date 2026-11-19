//! E-0 (B-021): proof trust records on a real library. Checks every proof
//! of a Metamath database with `metamath.exe` (`verify proof *`), records one
//! annotation per proof (SPEC-006 §5: checker, version, kernel, status,
//! inputs), and asks which theorems rest only on checked proofs. Prints the
//! Markdown that goes into [[Benchmark Results]]. See `sophia-mm-trust.md`.
//!
//! Usage: sophia-mm-trust --exe PATH/metamath FILE.mm [--db NAME] [--runs R] [--taint LABEL] [--dir DIR]

use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sophia_ingest::trust::{Checker, RESTS_ON_CHECKED, RESTS_ON_UNCHECKED, metamath_verify_failures, proof_checks};
use sophia_query::{Limits, Strategy, prepare_text};
use sophia_store::Store;
use sophia_store::sqlite::Sqlite;

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn loadavg() -> String {
    std::fs::read_to_string("/proc/loadavg").map(|l| l.split_whitespace().take(3).collect::<Vec<_>>().join(" ")).unwrap_or_default()
}

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

/// Runs metamath.exe on `file` with `commands`; its standard output.
fn metamath(exe: &str, file: &str, commands: &[&str]) -> String {
    let path = std::path::Path::new(file);
    let mut child = Command::new(exe)
        .current_dir(path.parent().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let commands: Vec<String> = commands.iter().map(|c| c.to_string()).collect();
    let writer = std::thread::spawn(move || {
        writeln!(stdin, "set scroll continuous\nset width 100000\nread \"{name}\"").unwrap();
        for c in &commands {
            writeln!(stdin, "{c}").unwrap();
        }
        writeln!(stdin, "exit").unwrap();
    });
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn main() {
    let exe = arg("--exe").expect("--exe PATH to metamath.exe");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let file = {
        let mut i = 0;
        let mut found = None;
        while i < args.len() {
            if args[i].starts_with("--") {
                i += 2;
            } else {
                found = Some(args[i].clone());
                break;
            }
        }
        found.expect("FILE.mm")
    };
    let db = arg("--db").unwrap_or_else(|| "set".into());
    let runs: usize = arg("--runs").map(|s| s.parse().unwrap()).unwrap_or(5);
    let taint = arg("--taint").unwrap_or_else(|| "a1i".into());
    let base = arg("--dir").map(PathBuf::from).unwrap_or_else(|| {
        std::env::var_os("SOPHIA_CORPORA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cache/sophia-corpora"))
    });
    std::fs::create_dir_all(&base).unwrap();
    let dir = tempfile::tempdir_in(&base).unwrap();
    let load_before = loadavg();

    // The checker: metamath.exe's version from its banner.
    let banner = metamath(&exe, &file, &[]);
    let version = banner
        .lines()
        .find_map(|l| l.split("Version ").nth(1))
        .map(|v| v.split_whitespace().next().unwrap_or("").trim_end_matches(',').to_string())
        .unwrap_or_else(|| "unknown".into());
    let checker = Checker {
        name: "metamath.exe".into(),
        version: version.clone(),
        kernel: "metamath.exe verify proof (substitution and disjoint-variable checks)".into(),
    };

    // The store: the source manifest (the check's input) and the records.
    let text = std::fs::read_to_string(&file).unwrap();
    let path = dir.path().join("trust.sqlite");
    let store = Store::new(Sqlite::open(&path).unwrap());
    let t = Instant::now();
    let (report, manifest) = sophia_ingest::store_metamath_source(&store, &db, &text, 1000).unwrap();
    let ingest = t.elapsed();
    let parsed = sophia_ingest::metamath::parse(&text).unwrap();
    let proof_bytes: usize = parsed.iter().filter_map(|a| a.proof.as_ref()).map(|p| p.bytes.len()).sum();
    let size = |p: &std::path::Path| -> u64 {
        ["", "-wal"].iter().map(|s| std::fs::metadata(format!("{}{s}", p.display())).map(|m| m.len()).unwrap_or(0)).sum()
    };
    let size_before = size(&path);

    // The check.
    let t = Instant::now();
    let out = metamath(&exe, &file, &["verify proof *"]);
    let verify = t.elapsed();
    let failed = metamath_verify_failures(&out);

    // A simulated failure first: the label `taint` marked failed shows how
    // far one bad proof reaches; then the real result, which adds a
    // `checked` beside it (a re-check).
    let limits = Limits { max_rows: usize::MAX, max_steps_k: u64::MAX, max_derived: u64::MAX, max_bytes: u64::MAX, deadline: None };
    let count = |q: &str| -> (usize, Duration) {
        let p = prepare_text(q).unwrap();
        let mut times = Vec::new();
        let mut n = 0;
        for _ in 0..runs {
            let t = Instant::now();
            n = store.read(|tx| Ok(p.run(tx, &[], limits, Strategy::Auto).unwrap())).unwrap().rows.len();
            times.push(t.elapsed());
        }
        (n, median(times))
    };
    let mut simulated = failed.clone();
    simulated.insert(taint.clone());
    let (sim_anns, unknown) = proof_checks(&checker, &parsed, &simulated, &[manifest]);
    assert!(unknown.is_empty() || unknown == vec![taint.clone()], "labels reported failed without a proof: {unknown:?}");
    store.annotate_many(&sim_anns).unwrap();
    let (sim_checked, _) = count(RESTS_ON_CHECKED);
    let (sim_unchecked, _) = count(RESTS_ON_UNCHECKED);

    let (anns, _) = proof_checks(&checker, &parsed, &failed, &[manifest]);
    let ann_bytes: usize =
        anns.iter().map(|a| a.payload.len() + 32 * (1 + a.subjects.len() + a.inputs.len()) + a.kind.len() + a.status.len() + 16).sum();
    let size_mid = size(&path);
    let t = Instant::now();
    store.annotate_many(&anns).unwrap();
    let annotate = t.elapsed();
    let size_after = size(&path);
    let (checked, t_checked) = count(RESTS_ON_CHECKED);
    let (unchecked, t_unchecked) = count(RESTS_ON_UNCHECKED);

    println!("#### E-0 proof trust on `{db}` (B-021), metamath.exe {version}\n");
    println!("Load average before {load_before}, after {}; SQLite store; {runs} runs per query (median).\n", loadavg());
    println!("| measure | value |\n|---|---|");
    println!("| assertions / distinct proofs | {} / {} |", report.assertions, anns.len());
    println!("| ingest with source manifest | {ingest:.1?} |");
    println!("| `verify proof *` (metamath.exe, reads the file) | {verify:.2?} |");
    // metamath.exe prints "All proofs in the database were verified" even
    // after an error, so only its `?Error` lines count.
    println!("| proofs reported failed (`?Error` lines) | {} |", failed.len());
    println!("| proof records, content bytes | {proof_bytes} |");
    println!(
        "| check annotations, logical bytes (estimate) | {ann_bytes} ({:.1} % of the proofs) |",
        100.0 * ann_bytes as f64 / proof_bytes as f64
    );
    println!(
        "| store on disk: before checks / after the simulated pass / after the real one | {} / {} / {} bytes |",
        size_before, size_mid, size_after
    );
    println!("| annotating (one transaction) | {annotate:.2?} |");
    println!("| rests on checked proofs only | {checked} proofs, {t_checked:.2?} |");
    println!("| rests on an unchecked proof | {unchecked} proofs, {t_unchecked:.2?} |");
    println!("| simulated: `{taint}` failed | {sim_unchecked} proofs tainted, {sim_checked} clean |");
    if !failed.is_empty() {
        let shown: BTreeSet<&String> = failed.iter().take(20).collect();
        println!("\nFailed: {shown:?}");
    }
}
