//! Module-level Julia resolution: a **proposal** for `moonkale-content-id`
//! v5, step 2 of SR-116 ([[SPEC-008 Module-Level Julia Resolution]]).
//! Julia resolves names per module, and a package is one module spread over
//! `include`d files, so same-file resolution misses most of a package's
//! dependency graph. This resolver takes a whole module and returns the same
//! kind of output as the per-file one (byte span → target key), plus, for
//! each definition, the files its id depends on (Moonkale's cache key). It
//! is conservative: anything ambiguous stays unresolved. See `module.md`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use moonkale_content_id::{Budget, ContentIdError, Identity, Part, Syntax, Tree, definition, identities, is_definition, parse};
use sophia_hash::Record;

use crate::code::{CodeError, Def, definitions, local_names, reason, text, to_record};

/// Bounds on one module (constraint 3).
pub const MAX_MODULE_FILES: usize = 1024;
pub const MAX_MODULE_BYTES: usize = 32 * 1024 * 1024;

/// A definition's key: (file index, start byte, end byte) of its node.
/// Spans, not traversal indices, so that two tools that find definitions
/// differently cannot silently disagree (SPEC-008 §4).
pub type Key = (usize, usize, usize);

/// Work bounds of one module request. They are **normative**: a budget
/// failure means no ids, so both tools must use the same numbers.
pub const MODULE_STEPS_BASE: usize = 1_000_000;
pub const MODULE_STEPS_PER_BYTE: usize = 64;

/// Definition kinds a module-level name may resolve to. Not modules (a
/// namespace), macros, or non-`const` globals (reassignable at run time).
pub fn resolvable(kind: &str) -> bool {
    matches!(
        kind,
        "struct_definition" | "abstract_definition" | "primitive_definition" | "const_statement" | "function_definition" | "assignment"
    )
}

/// Julia's `module … end` is a namespace, not a unit (SPEC-008 §5a).
fn is_unit(d: &Def) -> bool {
    d.kind != "module_definition"
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

/// The normaliser recorded on records resolved this way: WP-66 with
/// module-level resolution, which is not yet Moonkale's (until v5).
pub fn normaliser() -> String {
    format!("moonkale-wp66@{}+module-v0", moonkale_content_id::NORMALIZER_VERSION)
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

/// The module path (enclosing `module` names) of a node, within its file.
fn module_path(mut n: Syntax, src: &str) -> Vec<String> {
    let mut out = Vec::new();
    while let Some(p) = n.parent() {
        if p.kind() == "module_definition" {
            if let Some(name) = p.child_by_field_name("name") {
                out.push(text(name, src).to_string());
            }
        }
        n = p;
    }
    out.reverse();
    out
}

/// `include(…)` calls in a tree: (literal path or None, module path).
fn includes(tree: &Tree, src: &str) -> Vec<(Option<String>, Vec<String>)> {
    let mut out = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        if n.kind() == "call_expression"
            && n.named_child(0).is_some_and(|f| f.kind() == "identifier" && text(f, src) == "include")
        {
            let arg = n.named_child(1).and_then(|a| a.named_child(0));
            let literal = arg.filter(|a| a.kind() == "string_literal" && a.named_child_count() == 1).and_then(|a| {
                let c = a.named_child(0)?;
                (c.kind() == "content").then(|| text(c, src).to_string())
            });
            out.push((literal, module_path(n, src)));
            continue;
        }
        let mut c = n.walk();
        let children: Vec<Syntax> = n.children(&mut c).collect();
        stack.extend(children.into_iter().rev());
    }
    out
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
        let incs = includes(&tree, &text);
        d.files.push(SourceFile { path: path.display().to_string(), text: text.clone(), base_module: base.clone() });
        for (lit, local) in incs {
            let mut m = base.clone();
            m.extend(local);
            match lit {
                Some(rel) => visit(&path.parent().unwrap_or(Path::new(".")).join(rel), m, d, bytes, seen)?,
                None => d.dynamic.push((me, m)),
            }
        }
        Ok(())
    }
    visit(root, Vec::new(), &mut d, &mut bytes, &mut seen)?;
    Ok(d)
}

/// Is `n` inside a definition other than a module (a method body)?
fn nested_in_definition(n: Syntax) -> bool {
    let mut p = n.parent();
    while let Some(x) = p {
        if x.kind() != "module_definition" && is_definition("julia", x) {
            return true;
        }
        p = x.parent();
    }
    false
}

/// Identifiers of `using`, `import` and `export` statements, by module path.
fn imported(tree: &Tree, src: &str, base: &[String]) -> Vec<(Vec<String>, String)> {
    let mut out = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        if matches!(n.kind(), "using_statement" | "import_statement") {
            let mut m = base.to_vec();
            m.extend(module_path(n, src));
            let mut inner = vec![n];
            while let Some(x) = inner.pop() {
                if x.kind() == "identifier" {
                    out.push((m.clone(), text(x, src).to_string()));
                }
                let mut c = x.walk();
                inner.extend(x.children(&mut c));
            }
            continue;
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c));
    }
    out
}

/// The result for one module.
pub struct Resolved {
    /// Per file, per definition (in `definitions` order; modules have an
    /// empty map): span → target.
    pub refs: Vec<Vec<HashMap<(usize, usize), Key>>>,
    /// Per definition: the files its id depends on (its own and those of
    /// every definition it reaches through references), for cache keys.
    pub depends_on: HashMap<Key, BTreeSet<usize>>,
    /// Calls to a name with several bindings in the module (a generic
    /// function): (file, span, name). Drawn as Calls edges; not in ids.
    pub calls_generic: Vec<(usize, (usize, usize), String)>,
}

fn key(f: usize, d: &Def) -> Key {
    (f, d.start, d.end)
}

/// One module, parsed.
pub struct Module {
    pub files: Vec<SourceFile>,
    trees: Vec<Tree>,
}

impl Module {
    pub fn new(d: &Discovered) -> Result<Self, ModuleError> {
        let mut trees = Vec::new();
        for f in &d.files {
            trees.push(parse("julia", &f.text, &mut Budget::new())?);
        }
        Ok(Module { files: d.files.clone(), trees })
    }

    /// The definitions of every file, as `code::definitions` finds them.
    pub fn definitions(&self) -> Vec<Vec<(Def, Syntax<'_>)>> {
        self.files.iter().zip(&self.trees).map(|(f, t)| definitions("julia", t.root_node(), &f.text)).collect()
    }
}

/// Resolves references across a module (SPEC-008 §2).
pub fn resolve(m: &Module, defs: &[Vec<(Def, Syntax)>], dynamic: &[(usize, Vec<String>)]) -> Resolved {
    // Module-level bindings: definitions directly in a module's scope.
    let mut scope: HashMap<(Vec<String>, String), Vec<Key>> = HashMap::new();
    let mut functions: HashSet<Key> = HashSet::new();
    let full_module = |f: usize, n: Syntax| {
        let mut p = m.files[f].base_module.clone();
        p.extend(module_path(n, &m.files[f].text));
        p
    };
    for (f, fd) in defs.iter().enumerate() {
        for (d, node) in fd {
            if d.name.is_empty() || d.qualified || nested_in_definition(*node) || !is_unit(d) {
                continue;
            }
            if matches!(d.kind.as_str(), "function_definition" | "assignment") {
                functions.insert(key(f, d));
            }
            scope.entry((full_module(f, *node), d.name.clone())).or_default().push(key(f, d));
        }
    }
    let kind_of: HashMap<Key, String> =
        defs.iter().enumerate().flat_map(|(f, fd)| fd.iter().map(move |(d, _)| (key(f, d), d.kind.clone()))).collect();
    // Names that may come from elsewhere: imported or used explicitly.
    let mut blocked: HashSet<(Vec<String>, String)> = HashSet::new();
    for (f, t) in m.trees.iter().enumerate() {
        blocked.extend(imported(t, &m.files[f].text, &m.files[f].base_module));
    }
    let dynamic_modules: HashSet<&Vec<String>> = dynamic.iter().map(|(_, p)| p).collect();

    let mut refs: Vec<Vec<HashMap<(usize, usize), Key>>> = Vec::new();
    let mut calls_generic = Vec::new();
    for (f, fd) in defs.iter().enumerate() {
        let src = &m.files[f].text;
        let mut file_refs = Vec::new();
        for (d, node) in fd {
            let mut map = HashMap::new();
            if !is_unit(d) {
                file_refs.push(map);
                continue;
            }
            let module = full_module(f, *node);
            let locals = local_names("julia", *node, src, d.name_span);
            let mut stack = vec![*node];
            while let Some(n) = stack.pop() {
                if n.kind() == "identifier" {
                    let span = (n.start_byte(), n.end_byte());
                    let t = text(n, src);
                    let parent = n.parent();
                    // `x.name`: the field is not a module-level name.
                    let field = parent.is_some_and(|p| {
                        p.kind() == "field_expression" && p.child_by_field_name("value").is_none_or(|v| v.id() != n.id())
                    });
                    // `struct W; parent::Int; end`: a field declaration's
                    // name is not a reference (its type is).
                    let struct_field = parent.is_some_and(|p| {
                        let declares = p.kind() == "typed_expression" && p.named_child(0).is_some_and(|c| c.id() == n.id());
                        let bare = p.kind() == "block";
                        (declares && p.parent().is_some_and(|b| b.kind() == "block" && b.parent().is_some_and(|s| s.kind() == "struct_definition")))
                            || (bare && p.parent().is_some_and(|s| s.kind() == "struct_definition"))
                    });
                    let k = (module.clone(), t.to_string());
                    if Some(span) != d.name_span && !field && !struct_field && !locals.contains(t) && !blocked.contains(&k) {
                        match scope.get(&k).map(Vec::as_slice) {
                            Some([target]) if resolvable(&kind_of[target]) => {
                                // A dynamic include in this module could add
                                // methods to a function: functions stay open.
                                if !(dynamic_modules.contains(&module) && functions.contains(target)) {
                                    map.insert(span, *target);
                                }
                            }
                            Some(several) if several.len() > 1 && several.iter().all(|t| functions.contains(t)) => {
                                calls_generic.push((f, span, t.to_string()));
                            }
                            _ => {}
                        }
                    }
                }
                let mut c = n.walk();
                for child in n.children(&mut c) {
                    let skip = matches!(child.kind(), "using_statement" | "import_statement" | "export_statement")
                        || (child.id() != node.id() && is_definition("julia", child));
                    if !skip {
                        stack.push(child);
                    }
                }
            }
            file_refs.push(map);
        }
        refs.push(file_refs);
    }

    // Files each definition's id depends on: transitively through refs.
    let index: HashMap<Key, (usize, usize)> =
        defs.iter().enumerate().flat_map(|(f, fd)| fd.iter().enumerate().map(move |(i, (d, _))| (key(f, d), (f, i)))).collect();
    let mut depends_on: HashMap<Key, BTreeSet<usize>> = HashMap::new();
    for (f, fd) in defs.iter().enumerate() {
        for (d, _) in fd.iter().filter(|(d, _)| is_unit(d)) {
            let mut files = BTreeSet::new();
            let mut seen = HashSet::new();
            let mut stack = vec![key(f, d)];
            while let Some(k) = stack.pop() {
                if !seen.insert(k) {
                    continue;
                }
                files.insert(k.0);
                let (kf, ki) = index[&k];
                stack.extend(refs[kf][ki].values().copied());
            }
            depends_on.insert(key(f, d), files);
        }
    }
    Resolved { refs, depends_on, calls_generic }
}

/// What ingesting a module produced.
pub struct Ingested {
    /// Per file: the identified definitions and their records.
    pub files: Vec<(String, Vec<(Def, Record)>)>,
    pub refused: BTreeMap<String, usize>,
    pub references: usize,
    pub cross_file_references: usize,
    /// Calls to generic functions (several methods): drawn, not in ids.
    pub calls_generic: usize,
    pub depends_on: HashMap<Key, BTreeSet<usize>>,
}

/// Parses, resolves and identifies a whole module. Refused definitions are
/// skipped and references to them dropped, as in `code::file`.
pub fn ingest(d: &Discovered) -> Result<Ingested, ModuleError> {
    let m = Module::new(d)?;
    let defs = m.definitions();
    let mut r = resolve(&m, &defs, &d.dynamic);
    let mut failed: HashMap<Key, String> = HashMap::new();
    let mut parts: HashMap<Key, Vec<Part<Key>>> = HashMap::new();
    loop {
        let before = failed.len();
        for (f, fd) in defs.iter().enumerate() {
            for (i, (def, node)) in fd.iter().enumerate() {
                let k = key(f, def);
                if !is_unit(def) || failed.contains_key(&k) || parts.contains_key(&k) {
                    continue;
                }
                // v4's budget is sized for one file: one per definition
                // here; the crate's v5 uses a module budget (SPEC-008 §3).
                match definition("julia", *node, &m.files[f].text, &r.refs[f][i], &mut Budget::new()) {
                    Ok(p) => {
                        parts.insert(k, p);
                    }
                    Err(ContentIdError::LimitExceeded) => return Err(ContentIdError::LimitExceeded.into()),
                    Err(e) => {
                        failed.insert(k, reason(*node, &e));
                    }
                }
            }
        }
        if failed.len() == before {
            break;
        }
        for (f, fr) in r.refs.iter_mut().enumerate() {
            for (i, map) in fr.iter_mut().enumerate() {
                let n = map.len();
                map.retain(|_, t| !failed.contains_key(t));
                if map.len() != n {
                    parts.remove(&key(f, &defs[f][i].0));
                }
            }
        }
    }
    let ids: HashMap<Key, Identity> = identities(&parts, &mut Budget::new())?;
    let norm = normaliser();
    let mut files = Vec::new();
    for (f, fd) in defs.iter().enumerate() {
        let mut out = Vec::new();
        for (d, _) in fd {
            let k = key(f, d);
            if ids.contains_key(&k) {
                out.push((d.clone(), to_record("julia", d, &parts[&k], &ids, k, &norm)));
            }
        }
        files.push((m.files[f].path.clone(), out));
    }
    let mut refused = BTreeMap::new();
    for why in failed.values() {
        *refused.entry(why.clone()).or_default() += 1;
    }
    let references = r.refs.iter().flatten().map(HashMap::len).sum();
    let cross_file_references =
        r.refs.iter().enumerate().flat_map(|(f, fr)| fr.iter().flat_map(move |m| m.values().filter(move |t| t.0 != f))).count();
    Ok(Ingested { files, refused, references, cross_file_references, calls_generic: r.calls_generic.len(), depends_on: r.depends_on })
}
