//! Julia modules through `moonkale-content-id`'s module-level resolver
//! (normaliser v5; [[SPEC-008 Module-Level Julia Resolution]], step 2 of
//! SR-116). Julia resolves names per module, and a package is one module
//! spread over `include`d files. The crate resolves and identifies a whole
//! module; this file reads the files (the crate does no IO), finds the
//! package root, and turns the crate's definitions into Sophia records. The
//! prototype this replaced was ported into the crate from Sophia `eb549c6`.
//! See `module.md`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use moonkale_content_id::julia::{self, DefKey, ModuleFile};
use moonkale_content_id::{Budget, ContentIdError, Identity, Syntax, Tree, definition, is_definition, parse};
use sophia_hash::Record;

use crate::code::{CodeError, Def, definitions, local_names, text, to_record};

pub use moonkale_content_id::julia::{MAX_MODULE_BYTES, MAX_MODULE_FILES, MODULE_STEPS_BASE, MODULE_STEPS_PER_BYTE};

/// The normaliser recorded on module-resolved Julia records: Moonkale's,
/// so the ids are the ones Moonkale publishes.
pub fn normaliser() -> String {
    format!("moonkale-wp66@{}", moonkale_content_id::NORMALIZER_VERSION)
}

/// The caller's job (SPEC-008 §4): a package's root is `src/<name>.jl`,
/// where `<name>` is `Project.toml`'s `name`. Returns (name, root).
pub fn package_root(dir: &Path) -> Option<(String, PathBuf)> {
    let toml = std::fs::read_to_string(dir.join("Project.toml")).ok()?;
    let name = toml.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == "name").then(|| v.trim().trim_matches('"').to_string())
    })?;
    let root = dir.join("src").join(format!("{name}.jl"));
    root.exists().then_some((name, root))
}

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("{0}")]
    Code(#[from] CodeError),
    #[error("{0}")]
    ContentId(#[from] ContentIdError),
    #[error("io: {0}")]
    Io(String),
    #[error("module too large: {0}")]
    Limit(String),
}

/// One source file of a module, with the module path it was included at
/// (empty for the root file).
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: String,
    pub text: String,
    pub base_module: Vec<String>,
}

/// Files in include order, from a root file. Only `include("literal")`
/// is followed; other include calls are **dynamic** and recorded, because
/// they may add definitions the resolver cannot see.
#[derive(Debug, Clone, Default)]
pub struct Discovered {
    pub files: Vec<SourceFile>,
    /// (file index, module path) of each dynamic include.
    pub dynamic: Vec<(usize, Vec<String>)>,
}

/// Reads a module from its root file, following literal includes in order
/// (depth first, as Julia evaluates them).
pub fn discover(root: &Path) -> Result<Discovered, ModuleError> {
    let mut d = Discovered::default();
    let mut bytes = 0usize;
    let mut seen: HashSet<PathBuf> = HashSet::new();
    fn visit(
        path: &Path,
        base: Vec<String>,
        d: &mut Discovered,
        bytes: &mut usize,
        seen: &mut HashSet<PathBuf>,
    ) -> Result<(), ModuleError> {
        let canon = path.canonicalize().map_err(|e| ModuleError::Io(format!("{}: {e}", path.display())))?;
        if !seen.insert(canon) {
            return Ok(());
        }
        if d.files.len() >= MAX_MODULE_FILES {
            return Err(ModuleError::Limit(format!("more than {MAX_MODULE_FILES} files")));
        }
        let text = std::fs::read_to_string(path).map_err(|e| ModuleError::Io(format!("{}: {e}", path.display())))?;
        *bytes += text.len();
        if *bytes > MAX_MODULE_BYTES {
            return Err(ModuleError::Limit(format!("more than {MAX_MODULE_BYTES} bytes")));
        }
        let tree = parse("julia", &text, &mut Budget::new())?;
        let me = d.files.len();
        let incs = julia::includes(&text, &tree);
        d.files.push(SourceFile { path: path.display().to_string(), text: text.clone(), base_module: base.clone() });
        for inc in incs {
            let mut m = base.clone();
            m.extend(inc.module);
            match inc.path {
                Some(rel) => visit(&path.parent().unwrap_or(Path::new(".")).join(rel), m, d, bytes, seen)?,
                None => d.dynamic.push((me, m)),
            }
        }
        Ok(())
    }
    visit(root, Vec::new(), &mut d, &mut bytes, &mut seen)?;
    Ok(d)
}

/// What ingesting a module produced.
pub struct Ingested {
    /// Per file: the identified definitions and their records.
    pub files: Vec<(String, Vec<(Def, Record)>)>,
    /// Refused definitions, by the crate's error (`DependencyRefused` for a
    /// definition that refers to a refused one).
    pub refused: BTreeMap<String, usize>,
    pub references: usize,
    pub cross_file_references: usize,
    /// Calls to generic functions (several methods): drawn, not in ids.
    pub calls_generic: Vec<(u32, (usize, usize), String)>,
    /// The crate's result, for checks.
    pub module: julia::JuliaModule,
    /// For each identified definition (by record id), the names of the
    /// generic functions it uses (from `calls_generic`).
    pub uses_generic: HashMap<sophia_hash::Digest, Vec<String>>,
}

/// The definition node with exactly this span.
fn node_at(tree: &Tree, start: usize, end: usize) -> Option<Syntax<'_>> {
    let mut n = tree.root_node().descendant_for_byte_range(start, end);
    while let Some(x) = n {
        if x.start_byte() == start && x.end_byte() == end && is_definition("julia", x) {
            return Some(x);
        }
        n = x.parent();
    }
    None
}

/// Type parameters a definition binds in `where` clauses (`where {S, T<:X}`).
fn where_params(def: Syntax, src: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut stack = vec![def];
    while let Some(n) = stack.pop() {
        if n.kind() == "where_expression" {
            if let Some(params) = n.named_child(1) {
                let items: Vec<Syntax> = if params.kind() == "curly_expression" {
                    let mut c = params.walk();
                    params.named_children(&mut c).collect()
                } else {
                    vec![params]
                };
                for p in items {
                    let id = if p.kind() == "binary_expression" { p.named_child(0) } else { Some(p) };
                    if let Some(id) = id.filter(|x| x.kind() == "identifier") {
                        out.insert(text(id, src).to_string());
                    }
                }
            }
        }
        let mut c = n.walk();
        for child in n.children(&mut c) {
            if child.id() == def.id() || !is_definition("julia", child) {
                stack.push(child);
            }
        }
    }
    out
}

/// Resolves and identifies a whole module with the crate, and builds
/// records. A record's bytes are the canonical parts, re-derived from the
/// crate's references (the same input gives the same encoding).
pub fn ingest(d: &Discovered) -> Result<Ingested, ModuleError> {
    let mut trees = Vec::new();
    for f in &d.files {
        trees.push(parse("julia", &f.text, &mut Budget::new())?);
    }
    let files: Vec<ModuleFile> = d
        .files
        .iter()
        .zip(&trees)
        .map(|(f, t)| ModuleFile { text: &f.text, tree: t, module: f.base_module.clone() })
        .collect();
    let dynamic: Vec<Vec<String>> = d.dynamic.iter().map(|(_, m)| m.clone()).collect();
    let jm = julia::module(&files, &dynamic)?;
    let ids: HashMap<DefKey, Identity> =
        jm.definitions.iter().filter_map(|x| x.identity.as_ref().ok().map(|i| (x.key, i.clone()))).collect();
    let mut refused = BTreeMap::new();
    for x in &jm.definitions {
        if let Err(e) = &x.identity {
            *refused.entry(format!("{e:?}")).or_default() += 1;
        }
    }
    // Sophia's view of each definition (kind, name), by span.
    let mine: Vec<HashMap<(usize, usize), Def>> = d
        .files
        .iter()
        .zip(&trees)
        .map(|(f, t)| definitions("julia", t.root_node(), &f.text).into_iter().map(|(x, _)| ((x.start, x.end), x)).collect())
        .collect();
    let norm = normaliser();
    let mut out: Vec<(String, Vec<(Def, Record)>)> = d.files.iter().map(|f| (f.path.clone(), Vec::new())).collect();
    let mut budget = Budget::with_steps(MODULE_STEPS_BASE + MODULE_STEPS_PER_BYTE * d.files.iter().map(|f| f.text.len()).sum::<usize>());
    for x in &jm.definitions {
        if !ids.contains_key(&x.key) {
            continue;
        }
        let (f, start, end) = (x.key.0 as usize, x.key.1, x.key.2);
        let node = node_at(&trees[f], start, end).ok_or_else(|| ModuleError::Io(format!("no definition at {start}:{end}")))?;
        let refs: HashMap<(usize, usize), DefKey> = x.references.iter().copied().collect();
        let parts = definition("julia", node, &d.files[f].text, &refs, &mut budget)?;
        let def = mine[f].get(&(start, end)).cloned().unwrap_or(Def {
            name: x.name.clone().unwrap_or_default(),
            kind: node.kind().to_string(),
            start,
            end,
            name_span: None,
            qualified: x.name.is_none(),
        });
        out[f].1.push((def.clone(), to_record("julia", &def, &parts, &ids, x.key, &norm)));
    }
    // Which definition each generic call is in: the innermost one around
    // its span. A name the definition binds itself (a parameter, a local, a
    // `where` type parameter) is not a call to the module's generic
    // function: the crate's `calls_generic` (v5) still lists those, and
    // they are skipped here (reported to Moonkale).
    let mut uses_generic: HashMap<sophia_hash::Digest, Vec<String>> = HashMap::new();
    let mut locals_cache: HashMap<DefKey, HashSet<String>> = HashMap::new();
    for (f, span, name) in &jm.calls_generic {
        let around = jm
            .definitions
            .iter()
            .filter(|x| x.key.0 == *f && x.key.1 <= span.0 && span.1 <= x.key.2)
            .min_by_key(|x| x.key.2 - x.key.1);
        let Some(around) = around else { continue };
        let locals = locals_cache.entry(around.key).or_insert_with(|| {
            let fi = around.key.0 as usize;
            match node_at(&trees[fi], around.key.1, around.key.2) {
                Some(n) => {
                    let mut l = local_names("julia", n, &d.files[fi].text, None);
                    l.extend(where_params(n, &d.files[fi].text));
                    l
                }
                None => HashSet::new(),
            }
        });
        if locals.contains(name) {
            continue;
        }
        if let Some(id) = ids.get(&around.key) {
            let v = uses_generic.entry(sophia_hash::Digest(id.digest)).or_default();
            if !v.contains(name) {
                v.push(name.clone());
            }
        }
    }
    let references = jm.definitions.iter().map(|x| x.references.len()).sum();
    let cross_file_references =
        jm.definitions.iter().flat_map(|x| x.references.iter().filter(move |(_, t)| t.0 != x.key.0)).count();
    Ok(Ingested {
        files: out,
        refused,
        references,
        cross_file_references,
        calls_generic: jm.calls_generic.clone(),
        module: jm,
        uses_generic,
    })
}
