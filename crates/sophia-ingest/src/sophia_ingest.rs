//! Frontends that bring real code into Sophia ([[Real Code Corpora]], B-16).
//! Metamath first. See `sophia_ingest.md`.

pub mod code;
pub mod metamath;
pub mod module;

use std::collections::BTreeMap;

use sophia_store::{Annotation, Backend, Store, StoreError, Visibility};

/// What an ingest did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Assertions read (one name each).
    pub assertions: usize,
    /// Proofs read (one name each, `mmproof.<db>.<label>`).
    pub proofs: usize,
    /// Records that were new: fewer than `assertions` when content repeats.
    pub new_records: usize,
    /// Assertions per role (axiom, definition, theorem, syntax).
    pub kinds: BTreeMap<String, usize>,
}

/// The names of a library's collections (SPEC-006 §1, versioned nominal
/// collections): `mmlib.<db>.proofs`, `.axioms`, `.definitions`.
pub fn library_collection(db: &str, which: &str) -> String {
    format!("mmlib.{db}.{which}")
}

/// Stores a parsed Metamath database, in batches of `batch`:
/// - one record per distinct statement, named `mm.<db>.<label>`;
/// - one record per distinct proof, named `mmproof.<db>.<label>`;
/// - one `proof` annotation per proof, on the statement and the proof, so
///   queries can tell "proves" (the annotation) from "uses" (references);
/// - the library itself as three nominal collections: the proofs it uses,
///   the statements it assumes as axioms, and its definitions. Being an
///   axiom, or the chosen proof, is a library's choice, not a property of a
///   statement: a statement can be an axiom in one library and proved in
///   another, or even in another part of the same library.
pub fn store_metamath<B: Backend>(
    store: &Store<B>,
    db: &str,
    assertions: &[metamath::Assertion],
    batch: usize,
) -> Result<Report, StoreError> {
    let mut report = Report { assertions: assertions.len(), ..Report::default() };
    for a in assertions {
        *report.kinds.entry(a.role.clone()).or_default() += 1;
    }
    let collection = |which: &str| -> Result<sophia_hash::Digest, StoreError> {
        let path = library_collection(db, which);
        match store.resolve(&path)? {
            Some(n) => Ok(n.target.digest()),
            None => store.new_nominal(metamath::LANG, "relation", &path),
        }
    };
    let (proofs_n, axioms_n, defs_n) = (collection("proofs")?, collection("axioms")?, collection("definitions")?);
    for chunk in assertions.chunks(batch.max(1)) {
        let mut records = Vec::new();
        let mut names: Vec<(String, usize, Visibility)> = Vec::new();
        for a in chunk {
            names.push((metamath::name_for(db, &a.label), records.len(), Visibility::Public));
            records.push(a.record.clone());
            if let Some(p) = &a.proof {
                names.push((metamath::proof_name_for(db, &a.label), records.len(), Visibility::Public));
                records.push(p.clone());
                report.proofs += 1;
            }
        }
        report.new_records += store.add(&records, &names)?.new_records;
        let anns: Vec<Annotation> = chunk
            .iter()
            .filter_map(|a| a.proof.as_ref().map(|p| Annotation::new("proof", vec![a.record.id, p.id], Vec::new(), None)))
            .collect();
        if !anns.is_empty() {
            store.annotate_many(&anns)?;
        }
        let pick = |role: &str| -> Vec<sophia_hash::Digest> { chunk.iter().filter(|a| a.role == role).map(|a| a.record.id).collect() };
        store.add_members(&proofs_n, &chunk.iter().filter_map(|a| a.proof.as_ref().map(|p| p.id)).collect::<Vec<_>>())?;
        store.add_members(&axioms_n, &pick("axiom"))?;
        store.add_members(&defs_n, &pick("definition"))?;
    }
    Ok(report)
}

/// Stores identified definitions: records, names for non-functions, and
/// functions grouped into generic functions. A generic function is keyed by
/// **(module path, name)**, never by name alone: `A.f` and `B.f` are two
/// functions, and a method written `Base.show(…) = …` (or `show(…) = …`
/// after `import Base: show`) is a method of `Base.show`, whose entity is
/// `jl.Base.show`. Outside Julia, and for Julia without modules, the module
/// is `pkg`. `uses_generic` maps a record id to the generic functions it
/// calls (module path plus name): they become the record's `nominal_refs`
/// (queryable as `uses(d, g)`), which do not affect its id.
fn store_items<B: Backend>(
    store: &Store<B>,
    lang: &str,
    pkg: &str,
    items: &[(code::Def, sophia_hash::Record)],
    uses_generic: &std::collections::HashMap<sophia_hash::Digest, Vec<Vec<String>>>,
    report: &mut CodeReport,
) -> Result<(), StoreError> {
    let prefix = match lang {
        "julia" => "jl",
        "rust" => "rs",
        _ => "py",
    };
    let mut methods: BTreeMap<Vec<String>, Vec<sophia_hash::Digest>> = BTreeMap::new();
    let mut named: BTreeMap<Vec<String>, sophia_hash::Digest> = BTreeMap::new();
    for (d, r) in items {
        match binding(d, pkg) {
            Some(Binding::Method(k)) => methods.entry(k).or_default().push(r.id),
            Some(Binding::Named(k)) => {
                named.entry(k).or_insert(r.id);
            }
            None => {}
        }
    }
    // Generic functions first, so records can refer to them.
    let mut generics: BTreeMap<Vec<String>, sophia_hash::Digest> = BTreeMap::new();
    for k in methods.keys() {
        let mut path = entity_path(prefix, k);
        if named.contains_key(k) {
            path.push_str(".new");
        }
        let generic = match store.resolve(&path)? {
            Some(e) => e.target.digest(),
            None => store.new_nominal(lang, "generic", &path)?,
        };
        generics.insert(k.clone(), generic);
    }
    let records: Vec<sophia_hash::Record> = items
        .iter()
        .map(|(_, r)| {
            let mut r = r.clone();
            if let Some(keys) = uses_generic.get(&r.id) {
                for k in keys {
                    if let Some(g) = generics.get(k) {
                        if !r.nominal_refs.contains(g) {
                            r.nominal_refs.push(*g);
                        }
                    }
                }
            }
            r
        })
        .collect();
    report.new_records += store.put_records(&records)?;
    let mut names: Vec<(String, usize, Visibility)> = Vec::new();
    let mut named_records = Vec::new();
    for (k, id) in &named {
        let path = entity_path(prefix, k);
        if store.resolve(&path)?.is_none() {
            if let Some(r) = store.record(id)? {
                names.push((path, named_records.len(), Visibility::Public));
                named_records.push(r);
            }
        }
    }
    store.add(&named_records, &names)?;
    // The package's own top-level modules; a function under any other is
    // another package's (or Base's) that this one extends.
    let roots: std::collections::HashSet<String> = items.iter().map(|(d, _)| home(d, pkg)[0].clone()).collect();
    for (k, ms) in &methods {
        let mut ms = ms.clone();
        ms.sort();
        ms.dedup();
        report.methods += ms.len();
        report.generics += 1;
        if !roots.contains(&k[0]) {
            report.extended += 1;
        }
        store.add_members(&generics[k], &ms)?;
    }
    Ok(())
}

/// What a definition binds, as Sophia stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    /// A method of the generic function (module path, name).
    Method(Vec<String>),
    /// A directly named definition (type, constant, macro, module item).
    Named(Vec<String>),
}

fn home(d: &code::Def, pkg: &str) -> Vec<String> {
    if d.module.is_empty() { vec![pkg.to_string()] } else { d.module.clone() }
}

/// The one grouping rule ([[Julia Name Resolution]]): a function is a
/// method of `extends` if set, else of (its module, its name); other
/// definitions are named (module, name). A qualified method that could not
/// be placed (`Base.$f`), and anything unnamed, binds nothing.
pub fn binding(d: &code::Def, pkg: &str) -> Option<Binding> {
    if d.name.is_empty() {
        return None;
    }
    let function = matches!(d.kind.as_str(), "function_definition" | "assignment" | "function_item" | "function_signature_item");
    if function {
        if d.qualified && d.extends.is_none() {
            return None;
        }
        Some(Binding::Method(d.extends.clone().unwrap_or_else(|| {
            let mut k = home(d, pkg);
            k.push(d.name.clone());
            k
        })))
    } else if d.qualified {
        None
    } else {
        let mut k = home(d, pkg);
        k.push(d.name.clone());
        Some(Binding::Named(k))
    }
}

/// `jl` + `[LinearAlgebra, BLAS, gemm!]` → `jl.LinearAlgebra.BLAS.gemm!`.
pub fn entity_path(prefix: &str, key: &[String]) -> String {
    let mut s = prefix.to_string();
    for k in key {
        s.push('.');
        s.push_str(&segment(k));
    }
    s
}

/// Stores a whole Julia module through the crate's module-level resolver
/// (normaliser v5, SPEC-008): ids are Moonkale's; calls to generic
/// functions become nominal uses. `report.references` counts all resolved
/// references; `cross_file` those between files.
pub fn store_module<B: Backend>(
    store: &Store<B>,
    pkg: &str,
    root: &std::path::Path,
) -> Result<(CodeReport, usize, usize), Box<dyn std::error::Error>> {
    let d = module::discover(root)?;
    let r = module::ingest(&d)?;
    let calls = r.calls_generic.len();
    let mut report = CodeReport { files: d.files.len(), refused_definitions: r.refused.clone(), references: r.references, ..CodeReport::default() };
    let items: Vec<(code::Def, sophia_hash::Record)> = r.files.into_iter().flat_map(|(_, v)| v).collect();
    report.definitions = items.len();
    store_items(store, "julia", pkg, &items, &r.uses_generic, &mut report)?;
    Ok((report, r.cross_file_references, calls))
}

/// What a code ingest did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeReport {
    pub files: usize,
    /// Files that could not be parsed at all (too large, over budget).
    pub refused: Vec<(String, String)>,
    /// Definitions the normal form refused, by reason (the first refused
    /// construct inside them).
    pub refused_definitions: BTreeMap<String, usize>,
    pub definitions: usize,
    pub new_records: usize,
    /// Generic functions (one (module, name), its methods as members).
    pub generics: usize,
    /// Of those, functions owned by another top-level module that this
    /// package adds methods to (`Base.show`, `Base.:+`).
    pub extended: usize,
    pub methods: usize,
    /// Resolved same-file references.
    pub references: usize,
}

fn segment(name: &str) -> String {
    let plain = name.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '!');
    if plain { name.to_string() } else { format!("`{}`", name.replace('`', "")) }
}

/// Stores the sources of one package (Julia, Rust or Python) under `pkg`:
/// - every definition as a record with Moonkale's WP-66 id (SR-116);
/// - functions grouped by name into **generic functions** (nominal
///   entities named `<prefix>.<pkg>.<name>`, methods as members), as Julia's
///   multiple dispatch has it (SP-006);
/// - other definitions (types, constants, modules, macros) named directly.
///
/// A function named like a type (a Julia constructor) goes to
/// `<name>.new`. References are resolved within each file, as Moonkale
/// does, so cross-file calls are not yet edges.
pub fn store_code<B: Backend>(
    store: &Store<B>,
    lang: &str,
    pkg: &str,
    files: &[(String, String)],
) -> Result<CodeReport, StoreError> {

    let mut report = CodeReport { files: files.len(), ..CodeReport::default() };
    let mut items: Vec<(code::Def, sophia_hash::Record)> = Vec::new();
    for (path, src) in files {
        let parsed = match code::file(lang, src) {
            Ok(f) => f,
            Err(e) => {
                report.refused.push((path.clone(), e.to_string()));
                continue;
            }
        };
        report.definitions += parsed.records.len();
        report.references += parsed.records.iter().map(|r| r.refs.len()).sum::<usize>();
        for (_, why) in &parsed.refused {
            *report.refused_definitions.entry(why.clone()).or_default() += 1;
        }
        items.extend(parsed.defs.into_iter().zip(parsed.records));
    }
    store_items(store, lang, pkg, &items, &Default::default(), &mut report)?;
    Ok(report)
}
