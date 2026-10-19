//! The `sophia` command (Milestones M1). See `main.md`.
//!
//! ```text
//! sophia [--store PATH] [--backend sqlite|redb] <command>
//!   init                      create an empty store
//!   sample                    load the sample program (SPEC-004 §9)
//!   gen N [SEED]              load N synthetic definitions
//!   ls [PREFIX]               list names
//!   show NAME|ID              show a definition (core form, until a printer exists)
//!   deps NAME|ID              direct dependencies and dependants
//!   log [SINCE] [LIMIT]       the change log
//!   verify                    rehash every definition
//!   stats                     counts and size on disk
//!   sql QUERY                 a read-only query over the views (SQLite only)
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use sophia_hash::Digest;
use sophia_store::query::{QueryLimits, Value};
use sophia_store::{Backend, Store, Target, Visibility};

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn usage() -> &'static str {
    "usage: sophia [--store PATH] [--backend sqlite|redb] <init|sample|gen N [SEED]|ls [PREFIX]|show NAME|deps NAME|log [SINCE] [LIMIT]|verify|stats|sql QUERY>"
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut store_path: Option<PathBuf> = None;
    let mut backend = "sqlite".to_string();
    while let Some(a) = args.first().cloned() {
        match a.as_str() {
            "--store" if args.len() > 1 => {
                store_path = Some(args[1].clone().into());
                args.drain(..2);
            }
            "--backend" if args.len() > 1 => {
                backend = args[1].clone();
                args.drain(..2);
            }
            "-h" | "--help" => {
                println!("{}", usage());
                return ExitCode::SUCCESS;
            }
            _ => break,
        }
    }
    let Some(cmd) = args.first().cloned() else {
        eprintln!("{}", usage());
        return ExitCode::from(2);
    };
    let path = store_path.unwrap_or_else(|| {
        PathBuf::from(".sophia").join(if backend == "redb" { "store.redb" } else { "store.sqlite" })
    });
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        let _ = std::fs::create_dir_all(dir);
    }
    let result = match backend.as_str() {
        "sqlite" => sophia_store::sqlite::Sqlite::open(&path)
            .map_err(Into::into)
            .and_then(|b| run(Store::new(b).with_actor(actor()), &cmd, &args[1..], sql)),
        "redb" => sophia_store::redb_store::Redb::open(&path)
            .map_err(Into::into)
            .and_then(|b| run(Store::new(b).with_actor(actor()), &cmd, &args[1..], |_, _| Err("sql needs the sqlite backend".into()))),
        other => Err(format!("unknown backend {other:?}").into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sophia: {e}");
            ExitCode::FAILURE
        }
    }
}

fn actor() -> String {
    std::env::var("USER").map(|u| format!("cli:{u}")).unwrap_or_else(|_| "cli".into())
}

fn sql(store: &Store<sophia_store::sqlite::Sqlite>, q: &str) -> AnyResult<()> {
    let t = store.backend().query_readonly(q, QueryLimits::default())?;
    println!("{}", t.columns.join("\t"));
    for row in &t.rows {
        let cells: Vec<String> = row
            .iter()
            .map(|v| match v {
                Value::Null => "NULL".into(),
                Value::Int(i) => i.to_string(),
                Value::Real(r) => r.to_string(),
                Value::Text(s) => s.clone(),
                Value::Blob(b) => format!("x'{}'", b.iter().map(|x| format!("{x:02x}")).collect::<String>()),
            })
            .collect();
        println!("{}", cells.join("\t"));
    }
    if t.truncated {
        eprintln!("(truncated at {} rows)", t.rows.len());
    }
    Ok(())
}

fn lookup<B: Backend>(store: &Store<B>, what: &str) -> AnyResult<Target> {
    if let Some(n) = store.resolve(what)? {
        return Ok(n.target);
    }
    if let Ok(d) = what.parse::<Digest>() {
        return Ok(Target::Content(d));
    }
    // A short hex prefix: search names' targets and recent definitions.
    let hits: Vec<Digest> = store.read(|tx| {
        Ok(tx.records_of_kind(None, usize::MAX)?.into_iter().filter(|d| d.to_hex().starts_with(what)).collect())
    })?;
    match hits.as_slice() {
        [d] => Ok(Target::Content(*d)),
        [] => Err(format!("no name or id {what:?}").into()),
        _ => Err(format!("{what:?} is ambiguous ({} matches)", hits.len()).into()),
    }
}

fn run<B: Backend>(
    store: Store<B>,
    cmd: &str,
    args: &[String],
    sql: impl Fn(&Store<B>, &str) -> AnyResult<()>,
) -> AnyResult<()> {
    match cmd {
        "init" => {
            store.set_meta("format", b"sophia-store-v0")?;
            println!("initialised ({} backend)", store.backend().name());
        }
        "sample" => {
            let (names, defs): (Vec<_>, Vec<_>) = sophialang_core::samples::program().into_iter().unzip();
            let records = sophialang_core::normalise_batch(&defs, &mut Default::default())?;
            let named: Vec<(String, usize, Visibility)> = names
                .iter()
                .enumerate()
                .filter(|(_, n)| store.resolve(n).ok().flatten().is_none())
                .map(|(i, n)| (n.to_string(), i, Visibility::Public))
                .collect();
            let rep = store.add(&records, &named)?;
            for (name, kind) in [("Int64", "primitive"), ("Bool", "primitive"), ("Float64", "primitive"), ("Ptr", "primitive"), ("+", "generic"), ("==", "generic")] {
                store.ensure_nominal(sophialang_core::samples::builtin(name), sophialang_core::LANG, kind, name)?;
            }
            println!("{} new definitions, {} names", rep.new_records, rep.name_seqs.len());
        }
        "gen" => {
            let n: usize = args.first().ok_or("gen N [SEED]")?.parse()?;
            let seed: u64 = args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(42);
            let program = sophia_bench::generate(seed, n, sophia_bench::Shape::default());
            let t = std::time::Instant::now();
            let records = sophia_bench::ingest(&store, &program, 1000);
            println!("{} definitions in {:.2} s", records.len(), t.elapsed().as_secs_f64());
        }
        "ls" => {
            let prefix = args.first().map(String::as_str).unwrap_or("");
            for n in store.read(|tx| tx.names_under(prefix, usize::MAX))? {
                let (k, d) = match n.target {
                    Target::Content(d) => ("", d),
                    Target::Nominal(d) => (" (nominal)", d),
                };
                println!("{:<40} {}{}", n.path, d.short(12), k);
            }
        }
        "show" => {
            let what = args.first().ok_or("show NAME|ID")?;
            match lookup(&store, what)? {
                Target::Nominal(id) => {
                    let n = store.nominal(&id)?.ok_or("unknown nominal entity")?;
                    println!("nominal {} {:?} ({}), relation version {}", n.kind, n.label, id.short(12), n.version);
                    for m in store.methods(&id)? {
                        println!("  method {}", m.short(12));
                    }
                }
                Target::Content(id) => {
                    let r = store.record(&id)?.ok_or("not stored")?;
                    println!("id         {}", r.id);
                    println!("lang       {} ({})", r.lang, r.normaliser);
                    println!("kind       {}   status {}", r.kind, r.status.as_str());
                    println!("component  {} [{} of {}]", r.component.short(12), r.index, r.component_size);
                    println!("bytes      {}", r.bytes.len());
                    if r.lang == sophialang_core::LANG {
                        let d = sophialang_core::definition_of(&r)?;
                        println!("core       {d:#?}");
                    }
                }
            }
        }
        "deps" => {
            let what = args.first().ok_or("deps NAME|ID")?;
            let id = lookup(&store, what)?.digest();
            let (r, dependants) = store.read(|tx| Ok((tx.record(&id)?, tx.dependants(&id)?)))?;
            if let Some(r) = r {
                for d in r.refs {
                    println!("uses     {}", d.short(12));
                }
                for d in r.nominal_refs {
                    println!("dispatch {}", d.short(12));
                }
            }
            for d in dependants {
                println!("used by  {}", d.short(12));
            }
        }
        "log" => {
            let since: u64 = args.first().map(|s| s.parse()).transpose()?.unwrap_or(0);
            let limit: usize = args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(50);
            for e in store.changes_since(since, limit)?.entries {
                let d = |x: Option<Digest>| x.map(|d| d.short(10)).unwrap_or_else(|| "-".into());
                println!("{:>6} {:<15} {:<32} {} -> {}  {}", e.seq, e.op.as_str(), e.subject, d(e.old), d(e.new), e.actor);
            }
        }
        "verify" => {
            let ids = store.read(|tx| tx.records_of_kind(None, usize::MAX))?;
            let bad: Vec<_> = ids.iter().filter(|d| !store.verify(d).unwrap_or(false)).collect();
            println!("{} definitions, {} failed verification", ids.len(), bad.len());
            for d in &bad {
                println!("  {}", d);
            }
            if !bad.is_empty() {
                return Err("verification failed".into());
            }
        }
        "stats" => {
            let (n, names, seq) =
                store.read(|tx| Ok((tx.record_count()?, tx.names_under("", usize::MAX)?.len(), tx.last_seq()?)))?;
            println!("backend      {}", store.backend().name());
            println!("definitions  {n}");
            println!("names        {names}");
            println!("log entries  {seq}");
            if let Some(b) = store.backend().size_on_disk() {
                println!("on disk      {:.1} MiB", b as f64 / 1048576.0);
            }
        }
        "sql" => sql(&store, args.first().ok_or("sql QUERY")?)?,
        other => return Err(format!("unknown command {other:?}\n{}", usage()).into()),
    }
    Ok(())
}
