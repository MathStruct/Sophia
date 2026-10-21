//! How long Moonkale's Explorer calls take on a large store: `children` of
//! the root and of a namespace, and `all`. Read-only; run it on a copy.
//! See `listing_timing.md`.
//!
//! Usage: cargo run --release -p sophia-interface --example listing_timing -- STORE.sophia NS [RUNS]
use std::time::Instant;

use sophia_interface::{Interface, Key};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (path, ns) = (&a[1], &a[2]);
    let runs: usize = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(5);
    let iface = Interface::open(sophia_store::sqlite::Sqlite::open(path).unwrap(), "moonkale").unwrap();
    let time = |label: &str, f: &dyn Fn() -> usize| {
        let mut best = f64::MAX;
        let mut n = 0;
        for _ in 0..runs {
            let t = Instant::now();
            n = f();
            best = best.min(t.elapsed().as_secs_f64());
        }
        println!("{label:<28} {:>9.1} ms  {n} nodes", 1e3 * best);
    };
    time("children(ns:)", &|| iface.children(&Key::ns("")).unwrap().nodes.len());
    time(&format!("children(ns:{ns})"), &|| iface.children(&Key::ns(ns)).unwrap().nodes.len());
    time("all(100000)", &|| iface.all(100_000).unwrap().nodes.len());
}
