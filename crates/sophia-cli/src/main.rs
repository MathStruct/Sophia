//! The `sophia` command (Milestones M1). See `main.md`.
//!
//! ```text
//! sophia [--store PATH] [--backend sqlite|redb|lmdb|duckdb] <command>
//!   init                      create an empty store
//!   sample                    load the sample program (SPEC-004 §9)
//!   gen N [SEED]              load N synthetic definitions
//!   ls [PREFIX]               list names
//!   text NAME                 a definition as text, in the reader's profile
//!   edit NAME FILE            replace a definition with the text in FILE
//!   add NAMESPACE FILE        add the definitions in FILE
//!   profile [NAME]            show, or set, the reader's rendering profile
//!   show NAME|ID              a definition's record and core form
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
use sophia_interface::{EditError, Interface, Key};
use sophia_store::{Backend, Store, Target};

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn usage() -> &'static str {
    "usage: sophia [--store PATH] [--backend sqlite|redb|lmdb|duckdb] <init|sample|gen N [SEED]|ls [PREFIX]|text NAME|edit NAME FILE|add NS FILE|profile [NAME]|show NAME|deps NAME|log [SINCE] [LIMIT]|verify|stats|sql QUERY>"
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
        PathBuf::from(".sophia").join(format!("store.{backend}"))
    });
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        let _ = std::fs::create_dir_all(dir);
    }
    let result = match backend.as_str() {
        "sqlite" => sophia_store::sqlite::Sqlite::open(&path)
            .and_then(|b| Interface::open(b, actor()))
            .map_err(Into::into)
            .and_then(|i| run(&i, &cmd, &args[1..], sql)),
        "redb" => sophia_store::redb_store::Redb::open(&path)
            .and_then(|b| Interface::open(b, actor()))
            .map_err(Into::into)
            .and_then(|i| run(&i, &cmd, &args[1..], |_, _| Err("sql needs the sqlite or duckdb backend".into()))),
        "lmdb" => sophia_store::lmdb_store::Lmdb::open(&path)
            .and_then(|b| Interface::open(b, actor()))
            .map_err(Into::into)
            .and_then(|i| run(&i, &cmd, &args[1..], |_, _| Err("sql needs the sqlite or duckdb backend".into()))),
        "duckdb" => sophia_store::duckdb_store::DuckDb::open(&path)
            .and_then(|b| Interface::open(b, actor()))
            .map_err(Into::into)
            .and_then(|i| run(&i, &cmd, &args[1..], sql)),
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

fn sql<B: Backend + sophia_store::query::ReadOnlyQuery>(store: &Store<B>, q: &str) -> AnyResult<()> {
    let t = sophia_store::query::ReadOnlyQuery::query_readonly(store.backend(), q, QueryLimits::default())?;
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

fn edit_error(e: EditError, text: &str) -> Box<dyn std::error::Error> {
    match e {
        EditError::Diagnostics(ds) => ds
            .iter()
            .map(|d| {
                let (l, c) = d.line_col(text);
                format!("{l}:{c}: {}", d.message)
            })
            .collect::<Vec<_>>()
            .join("\n")
            .into(),
        e => e.to_string().into(),
    }
}

fn run<B: Backend + Sync>(
    i: &Interface<B>,
    cmd: &str,
    args: &[String],
    sql: impl Fn(&Store<B>, &str) -> AnyResult<()>,
) -> AnyResult<()> {
    let store = i.store();
    match cmd {
        "text" => {
            let name = args.first().ok_or("text NAME")?;
            print!("{}", i.fetch_text(&Key::name(name))?.0);
        }
        "edit" => {
            let (name, file) = (args.first().ok_or("edit NAME FILE")?, args.get(1).ok_or("edit NAME FILE")?);
            let text = std::fs::read_to_string(file)?;
            let (_, version) = i.fetch_text(&Key::name(name))?;
            let w = i.write_text(&Key::name(name), version, &text).map_err(|e| edit_error(e, &text))?;
            for d in &w.warnings {
                let (l, c) = d.line_col(&text);
                eprintln!("{l}:{c}: warning: {}", d.message);
            }
            for (p, id) in &w.targets {
                println!("{p} -> {}", id.short(12));
            }
            if !w.stale_dependants.is_empty() {
                println!("{} definitions still use the old version", w.stale_dependants.len());
            }
        }
        "add" => {
            let (ns, file) = (args.first().ok_or("add NAMESPACE FILE")?, args.get(1).ok_or("add NAMESPACE FILE")?);
            let text = std::fs::read_to_string(file)?;
            let first = text.split_whitespace().nth(1).unwrap_or("x").split(['(', ':']).next().unwrap_or("x").to_string();
            let (_, w) = i.create_text(&Key::ns(ns), &first, &text).map_err(|e| edit_error(e, &text))?;
            for (p, id) in &w.targets {
                println!("{p} -> {}", id.short(12));
            }
        }
        "profile" => match args.first() {
            None => print!("{}", i.fetch_text(&Key::settings())?.0),
            Some(p) => {
                let (_, v) = i.fetch_text(&Key::settings())?;
                i.write_text(&Key::settings(), v, &format!("based_on = {p}\n")).map_err(|e| edit_error(e, p))?;
                println!("reader {} now uses a profile based on {p}", i.reader());
            }
        },
        "init" => {
            store.set_meta("format", b"sophia-store-v0")?;
            println!("initialised ({} backend)", store.backend().name());
        }
        "sample" => {
            let before = store.read(|tx| tx.record_count())?;
            i.load_sample()?;
            let after = store.read(|tx| tx.record_count())?;
            println!("{} new definitions", after - before);
        }
        "gen" => {
            let n: usize = args.first().ok_or("gen N [SEED]")?.parse()?;
            let seed: u64 = args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(42);
            let program = sophia_bench::generate(seed, n, sophia_bench::Shape::default());
            let t = std::time::Instant::now();
            let records = sophia_bench::ingest(store, &program, 1000);
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
            match lookup(store, what)? {
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
            let id = lookup(store, what)?.digest();
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
        "sql" => sql(store, args.first().ok_or("sql QUERY")?)?,
        other => return Err(format!("unknown command {other:?}\n{}", usage()).into()),
    }
    Ok(())
}
