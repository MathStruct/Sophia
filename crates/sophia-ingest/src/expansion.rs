//! Macro expansions as transformation records (experiment E-4): the output
//! of `tests/julia-semantics/expansions.jl` (each module-level form that
//! calls a macro, replaced by Julia's expansion of it as source text) spliced
//! into the package in place of the forms and ingested like any source, so
//! that what the expansions define gets Sophia's ids. For every method Julia
//! made, whether a definition of the original source covers it, one of the
//! expansion does, or neither; and for every expansion, the transformation
//! record SPEC-006 §6a describes (by, input, parameters, world, output). See
//! `expansion.md` and [[Semantic Boundary Experiments]] §E-4.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use sophia_hash::Digest;
use sophia_hash::canonical::Writer as Canonical;

use crate::conformance::{module_forms, template_at};
use crate::module::{self, ModuleError};

/// One record of `expansions.jl`: one expansion of one form.
#[derive(Debug, Clone, Default)]
pub struct Expansion {
    pub file: String,
    pub line: usize,
    /// The record's number within its form, from 1; 0 for `rest` (what a
    /// form made outside any expansion).
    pub k: usize,
    /// `eval`, `macro` or `rest`.
    pub kind: String,
    /// The macro's binding (`Core.@eval`, `Base.@deprecate`).
    pub by: String,
    /// The module the record was evaluated in (`Pkg.Inner` for `@eval
    /// Inner ex`).
    pub module: String,
    /// Recomputed from the recorded inputs alone, the same text (`None`
    /// for `rest`).
    pub stable: Option<bool>,
    /// `ok`, `unparsed` or `failed` (see `expansions.jl`).
    pub status: String,
    pub ns_expand: u64,
    pub ns_eval: u64,
    /// The values the template's `$` splices had (`repr`), for an eval record.
    pub inputs: Vec<String>,
    /// An eval record's template (the `@eval` argument as written).
    pub template: String,
    pub text: String,
    /// (owner, signature) of each method the record's text made.
    pub methods: Vec<(String, String)>,
    /// (kind, `Module.name`) of each name it bound.
    pub bindings: Vec<(String, String)>,
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

/// Reads `expansions.jl`'s output: Julia's version and the records in order.
pub fn parse_expansions(tsv: &str) -> (String, Vec<Expansion>) {
    let mut version = String::new();
    let mut out: Vec<Expansion> = Vec::new();
    for l in tsv.lines() {
        let c: Vec<&str> = l.split('\t').collect();
        let num = |i: usize| c.get(i).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0);
        match c[0] {
            "julia" if c.len() == 2 => version = c[1].to_string(),
            "record" if c.len() == 15 => out.push(Expansion {
                file: c[1].to_string(),
                line: num(2) as usize,
                k: num(3) as usize,
                kind: c[4].to_string(),
                by: c[5].to_string(),
                module: c[6].to_string(),
                stable: match c[7] {
                    "yes" => Some(true),
                    "no" => Some(false),
                    _ => None,
                },
                status: c[8].to_string(),
                ns_expand: num(9),
                ns_eval: num(10),
                inputs: if c[12].is_empty() { Vec::new() } else { unescape(c[12]).split('\t').map(str::to_string).collect() },
                template: unescape(c[13]),
                text: unescape(c[14]),
                ..Default::default()
            }),
            "method" if c.len() == 6 => {
                if let Some(r) = out.last_mut() {
                    r.methods.push((c[4].to_string(), c[5].to_string()));
                }
            }
            "binding" if c.len() == 6 => {
                if let Some(r) = out.last_mut() {
                    r.bindings.push((c[4].to_string(), c[5].to_string()));
                }
            }
            _ => {}
        }
    }
    (version, out)
}

/// What E-4 found for one method Julia made in a form that calls a macro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MethodOutcome {
    /// A definition of the source as written covers it (not a template):
    /// Sophia knew it before expansion.
    Known,
    /// Only a definition of the expansion covers it, and that definition
    /// has an id: recovered by expansion.
    Recovered,
    /// Only a definition of the expansion covers it, but the normaliser
    /// refuses that definition (no id).
    RecoveredRefused,
    /// No definition of the expansion names its function, but one has an id
    /// and no name: Moonkale's naming gives `T{P}(…) = …` none ([[Julia
    /// Conformance]] part 8), so the method is likely there and its
    /// function is left open.
    Unnamed,
    /// Julia evaluated the record in another module than the form's
    /// (`@eval Inner ex`), and the text spliced in place of the form puts
    /// its definitions in the form's module: a definition of the expansion
    /// has the function's name, in the wrong module (approximates).
    OtherModule,
    /// A constructor Julia adds for a `struct` of the expansion (the type
    /// is a definition; its default constructors are not written).
    Constructor,
    /// No definition of the expansion covers it (or the form's other
    /// output: a record 0 method).
    Open,
}

/// One method of a form, by record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodRow {
    pub file: String,
    pub line: usize,
    pub k: usize,
    pub by: String,
    pub owner: String,
    pub sig: String,
    pub outcome: MethodOutcome,
    /// For `RecoveredRefused`, the normaliser's reason.
    pub detail: String,
}

/// One expansion as a transformation record (SPEC-006 §6a): `address` is
/// the input address, computed before the expansion from `by`, the macro's
/// own id when the package defines it, `input`, `parameters` and `world`;
/// `output` is what the expansion's text holds (the text's digest and the
/// ids of its definitions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transformation {
    pub file: String,
    pub line: usize,
    pub k: usize,
    pub by: String,
    /// The id of the macro's definition, when the package defines it and
    /// the normaliser gives it one.
    pub by_id: Option<Digest>,
    /// The digest of the macro definition's source text, when the package
    /// defines it: in the address when there is no id (a macro whose body
    /// is a `quote` with `$` is refused, so most have none).
    pub by_text: Option<Digest>,
    /// The form's source text (bytes, as written).
    pub input: Digest,
    /// The template and the splices' values, for an eval record.
    pub template: String,
    pub parameters: Vec<String>,
    /// Julia's version: the world the macros of Base and the stdlibs come
    /// from (an approximation: a package's macro is identified by `by_id`).
    pub world: String,
    pub address: Digest,
    pub text: Digest,
    /// Ids of the expansion's definitions that have one, in source order.
    pub output: Vec<Digest>,
    /// The expansion's definitions the normaliser refused.
    pub refused: usize,
}

/// The whole comparison of E-4.
#[derive(Debug, Default)]
pub struct ExpansionLedger {
    pub julia: String,
    pub rows: Vec<MethodRow>,
    pub transformations: Vec<Transformation>,
    /// Forms compared, and records in them (`rest` excluded).
    pub forms: usize,
    pub records: usize,
    /// Forms at a line where Sophia's parse has no module-level form, or a
    /// file read into several modules: not compared.
    pub unmatched: Vec<(String, usize)>,
    /// Lines with several forms in Sophia's parse: not compared.
    pub ambiguous: Vec<(String, usize)>,
    /// Records not recomputed to the same text from their inputs.
    pub unstable: Vec<(String, usize, usize)>,
    /// Records whose address another record shares with a different text:
    /// state the address does not cover changed between them (E-4's
    /// failure condition; `stable` re-expands in the current state only).
    pub clashes: Vec<(String, usize, usize)>,
    /// Records whose text did not parse back or threw in Julia.
    pub not_ok: Vec<(String, usize, usize, String)>,
    /// Bindings Julia's records made, and those a definition of the
    /// expansion names.
    pub bindings: usize,
    pub bindings_covered: usize,
    /// Definitions Sophia finds in the compared forms as written (templates
    /// among them), and in their expansions (refused among them).
    pub definitions_before: usize,
    pub templates_before: usize,
    pub definitions_after: usize,
    pub refused_after: usize,
    /// Bytes of the compared forms as written and of their expansions.
    pub bytes_before: usize,
    pub bytes_after: usize,
    pub ns_expand: u64,
    pub ns_eval: u64,
}

impl ExpansionLedger {
    pub fn tally(&self) -> BTreeMap<MethodOutcome, usize> {
        let mut t = BTreeMap::new();
        for r in &self.rows {
            *t.entry(r.outcome).or_default() += 1;
        }
        t
    }
}

/// One of Sophia's definitions: its node kind, the generic function or
/// name it defines (`Module.name`; `None` for a callable object's method),
/// its id, or why the normaliser refused it.
pub(crate) struct Seen {
    pub(crate) kind: String,
    /// The module the definition is in (`Pkg.Inner`).
    pub(crate) module: String,
    pub(crate) owner: Option<String>,
    pub(crate) id: Option<Digest>,
    pub(crate) refusal: Option<String>,
}

/// Sophia's definitions of a package, by span (file, start, end).
pub(crate) type Owners = HashMap<(usize, usize, usize), Seen>;

/// Also returns the parse trees and the ingest itself (E-5 reads its
/// references).
pub(crate) fn owners(d: &module::Discovered) -> Result<(Owners, Vec<moonkale_content_id::Tree>, module::Ingested), ModuleError> {
    let r = module::ingest(d)?;
    let mut trees = Vec::new();
    for f in &d.files {
        trees.push(moonkale_content_id::parse("julia", &f.text, &mut moonkale_content_id::Budget::new())?);
    }
    let mut scopes = crate::code::JuliaScopes::default();
    for (f, t) in d.files.iter().zip(&trees) {
        crate::code::julia_scopes(t.root_node(), &f.text, &f.base_module, &mut scopes);
    }
    let mut defs: HashMap<(usize, usize, usize), crate::code::Def> = HashMap::new();
    for (f, (sf, t)) in d.files.iter().zip(&trees).enumerate() {
        for (x, _) in crate::code::definitions("julia", t.root_node(), &sf.text) {
            defs.insert((f, x.start, x.end), x);
        }
    }
    let mut out = Owners::new();
    for x in &r.module.definitions {
        let key = (x.key.0 as usize, x.key.1, x.key.2);
        let Some(def) = defs.get_mut(&key) else { continue };
        def.module = x.module.clone();
        crate::code::julia_extends(def, &scopes);
        let qualified = |n: &str| {
            let mut p = x.module.clone();
            p.push(n.to_string());
            p.join(".")
        };
        let owner = match (&def.extends, &x.name) {
            (Some(p), _) => Some(p.join(".")),
            (None, Some(n)) if def.kind == "macro_definition" => Some(qualified(&format!("@{}", n.trim_start_matches('@')))),
            (None, Some(n)) => Some(qualified(n)),
            (None, None) => None,
        };
        let (id, refusal) = match &x.identity {
            Ok(i) => (Some(Digest(i.digest)), None),
            Err(e) => (None, Some(e.to_string())),
        };
        out.insert(key, Seen { kind: def.kind.clone(), module: x.module.join("."), owner, id, refusal });
    }
    Ok((out, trees, r))
}

/// The last component of a dotted name (`Base.Math.cbrt` → `cbrt`): Julia
/// and Sophia may name one function by two bindings (SP-013).
fn last(o: &str) -> &str {
    o.rsplit('.').next().unwrap_or(o)
}

/// Whether a definition naming `owner` covers a method of Julia's function
/// `o`: the same function, a function of that name (an alias, SP-013), a
/// constructor (Sophia names it `T.new`), or a callable object's method
/// (no name on Sophia's side, `(::T)` on Julia's).
fn covers(owner: Option<&str>, o: &str) -> bool {
    match owner {
        None => o.starts_with("(::"),
        Some(w) => {
            let w = w.strip_suffix(".new").unwrap_or(w);
            w == o || last(w) == last(o)
        }
    }
}

fn is_function(kind: &str) -> bool {
    matches!(kind, "function_definition" | "assignment" | "short_function_definition" | "macro_definition")
}

/// Compares the package at `dir` with the output of `expansions.jl` for it
/// (E-4): each form's records are spliced into its file in place of the
/// form, one after the other, and the package is ingested again; a method a
/// record made is covered by a definition in that record's text with its
/// generic function (or a function of that name: an alias, SP-013).
pub fn expansion_ledger(dir: &Path, tsv: &str) -> Result<ExpansionLedger, ModuleError> {
    let dir = &dir.canonicalize().map_err(|e| ModuleError::Io(format!("{}: {e}", dir.display())))?;
    let (_, root) = module::package_root(dir).ok_or_else(|| ModuleError::Io("no Project.toml and src/<name>.jl".into()))?;
    let src = dir.join("src");
    let d = module::discover(&root)?;
    let rel: Vec<String> =
        d.files.iter().map(|f| Path::new(&f.path).strip_prefix(&src).unwrap_or(Path::new(&f.path)).to_string_lossy().to_string()).collect();
    let line_of = |text: &str, byte: usize| text[..byte].matches('\n').count() + 1;
    let (before, trees, _) = owners(&d)?;
    let forms: Vec<Vec<crate::conformance::SophiaForm>> = trees.iter().map(|t| module_forms(t.root_node())).collect();

    let (julia, records) = parse_expansions(tsv);
    let mut ledger = ExpansionLedger { julia: julia.clone(), ..Default::default() };
    let mut by_form: BTreeMap<(String, usize), Vec<Expansion>> = BTreeMap::new();
    for r in records {
        by_form.entry((r.file.clone(), r.line)).or_default().push(r);
    }

    // Each form found in Sophia's parse: (file, form index) → its records.
    let mut matched: Vec<(usize, usize, Vec<Expansion>)> = Vec::new();
    for ((file, at), rs) in by_form {
        let fs: Vec<usize> = rel.iter().enumerate().filter(|(_, r)| **r == file).map(|(i, _)| i).collect();
        let [f] = fs[..] else {
            ledger.unmatched.push((file, at));
            continue;
        };
        let here: Vec<usize> = (0..forms[f].len()).filter(|&i| line_of(&d.files[f].text, forms[f][i].start) == at).collect();
        match here[..] {
            [i] => matched.push((f, i, rs)),
            [] => ledger.unmatched.push((file, at)),
            _ => ledger.ambiguous.push((file, at)),
        }
    }

    // The package with each form replaced by its records' texts, one per
    // line block; the span of each record's text.
    let mut expanded = d.clone();
    let mut regions: HashMap<(usize, usize, usize), (usize, usize)> = HashMap::new(); // (f, form, k) → span
    let mut per_file: BTreeMap<usize, Vec<(usize, &Vec<Expansion>)>> = BTreeMap::new();
    for (f, i, rs) in &matched {
        per_file.entry(*f).or_default().push((*i, rs));
    }
    for (f, mut list) in per_file {
        list.sort_by_key(|(i, _)| forms[f][*i].start);
        let old = &d.files[f].text;
        let mut text = String::with_capacity(old.len());
        let mut at = 0;
        for (i, rs) in list {
            let form = &forms[f][i];
            text.push_str(&old[at..form.start]);
            for r in rs.iter().filter(|r| r.k > 0) {
                let a = text.len();
                text.push_str(&r.text);
                regions.insert((f, i, r.k), (a, text.len()));
                text.push('\n');
            }
            at = form.end;
        }
        text.push_str(&old[at..]);
        expanded.files[f].text = text;
    }
    let (after, _, _) = owners(&expanded)?;

    // The package's own macros, for `by_id` and `by_text`.
    let macros: HashMap<&str, (Option<Digest>, Digest)> = before
        .iter()
        .filter(|(_, x)| x.kind == "macro_definition")
        .filter_map(|((f, a, b), x)| Some((x.owner.as_deref()?, (x.id, Digest::of(&d.files[*f].text.as_bytes()[*a..*b])))))
        .collect();

    for (f, i, rs) in &matched {
        let (f, i) = (*f, *i);
        let form = &forms[f][i];
        let old = &d.files[f].text;
        ledger.forms += 1;
        ledger.bytes_before += form.end - form.start;
        // The form as written: its definitions; those not templates cover
        // their methods already.
        let mut known: Vec<Option<String>> = Vec::new();
        for ((g, a, b), x) in &before {
            if *g == f && form.start <= *a && *b <= form.end {
                ledger.definitions_before += 1;
                if template_at(&trees[f], *a, *b) {
                    ledger.templates_before += 1;
                } else if is_function(&x.kind) {
                    known.push(x.owner.clone());
                }
            }
        }
        let known_covers = |o: &str| known.iter().any(|w| covers(w.as_deref(), o));
        let input = Digest::of(&old.as_bytes()[form.start..form.end]);
        for r in rs {
            if r.k == 0 {
                for (o, s) in &r.methods {
                    let outcome = if known_covers(o) { MethodOutcome::Known } else { MethodOutcome::Open };
                    ledger.rows.push(MethodRow {
                        file: r.file.clone(),
                        line: r.line,
                        k: 0,
                        by: "-".into(),
                        owner: o.clone(),
                        sig: s.clone(),
                        outcome,
                        detail: String::new(),
                    });
                }
                continue;
            }
            ledger.records += 1;
            ledger.bytes_after += r.text.len();
            ledger.ns_expand += r.ns_expand;
            ledger.ns_eval += r.ns_eval;
            if r.stable == Some(false) {
                ledger.unstable.push((r.file.clone(), r.line, r.k));
            }
            if r.status != "ok" {
                ledger.not_ok.push((r.file.clone(), r.line, r.k, r.status.clone()));
            }
            let (a0, b0) = regions[&(f, i, r.k)];
            let mut mine: Vec<((usize, usize), &Seen)> =
                after.iter().filter(|((g, a, b), _)| *g == f && a0 <= *a && *b <= b0).map(|((_, a, b), v)| ((*a, *b), v)).collect();
            mine.sort_by_key(|(k, _)| *k);
            ledger.definitions_after += mine.len();
            let refused = mine.iter().filter(|(_, x)| x.id.is_none()).count();
            ledger.refused_after += refused;
            for (o, s) in &r.methods {
                let covering: Vec<&Seen> =
                    mine.iter().filter(|(_, x)| is_function(&x.kind) && covers(x.owner.as_deref(), o)).map(|(_, x)| *x).collect();
                let structs = mine.iter().any(|(_, x)| x.kind == "struct_definition" && x.owner.as_deref() == Some(o.as_str()));
                let outcome = if known_covers(o) {
                    MethodOutcome::Known
                } else if !covering.is_empty() && covering.iter().all(|x| x.module != r.module) {
                    MethodOutcome::OtherModule
                } else if covering.iter().any(|x| x.id.is_some()) {
                    MethodOutcome::Recovered
                } else if !covering.is_empty() {
                    MethodOutcome::RecoveredRefused
                } else if structs {
                    MethodOutcome::Constructor
                } else if mine.iter().any(|(_, x)| is_function(&x.kind) && x.owner.is_none() && x.id.is_some()) {
                    MethodOutcome::Unnamed
                } else {
                    MethodOutcome::Open
                };
                ledger.rows.push(MethodRow {
                    file: r.file.clone(),
                    line: r.line,
                    k: r.k,
                    by: r.by.clone(),
                    owner: o.clone(),
                    sig: s.clone(),
                    outcome,
                    detail: match outcome {
                        MethodOutcome::RecoveredRefused => covering.iter().find_map(|x| x.refusal.clone()).unwrap_or_default(),
                        // What the expansion's text does define.
                        MethodOutcome::Open => format!(
                            "sophia: {}",
                            mine.iter()
                                .map(|(_, x)| format!("{} {}", x.kind, x.owner.as_deref().unwrap_or("-")))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        _ => String::new(),
                    },
                });
            }
            for (_, name) in &r.bindings {
                ledger.bindings += 1;
                if mine.iter().any(|(_, x)| x.owner.as_deref() == Some(name.as_str())) {
                    ledger.bindings_covered += 1;
                }
            }
            let (by_id, by_text) = match macros.get(r.by.as_str()) {
                Some((id, t)) => (*id, Some(*t)),
                None => (None, None),
            };
            let mut c = Canonical::new();
            c.str("by", &r.by);
            match (&by_id, &by_text) {
                (Some(id), _) => c.field("by-id", id.as_bytes()),
                (None, Some(t)) => c.field("by-text", t.as_bytes()),
                (None, None) => {}
            }
            c.field("input", input.as_bytes());
            // Several `@eval`s in one loop share the form and the values.
            c.str("template", &r.template);
            for p in &r.inputs {
                c.str("parameter", p);
            }
            c.str("world", &julia);
            ledger.transformations.push(Transformation {
                file: r.file.clone(),
                line: r.line,
                k: r.k,
                by: r.by.clone(),
                by_id,
                by_text,
                input,
                template: r.template.clone(),
                parameters: r.inputs.clone(),
                world: julia.clone(),
                address: Digest::of(&c.finish()),
                text: Digest::of(r.text.as_bytes()),
                output: mine.iter().filter_map(|(_, x)| x.id).collect(),
                refused,
            });
        }
    }
    let mut texts: HashMap<Digest, Vec<Digest>> = HashMap::new();
    for t in &ledger.transformations {
        let e = texts.entry(t.address).or_default();
        if !e.contains(&t.text) {
            e.push(t.text);
        }
    }
    ledger.clashes = ledger.transformations.iter().filter(|t| texts[&t.address].len() > 1).map(|t| (t.file.clone(), t.line, t.k)).collect();
    Ok(ledger)
}

/// Whether each expanded form's records made the same methods as the form
/// itself: the owners (with multiplicity) of the methods `expansions.jl`
/// lists for the form's records, against those `macros.jl` lists for the
/// form as written. Returns (file, line, the same) per form with a record.
pub fn faithful(expansions: &str, macros: &str) -> Vec<(String, usize, bool)> {
    let (_, records) = parse_expansions(expansions);
    let mut mine: BTreeMap<(String, usize), BTreeMap<String, usize>> = BTreeMap::new();
    for r in &records {
        let e = mine.entry((r.file.clone(), r.line)).or_default();
        for (o, _) in &r.methods {
            *e.entry(o.clone()).or_default() += 1;
        }
    }
    let mut theirs: BTreeMap<(String, usize), BTreeMap<String, usize>> = BTreeMap::new();
    for l in macros.lines() {
        let c: Vec<&str> = l.split('\t').collect();
        if c.len() == 5 && c[0] == "method" {
            *theirs.entry((c[1].to_string(), c[2].parse().unwrap_or(0))).or_default().entry(c[3].to_string()).or_default() += 1;
        }
    }
    mine.into_iter()
        .map(|(k, m)| {
            let same = theirs.get(&k).cloned().unwrap_or_default() == m;
            (k.0, k.1, same)
        })
        .collect()
}
