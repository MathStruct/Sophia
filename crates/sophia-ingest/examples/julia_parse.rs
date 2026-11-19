//! Compare source parsers per definition; see julia_parse.md.
use sophia_ingest::conformance::{ParseOutcome, parse_ledger};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 && args.len() != 4 {
        eprintln!("usage: julia_parse PKG_DIR truth.tsv [--rows]");
        std::process::exit(2);
    }
    let ledger = parse_ledger(Path::new(&args[1]), &std::fs::read_to_string(&args[2])?)?;
    println!(
        "{} rows; {} malformed; {} Julia errors; {} undiscovered files",
        ledger.rows.len(),
        ledger.malformed,
        ledger.julia_errors.len(),
        ledger.undiscovered.len()
    );
    for (outcome, count) in ledger.tally() {
        println!("{outcome:?}\t{count}");
    }
    for row in &ledger.rows {
        if row.outcome == ParseOutcome::Wrong || args.get(3).is_some_and(|s| s == "--rows") {
            println!("{:?}\t{}:{}\t{}\t{}\t{}", row.outcome, row.file, row.line, row.name_byte, row.name, row.detail);
        }
    }
    for (file, first, last) in &ledger.julia_errors {
        println!("Julia error\t{file}\t{first}\t{last}");
    }
    for file in &ledger.undiscovered {
        println!("undiscovered\t{file}");
    }
    Ok(())
}
