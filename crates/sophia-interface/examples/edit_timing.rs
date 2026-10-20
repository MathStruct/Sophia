//! B-15 from Sophia's side: the four steps of Moonkale's edit loop
//! (`moonkale-sources-sophia` example `edit_loop`) called on the Interface
//! directly, without Moonkale's adapter or tokio, to see which part of a
//! cold first edit is Sophia's. See `edit_timing.md`.
//!
//! Usage: cargo run --release -p sophia-interface --example edit_timing -- STORE.sophia NAME EDITED.txt [RUNS]
use std::time::Instant;

use sophia_interface::{Direction, Interface, Key};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (path, name, edited) = (&a[1], &a[2], std::fs::read_to_string(&a[3]).unwrap());
    let runs: usize = a.get(4).map(|s| s.parse().unwrap()).unwrap_or(20);
    let t = Instant::now();
    let iface = Interface::open(sophia_store::sqlite::Sqlite::open(path).unwrap(), "moonkale").unwrap();
    eprintln!("open\t{} µs", t.elapsed().as_micros());
    let key = Key::name(name);
    let (original, _) = iface.fetch_text(&key).unwrap();
    println!("run\tfetch_us\twrite_us\tnew_id_us\tdependants_us\tdependants");
    for run in 0..runs {
        let text = if run % 2 == 0 { &edited } else { &original };
        let old = iface.store().resolve(name).unwrap().unwrap().target.digest();
        let t0 = Instant::now();
        let (_, version) = iface.fetch_text(&key).unwrap();
        let t1 = Instant::now();
        iface.write_text(&key, version, text).unwrap();
        let t2 = Instant::now();
        let new = iface.store().resolve(name).unwrap().unwrap().target.digest();
        let t3 = Instant::now();
        let g = iface.neighbours(&Key::Content(old), 1, Direction::In, usize::MAX).unwrap();
        let t4 = Instant::now();
        assert_ne!(new, old);
        let deps = g.edges.iter().filter(|e| e.to == Key::Content(old)).count();
        println!(
            "{run}\t{}\t{}\t{}\t{}\t{deps}",
            (t1 - t0).as_micros(),
            (t2 - t1).as_micros(),
            (t3 - t2).as_micros(),
            (t4 - t3).as_micros()
        );
    }
}
