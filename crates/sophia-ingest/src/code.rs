//! Julia, Rust and Python through Moonkale's WP-66 normal form
//! ([[Real Code Corpora]] rung 2; SR-116). The normal form and hashing come
//! from `moonkale-content-id`, so a definition gets the **same id** as in
//! Moonkale, provided references resolve the same way. This file finds the
//! definitions, resolves references (same file, by spelling, skipping local
//! bindings), and turns identities into Sophia records. See `code.md`.

use std::collections::{HashMap, HashSet};

use moonkale_content_id::{Budget, Identity, Part, Syntax, definition, identities, is_definition, parse};
use sophia_hash::{Digest, Record, Status};

/// The normaliser recorded on each record: Moonkale's WP-66, by version.
pub fn normaliser() -> String {
    format!("moonkale-wp66@{}", moonkale_content_id::NORMALIZER_VERSION)
}

#[derive(Debug, thiserror::Error)]
pub enum CodeError {
    #[error("{0}")]
    ContentId(#[from] moonkale_content_id::ContentIdError),
}

/// One definition found in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Def {
    pub name: String,
    /// The syntax node kind (`function_definition`, `struct_definition`, …).
    pub kind: String,
    pub start: usize,
    pub end: usize,
    /// The byte span of the definition's own name.
    pub name_span: Option<(usize, usize)>,
    /// The name is qualified (`Base.show(…) = …`): the definition adds a
    /// method to another module's function and binds nothing here.
    pub qualified: bool,
}

pub(crate) fn text<'a>(n: Syntax, src: &'a str) -> &'a str {
    &src[n.start_byte()..n.end_byte()]
}

/// The node naming a definition: Julia's head (the first identifier down
/// the first-child chain), otherwise the `name` field.
fn name_node<'t>(lang: &str, n: Syntax<'t>) -> Option<Syntax<'t>> {
    if lang == "julia" {
        let mut head = n.named_child(0);
        while let Some(h) = head {
            if matches!(h.kind(), "identifier" | "operator" | "macro_identifier") {
                return Some(h);
            }
            if h.kind() == "field_expression" {
                // `Base.show(…)`: the method's name is the last field.
                return h.named_child(h.named_child_count().saturating_sub(1) as u32).or(Some(h));
            }
            head = h.named_child(0);
        }
        None
    } else {
        n.child_by_field_name("name")
    }
}

/// Every definition the normal form covers, in source order (nested ones,
/// such as methods in a Python class or items in a module, included).
pub fn definitions<'t>(lang: &str, root: Syntax<'t>, src: &str) -> Vec<(Def, Syntax<'t>)> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if is_definition(lang, n) {
            // Julia: the crate's single naming rule (v6), shared with the
            // resolver and the own-name encoding. Callable-type heads
            // (`(::Type{T})(…)`) and interpolated heads (`$S(…)`) name
            // nothing; `Base.show(…)` is qualified.
            let (nn, qualified) = if lang == "julia" {
                match moonkale_content_id::julia_name(n) {
                    Some((x, q)) => (Some(x), q),
                    None => (None, false),
                }
            } else {
                (name_node(lang, n), false)
            };
            out.push((
                Def {
                    name: nn.map(|x| text(x, src).to_string()).unwrap_or_default(),
                    kind: n.kind().to_string(),
                    start: n.start_byte(),
                    end: n.end_byte(),
                    name_span: nn.map(|x| (x.start_byte(), x.end_byte())),
                    qualified: qualified || (lang == "julia" && nn.is_none()),
                },
                n,
            ));
        }
        let mut c = n.walk();
        let children: Vec<Syntax> = n.children(&mut c).collect();
        stack.extend(children.into_iter().rev());
    }
    out
}

/// Names a definition binds locally: parameters, assignment and `let`
/// targets, loop variables. A use of such a name is not a reference to a
/// top-level definition of the same spelling (a possible shadow is left
/// unresolved, as Moonkale does).
/// The local names of a definition, for diagnostics (examples).
pub fn locals_for_check(lang: &str, def: Syntax, src: &str) -> HashSet<String> {
    local_names(lang, def, src, None)
}

pub(crate) fn local_names(lang: &str, def: Syntax, src: &str, own: Option<(usize, usize)>) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut stack = vec![def];
    let binding_parent = |p: Syntax| -> bool {
        match lang {
            "julia" => matches!(p.kind(), "argument_list" | "parameter_list" | "for_binding" | "let_binding" | "keyword_parameters"),
            "python" => matches!(p.kind(), "parameters" | "default_parameter" | "typed_parameter" | "for_statement" | "lambda_parameters"),
            _ => matches!(p.kind(), "parameter" | "let_declaration" | "closure_parameters" | "for_expression" | "tuple_pattern"),
        }
    };
    while let Some(n) = stack.pop() {
        if n.kind() == "identifier" && Some((n.start_byte(), n.end_byte())) != own {
            if let Some(p) = n.parent() {
                let is_target = match p.kind() {
                    // `x = …`, `x::T` as a parameter, `x::T = default`
                    "assignment" | "typed_expression" | "typed_parameter" | "optional_parameter" => {
                        p.named_child(0).is_some_and(|c| c.id() == n.id())
                    }
                    "let_declaration" => p.child_by_field_name("pattern").is_some_and(|c| c.id() == n.id()),
                    _ => false,
                };
                // A typed parameter's name is a binding only inside a
                // parameter list; elsewhere `x::T` is an annotation.
                let in_params = |mut q: Syntax| loop {
                    if binding_parent(q) {
                        return true;
                    }
                    match q.parent() {
                        Some(x) if x.id() != def.id() => q = x,
                        _ => return false,
                    }
                };
                let assigned = p.kind() == "assignment" && is_target && p.id() != def.id();
                if assigned || (is_target && in_params(p)) || binding_parent(p) {
                    out.insert(text(n, src).to_string());
                }
            }
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c));
    }
    out
}

/// Same-file references: identifier spans inside a definition whose text is
/// the name of a definition in the file, except its own name and names it
/// binds locally. Values are indices into `defs`.
pub fn resolve(lang: &str, defs: &[(Def, Syntax)], src: &str) -> Vec<HashMap<(usize, usize), usize>> {
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, (d, _)) in defs.iter().enumerate() {
        if !d.name.is_empty() {
            by_name.entry(d.name.as_str()).or_default().push(i);
        }
    }
    defs.iter()
        .map(|(d, node)| {
            let locals = local_names(lang, *node, src, d.name_span);
            let mut refs = HashMap::new();
            let mut stack = vec![*node];
            while let Some(n) = stack.pop() {
                if matches!(n.kind(), "identifier" | "type_identifier") {
                    let span = (n.start_byte(), n.end_byte());
                    let t = text(n, src);
                    if Some(span) != d.name_span && !locals.contains(t) {
                        // Several definitions with one name (Julia methods):
                        // the reference is to the name, i.e. the generic
                        // function, so it is ambiguous at the content level
                        // and left unresolved, as Moonkale does.
                        if let Some([target]) = by_name.get(t).map(Vec::as_slice) {
                            refs.insert(span, *target);
                        }
                    }
                }
                // Nested definitions are their own units.
                let mut c = n.walk();
                for child in n.children(&mut c) {
                    if child.id() == node.id() || !is_definition(lang, child) {
                        stack.push(child);
                    }
                }
            }
            refs
        })
        .collect()
}

/// Each definition's parts, and the identities by definition index.
pub type Identified = (Vec<Vec<Part<usize>>>, HashMap<usize, Identity>);

/// The identities of every definition in one file, given references.
pub fn identify(
    lang: &str,
    src: &str,
    defs: &[(Def, Syntax)],
    refs: &[HashMap<(usize, usize), usize>],
    budget: &mut Budget,
) -> Result<Identified, CodeError> {
    let mut units = HashMap::new();
    let mut parts = Vec::new();
    for (i, ((_, node), r)) in defs.iter().zip(refs).enumerate() {
        let p = definition(lang, *node, src, r, budget)?;
        units.insert(i, p.clone());
        parts.push(p);
    }
    Ok((parts, identities(&units, budget)?))
}

/// Node kinds the WP-66 v4 normal form still refuses inside a definition
/// (some only in some languages), from Moonkale's `normalize.rs`. Used only
/// to explain refusals; the normaliser's own error is authoritative.
const REFUSED: [&str; 15] = [
    "named_argument", "global_statement", "macro_definition", "function_definition", "lambda", "class_definition",
    "match_statement", "delete_statement", "augmented_assignment", "named_expression", "import_statement",
    "import_from_statement", "use_declaration", "nonlocal_statement", "with_statement",
];

/// Why a definition was refused: the first refused construct inside it, or
/// the error.
pub(crate) fn reason(node: Syntax, err: &moonkale_content_id::ContentIdError) -> String {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if n.id() != node.id() && REFUSED.contains(&n.kind()) {
            return n.kind().to_string();
        }
        if n.is_error() || n.is_missing() {
            return "syntax error".into();
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c));
    }
    format!("{err:?}")
}

/// The record of one identified definition. `parts` reference other units
/// by key; references inside the definition's recursive group are not
/// `refs`.
pub(crate) fn to_record<K: Copy + Eq + std::hash::Hash>(
    lang: &str,
    d: &Def,
    parts: &[Part<K>],
    ids: &HashMap<K, Identity>,
    key: K,
    normaliser: &str,
) -> Record {
    let id = &ids[&key];
    let mut bytes = Vec::new();
    let mut targets = Vec::new();
    for p in parts {
        match p {
            Part::Bytes(b) => bytes.extend_from_slice(b),
            Part::Reference(k) => {
                let t = Digest(ids[k].digest);
                bytes.extend_from_slice(t.as_bytes());
                if ids[k].component_digest != id.component_digest && !targets.contains(&t) {
                    targets.push(t);
                }
            }
        }
    }
    Record {
        id: Digest(id.digest),
        lang: lang.into(),
        normaliser: normaliser.into(),
        kind: d.kind.clone(),
        component: Digest(id.component_digest),
        index: id.component_index,
        component_size: id.component_size,
        bytes,
        display: d.name.as_bytes().to_vec(),
        status: Status::Complete,
        refs: targets,
        nominal_refs: Vec::new(),
    }
}

/// A parsed file: definitions, their records, and the refused ones.
pub struct File {
    /// The definitions that got an identity, in source order.
    pub defs: Vec<Def>,
    pub records: Vec<Record>,
    /// Definitions the normal form refused, with the reason.
    pub refused: Vec<(Def, String)>,
}

/// Parses a file and turns each definition into a Sophia record whose id is
/// the WP-66 digest (Moonkale's `NodeId::from_content`). The record's bytes
/// are the canonical parts with references written as target digests.
///
/// A definition the normal form refuses is skipped and explained;
/// references to it are left unresolved in the others (so their ids are
/// what they would be if it were not a definition at all).
pub fn file(lang: &str, src: &str) -> Result<File, CodeError> {
    let mut budget = Budget::new();
    let tree = parse(lang, src, &mut budget)?;
    let found = definitions(lang, tree.root_node(), src);
    let mut refs = resolve(lang, &found, src);
    let mut failed: HashMap<usize, String> = HashMap::new();
    let mut parts: HashMap<usize, Vec<Part<usize>>> = HashMap::new();
    loop {
        let before = failed.len();
        for (i, (_, node)) in found.iter().enumerate() {
            if failed.contains_key(&i) || parts.contains_key(&i) {
                continue;
            }
            match definition(lang, *node, src, &refs[i], &mut budget) {
                Ok(p) => {
                    parts.insert(i, p);
                }
                Err(moonkale_content_id::ContentIdError::LimitExceeded) => {
                    return Err(moonkale_content_id::ContentIdError::LimitExceeded.into());
                }
                Err(e) => {
                    failed.insert(i, reason(*node, &e));
                }
            }
        }
        if failed.len() == before {
            break;
        }
        // Drop references to refused definitions and redo their users.
        for (i, r) in refs.iter_mut().enumerate() {
            let n = r.len();
            r.retain(|_, t| !failed.contains_key(t));
            if r.len() != n {
                parts.remove(&i);
            }
        }
    }
    let ids = identities(&parts, &mut budget)?;
    let mut records = Vec::new();
    let mut defs = Vec::new();
    let normaliser = normaliser();
    for (i, (d, _)) in found.iter().enumerate() {
        if !ids.contains_key(&i) {
            continue;
        }
        defs.push(d.clone());
        records.push(to_record(lang, d, &parts[&i], &ids, i, &normaliser));
    }
    let mut refused: Vec<(Def, String)> = failed.into_iter().map(|(i, r)| (found[i].0.clone(), r)).collect();
    refused.sort_by_key(|(d, _)| d.start);
    Ok(File { defs, records, refused })
}
