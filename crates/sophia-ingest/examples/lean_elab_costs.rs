//! B-035 part 2, the expensive-fact workload of the fact cache experiment
//! (SPEC-001 §8a, SR-152): what Lean's elaborator spends per command on a
//! deterministic sample of Mathlib files. The cost of a fact "the elaborated
//! declaration" is that time. Nothing in Mathlib is changed. See
//! `lean_elab_costs.md`.
//!
//! ```sh
//! PATH=~/.elan/bin:$PATH cargo run --release -p sophia-ingest --example lean_elab_costs -- \
//!     OUTDIR MATHLIB_ROOT DRIVER [--sample N] [--seed S]
//! ```
//!
//! DRIVER is `experiments/lean-elab-costs/elab_costs.lean`. Each sampled file
//! is elaborated three times, always synchronously (`Elab.async = false`, so a
//! proof's time is charged to its command):
//! 1. by `lean FILE`, to bring its imports into the page cache;
//! 2. by the driver, which times each source command through Lean's own
//!    frontend and prints one row per command;
//! 3. by `lean FILE` again, for the file's wall time (the cross-check).
//!
//! A file with any error (an exit status from `lean`, or an error message in
//! the driver's rows) stops the run.
//!
//! Lean's trace profiler is not used: it prints each macro-expanded command
//! as a top-level node beside the command it came from, so summing its
//! `[Elab.command]` lines counts expansions twice.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

type R<T> = Result<T, Box<dyn std::error::Error>>;

/// One row of the driver's output.
#[derive(Clone, Debug)]
struct Row {
    line: usize,
    kind: String,
    decl_kind: String,
    added: usize,
    errors: usize,
    ms: f64,
    heartbeats: u64,
}

/// Runs `lean` in `root` with `args`; returns stdout and the wall time.
fn lean(root: &Path, args: &[&str]) -> R<(String, f64)> {
    let start = Instant::now();
    let out = Command::new("lake").arg("env").arg("lean").args(args).current_dir(root).stderr(Stdio::inherit()).output()?;
    let wall = start.elapsed().as_secs_f64();
    if !out.status.success() {
        return Err(format!("lean {args:?} exited with {}", out.status).into());
    }
    Ok((String::from_utf8(out.stdout)?, wall))
}

/// Parses the driver's rows; the `#header` row gives the import time in ms.
/// An error in the header or in any command is an error here.
fn parse(out: &str) -> R<(f64, Vec<Row>)> {
    let mut header = f64::NAN;
    let mut rows = Vec::new();
    for l in out.lines() {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() != 8 {
            return Err(format!("bad driver row {l:?}").into());
        }
        let ms = f[6].parse::<f64>()? / 1e6;
        if f[5] != "0" {
            return Err(format!("the driver reports errors: {l:?}").into());
        }
        if f[0] == "#header" {
            header = ms;
            continue;
        }
        rows.push(Row {
            line: f[1].parse()?,
            kind: short(f[2]),
            decl_kind: short(f[3]),
            added: f[4].parse()?,
            errors: f[5].parse()?,
            ms,
            heartbeats: f[7].parse()?,
        });
    }
    Ok((header, rows))
}

/// `Lean.Parser.Command.theorem` -> `theorem`.
fn short(kind: &str) -> String {
    kind.strip_prefix("Lean.Parser.Command.").unwrap_or(kind).to_string()
}

/// A command that declares something: Lean's `declaration` command (theorem,
/// def, instance, abbrev, structure, class, inductive, example, opaque,
/// axiom), or any command that added a constant (Mathlib's `irreducible_def`,
/// `alias`, `mutual`, `initialize_simps_projections`, ...).
fn is_declaration(r: &Row) -> bool {
    const KINDS: [&str; 10] =
        ["theorem", "definition", "instance", "abbrev", "structure", "classInductive", "inductive", "example", "opaque", "axiom"];
    r.added > 0 || KINDS.contains(&r.decl_kind.as_str())
}

/// A deterministic sample: the `n` files with the smallest hash of
/// (seed, path), so the same seed gives the same files on any machine.
fn sample(files: &[String], n: usize, seed: u64) -> Vec<String> {
    let mut keyed: Vec<(u64, &String)> = files
        .iter()
        .map(|f| {
            // FNV-1a over the seed and the path.
            let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ seed;
            for b in f.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
            (h, f)
        })
        .collect();
    keyed.sort();
    let mut out: Vec<String> = keyed.into_iter().take(n).map(|(_, f)| f.clone()).collect();
    out.sort();
    out
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) -> R<()> {
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            walk(&p, root, out)?;
        } else if p.extension().is_some_and(|x| x == "lean") {
            out.push(p.strip_prefix(root)?.to_string_lossy().into_owned());
        }
    }
    Ok(())
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

/// `--sample N` (N >= 1, default 40) and `--seed S` (default 35).
fn options(args: &[String]) -> R<(usize, u64)> {
    let (mut n, mut seed) = (40usize, 35u64);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let v = it.next().ok_or(format!("{a} needs a value"))?;
        match a.as_str() {
            "--sample" => n = v.parse()?,
            "--seed" => seed = v.parse()?,
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    if n == 0 {
        return Err("--sample must be at least 1".into());
    }
    Ok((n, seed))
}

fn main() -> R<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        return Err("usage: lean_elab_costs OUTDIR MATHLIB_ROOT DRIVER [--sample N] [--seed S]".into());
    }
    let outdir = PathBuf::from(&args[0]);
    let root = PathBuf::from(&args[1]);
    let driver = std::fs::canonicalize(&args[2])?;
    let driver = driver.to_str().ok_or("driver path is not UTF-8")?;
    let (n, seed) = options(&args[3..])?;
    std::fs::create_dir_all(&outdir)?;

    let mut all = Vec::new();
    walk(&root.join("Mathlib"), &root, &mut all)?;
    all.sort();
    let files = sample(&all, n, seed);
    eprintln!("{} Mathlib files, sampling {} (seed {seed})", all.len(), files.len());

    let mut cmd_tsv = std::fs::File::create(outdir.join("commands.tsv"))?;
    writeln!(cmd_tsv, "file\tindex\tline\tkind\tdecl_kind\tdeclaration\tconstants_added\terrors\tms\theartbeats")?;
    let mut file_tsv = std::fs::File::create(outdir.join("files.tsv"))?;
    writeln!(file_tsv, "file\tlines\twall_lean_s\theader_ms\tcommands\tdeclarations\tcommands_ms\tdeclarations_ms\terrors")?;

    // (is a declaration, ms) of every command in the sample.
    let mut costs: Vec<(bool, f64)> = Vec::new();
    let mut kinds: BTreeMap<String, (usize, f64)> = BTreeMap::new();
    let (mut wall_total, mut header_total, mut cmd_total) = (0.0, 0.0, 0.0);
    for (fi, f) in files.iter().enumerate() {
        let lines = std::fs::read_to_string(root.join(f))?.lines().count();
        let module = f.trim_end_matches(".lean").replace('/', ".");
        lean(&root, &["-DElab.async=false", f])?;
        let (out, _) = lean(&root, &["--run", driver, f, &module])?;
        let (_, wall) = lean(&root, &["-DElab.async=false", f])?;
        let (header, rows) = parse(&out)?;
        let (mut sum, mut sum_decl, mut decls) = (0.0, 0.0, 0);
        for (k, r) in rows.iter().enumerate() {
            let decl = is_declaration(r);
            let kind = if r.kind == "declaration" || r.kind == "in" { &r.decl_kind } else { &r.kind };
            let e = kinds.entry(kind.clone()).or_default();
            e.0 += 1;
            e.1 += r.ms;
            sum += r.ms;

            if decl {
                sum_decl += r.ms;
                decls += 1;
            }
            writeln!(
                cmd_tsv,
                "{f}\t{k}\t{}\t{}\t{}\t{decl}\t{}\t{}\t{:.4}\t{}",
                r.line, r.kind, r.decl_kind, r.added, r.errors, r.ms, r.heartbeats
            )?;
            costs.push((decl, r.ms));
        }
        wall_total += wall;
        header_total += header;
        cmd_total += sum;
        writeln!(file_tsv, "{f}\t{lines}\t{wall:.3}\t{header:.1}\t{}\t{decls}\t{sum:.1}\t{sum_decl:.1}\t0", rows.len())?;
        eprintln!(
            "[{}/{}] {f}: {lines} lines, {} commands, lean {wall:.2} s, imports {:.2} s, commands {:.2} s",
            fi + 1,
            files.len(),
            rows.len(),
            header / 1e3,
            sum / 1e3
        );
    }

    // Summary: for declarations and for all commands, the share above each
    // threshold, by count and by time.
    let mut sum_tsv = std::fs::File::create(outdir.join("summary.tsv"))?;
    writeln!(sum_tsv, "set\tthreshold_ms\tcount_above\tshare_count\tms_above\tshare_time\tshare_all_command_time")?;
    let all_ms: f64 = costs.iter().map(|c| c.1).sum();
    for (name, only_decl) in [("declarations", true), ("all commands", false)] {
        let mut v: Vec<f64> = costs.iter().filter(|(d, _)| !only_decl || *d).map(|c| c.1).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        let total: f64 = v.iter().sum();
        println!(
            "{name}: {} commands, {:.1} s; p10 {:.3} p50 {:.3} p90 {:.2} p99 {:.1} max {:.0} ms",
            v.len(),
            total / 1e3,
            quantile(&v, 0.1),
            quantile(&v, 0.5),
            quantile(&v, 0.9),
            quantile(&v, 0.99),
            v.last().copied().unwrap_or(f64::NAN)
        );
        for t in [0.01, 0.1, 1.0, 10.0, 100.0, 1000.0] {
            let above: Vec<f64> = v.iter().copied().filter(|x| *x >= t).collect();
            let ms: f64 = above.iter().sum();
            writeln!(
                sum_tsv,
                "{name}\t{t}\t{}\t{:.4}\t{ms:.1}\t{:.4}\t{:.4}",
                above.len(),
                above.len() as f64 / v.len() as f64,
                ms / total,
                ms / all_ms
            )?;
            println!(
                "  >= {t:>7} ms: {:>6} ({:5.1} %) of the commands, {:5.1} % of their time, {:5.1} % of all command time",
                above.len(),
                100.0 * above.len() as f64 / v.len() as f64,
                100.0 * ms / total,
                100.0 * ms / all_ms
            );
        }
    }
    println!(
        "{} files: lean {wall_total:.1} s; driver imports {:.1} s, commands {:.1} s",
        files.len(),
        header_total / 1e3,
        cmd_total / 1e3
    );
    let mut by_time: Vec<_> = kinds.into_iter().collect();
    by_time.sort_by(|a, b| b.1.1.total_cmp(&a.1.1));
    for (k, (c, ms)) in by_time.iter().take(15) {
        println!("  {k}: {c} commands, {:.1} s", ms / 1e3);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows() {
        let out = "#header\t1\t-\t-\t0\t0\t934592540\t0\n\
                   3\t40\tLean.Parser.Command.declaration\tLean.Parser.Command.theorem\t1\t0\t2500000\t77\n\
                   4\t44\tLean.Parser.Command.in\tLean.Parser.Command.attribute\t0\t0\t100000\t5\n\
                   5\t50\tLean.Parser.Command.in\tLean.Parser.Command.example\t0\t0\t100000\t5\n";
        let (h, rows) = parse(out).unwrap();
        assert!((h - 934.59254).abs() < 1e-6);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].decl_kind, "theorem");
        assert!((rows[0].ms - 2.5).abs() < 1e-9);
        assert!(is_declaration(&rows[0]) && !is_declaration(&rows[1]) && is_declaration(&rows[2]));
        assert!(parse("1\t2\n").is_err());
        assert!(parse("3\t40\tLean.Parser.Command.declaration\tLean.Parser.Command.theorem\t1\t2\t2500000\t77\n").is_err());
        assert!(parse("#header\t1\t-\t-\t0\t1\t934592540\t0\n").is_err());
    }

    #[test]
    fn arguments() {
        let a = |v: &[&str]| options(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(a(&[]).unwrap(), (40, 35));
        assert_eq!(a(&["--sample", "60", "--seed", "7"]).unwrap(), (60, 7));
        assert!(a(&["--sample"]).is_err());
        assert!(a(&["--sample", "0"]).is_err());
        assert!(a(&["--bogus", "1"]).is_err());
    }

    #[test]
    fn sample_is_deterministic() {
        let files: Vec<String> = (0..100).map(|i| format!("Mathlib/F{i}.lean")).collect();
        let a = sample(&files, 10, 35);
        assert_eq!(a, sample(&files, 10, 35));
        assert_eq!(a.len(), 10);
        assert_ne!(a, sample(&files, 10, 36));
    }
}
