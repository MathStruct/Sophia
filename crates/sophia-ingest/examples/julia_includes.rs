//! Compare Sophia's include discovery with the files Julia evaluates; see
//! julia_includes.md.
use sophia_ingest::conformance::{IncludeOutcome, PlaceOutcome, include_ledger};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 && args.len() != 4 {
        eprintln!("usage: julia_includes PKG_DIR truth.tsv [--rows]");
        std::process::exit(2);
    }
    let all = args.get(3).is_some_and(|s| s == "--rows");
    let l = include_ledger(Path::new(&args[1]), &std::fs::read_to_string(&args[2])?)?;
    println!(
        "{} files; {} methods placed; {} methods at lines no definition covers; {} Sophia definitions in files Julia does not evaluate; {} malformed",
        l.files.len(),
        l.methods.len(),
        l.methods_outside,
        l.extra_definitions,
        l.malformed
    );
    for (o, n) in l.file_tally() {
        println!("file\t{o:?}\t{n}");
    }
    for (o, n) in l.method_tally() {
        println!("method\t{o:?}\t{n}");
    }
    for r in &l.files {
        if all || r.outcome != IncludeOutcome::Knows {
            println!("{:?}\t{}\tJulia {:?}\tSophia {:?}", r.outcome, r.file, r.julia, r.sophia);
        }
    }
    for r in &l.methods {
        if all || r.outcome != PlaceOutcome::Knows {
            println!("{:?}\t{}:{}\t{}.{}\t{}", r.outcome, r.file, r.line, r.module, r.name, r.detail);
        }
    }
    for f in &l.moved {
        println!("moved\t{f}");
    }
    Ok(())
}
