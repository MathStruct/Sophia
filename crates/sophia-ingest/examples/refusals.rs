//! Samples of definitions the WP-66 normal form refuses, by reason.
fn main() {
    let mut args = std::env::args().skip(1);
    let lang = args.next().expect("lang");
    let want = args.next().expect("reason");
    let mut shown = 0;
    for path in args {
        let src = std::fs::read_to_string(&path).unwrap();
        let Ok(f) = sophia_ingest::code::file(&lang, &src) else { continue };
        for (d, why) in f.refused {
            if why == want && shown < 8 {
                shown += 1;
                let t: String = src[d.start..d.end].chars().take(160).collect();
                println!("--- {path}:{}\n{t}\n", d.start);
            }
        }
    }
}
