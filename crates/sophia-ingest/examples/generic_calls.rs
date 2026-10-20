//! How many of the crate's `calls_generic` spans are names bound locally in
//! the definition around them (a parameter, a `where` type parameter, a
//! local variable)? Those are not calls to a module's generic function.
use std::collections::BTreeMap;

fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("package root .jl"));
    let d = sophia_ingest::module::discover(&root).unwrap();
    let r = sophia_ingest::module::ingest(&d).unwrap();
    let mut local: BTreeMap<String, usize> = BTreeMap::new();
    let mut free = 0;
    for (f, span, name) in &r.calls_generic {
        let src = &d.files[*f as usize].text;
        let tree = moonkale_content_id::parse("julia", src, &mut moonkale_content_id::Budget::new()).unwrap();
        // The innermost definition around the span, and its local names.
        let mut n = tree.root_node().descendant_for_byte_range(span.0, span.1);
        let mut def = None;
        while let Some(x) = n {
            if moonkale_content_id::is_definition("julia", x) && x.kind() != "module_definition" {
                def = Some(x);
                break;
            }
            n = x.parent();
        }
        let is_local = def.is_some_and(|x| sophia_ingest::code::locals_for_check("julia", x, src).contains(name.as_str()))
            || src[..span.0].rfind("where").is_some_and(|w| {
                // `where {S}` / `where S` on the same definition's signature line
                let line_start = src[..span.0].rfind('\n').map(|i| i + 1).unwrap_or(0);
                w >= line_start && src[w..].split(['\n', '=']).next().is_some_and(|clause| clause.contains(name.as_str()))
            });
        if is_local {
            *local.entry(name.clone()).or_default() += 1;
        } else {
            free += 1;
        }
    }
    let total: usize = local.values().sum();
    println!("calls_generic: {} spans; {free} free names; {total} bound locally:", r.calls_generic.len());
    let mut v: Vec<_> = local.into_iter().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (n, c) in v.iter().take(12) {
        println!("  {c:>5}  {n}");
    }
}
