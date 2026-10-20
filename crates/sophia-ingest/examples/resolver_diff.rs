//! Same-file references the per-file resolver finds and the module-level
//! one does not, classified. Run on a Julia package root.
use std::collections::BTreeMap;

/// Is the span inside a `using`, `import` or `export` statement?
fn in_declaration(src: &str, span: (usize, usize)) -> bool {
    // inside a using/import/export statement?
    let tree = moonkale_content_id::parse("julia", src, &mut moonkale_content_id::Budget::new()).unwrap();
    let mut n = tree.root_node().descendant_for_byte_range(span.0, span.1);
    let mut inside = false;
    while let Some(x) = n {
        if matches!(x.kind(), "export_statement" | "using_statement" | "import_statement") {
            inside = true;
            break;
        }
        n = x.parent();
    }
    inside
}

fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("root .jl"));
    let d = sophia_ingest::module::discover(&root).unwrap();
    let m = sophia_ingest::module::Module::new(&d).unwrap();
    let defs = m.definitions();
    let r = sophia_ingest::module::resolve(&m, &defs, &d.dynamic);
    let mut why: BTreeMap<String, usize> = BTreeMap::new();
    let mut samples: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (f, file) in m.files.iter().enumerate() {
        let per_file = sophia_ingest::code::resolve("julia", &defs[f], &file.text);
        for (i, map) in per_file.iter().enumerate() {
            for (span, t) in map {
                let td = &defs[f][*t].0;
                let module_has = r.refs[f][i].get(span).is_some_and(|k| *k == (f, td.start, td.end));
                if module_has {
                    continue;
                }
                let src = &file.text;
                let before = &src[span.0.saturating_sub(1)..span.0];
                let target = &defs[f][*t].0;
                let reason = if before == "." {
                    "field of x.name (e.g. BLAS.gemm!)".to_string()
                } else if target.qualified {
                    "target is a qualified method (Base.f)".to_string()
                } else if in_declaration(src, *span) {
                    "inside export/using/import".to_string()
                } else if defs.iter().flatten().filter(|(d, _)| d.name == target.name && !d.qualified).count() > 1 {
                    "name defined more than once in the package (several methods across files)".to_string()
                } else if r.refs[f][i].contains_key(span) {
                    "module resolver picked another target".to_string()
                } else {
                    "name imported, defined twice in the module, or in another module".to_string()
                };
                *why.entry(reason.clone()).or_default() += 1;
                let s = samples.entry(reason).or_default();
                if s.len() < 3 {
                    let line_start = src[..span.0].rfind('\n').map(|x| x + 1).unwrap_or(0);
                    let line_end = src[span.0..].find('\n').map(|x| span.0 + x).unwrap_or(src.len());
                    s.push(format!("{}: {}", std::path::Path::new(&file.path).file_name().unwrap().to_string_lossy(), src[line_start..line_end].trim()));
                }
            }
        }
    }
    for (w, n) in &why {
        println!("{n:>5}  {w}");
        for s in &samples[w] {
            println!("         {s}");
        }
    }
}
