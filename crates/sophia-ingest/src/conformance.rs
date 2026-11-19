//! Part 3 of the Julia conformance suite (dispatch): what Sophia says a
//! module-level name in a definition's body refers to, against what Julia's
//! lowered code says (`tests/julia-semantics/calls.jl`). See
//! `conformance.md` and [[Julia Conformance]]. Part 4 (world change,
//! `world_ledger`): the same comparison before and after a change that adds
//! a method or an import, and which of Sophia's facts the change left true,
//! made false, or moved.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::module::{self, ModuleError};

/// What one comparison found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Outcome {
    /// A content reference to a function definition, and every method of
    /// Julia's function comes from that definition (one, or several made by
    /// default arguments): the call can reach only it.
    KnowsOnlyDefinition,
    /// A generic use (`nominal_refs`) of the function Julia calls; which of
    /// its methods a call reaches is left open.
    KnowsFunction,
    /// A content reference to a type, a module or a constant, and Julia's
    /// binding is that one.
    KnowsBinding,
    /// A content reference to one method of a function that has several in
    /// Julia: the id says "reaches this method", Julia may dispatch to
    /// another (wrong).
    WrongOneOfMany,
    /// Sophia's target is not the function, type or binding Julia uses
    /// (wrong).
    WrongTarget,
    /// Sophia states a reference and Julia's lowered code of that
    /// definition does not use the name (a quoted name, code Julia did not
    /// compile here, a name lowering folds away).
    NoJuliaUse,
    /// As `NoJuliaUse`, and every use is written `$name` inside a quoted
    /// template (`for (T, uplo) in … @eval … $uplo`): it is the loop's
    /// variable, spliced in when the template is evaluated, not the module's
    /// binding (wrong).
    WrongInterpolated,
    /// Julia uses a function or type of this package; Sophia states nothing
    /// (an import, a qualified name, a local, an unresolved name).
    OpenHere,
    /// Julia uses a function or type from outside the package (`Base.+`);
    /// Sophia states nothing.
    OpenOutside,
}

/// One comparison: in the definition at `file:line` named `caller`, the
/// use of `name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub file: String,
    pub line: usize,
    pub caller: String,
    pub name: String,
    pub outcome: Outcome,
    /// Sophia's target and Julia's, for reading the ledger.
    pub detail: String,
}

/// The whole comparison.
#[derive(Debug, Default)]
pub struct Ledger {
    pub rows: Vec<Row>,
    /// Lines of Julia methods inside none of Sophia's identified definitions
    /// (macro output, default constructors, refused definitions); methods on
    /// one line (default arguments) count once.
    pub julia_method_lines_outside: usize,
    /// Lines with Julia methods where several Sophia definitions start and
    /// end (`f(x) = 1; g(x) = 2`): not attributed, not compared.
    pub julia_lines_ambiguous: usize,
    /// Sophia's identified definitions inside which Julia lists no use of a
    /// binding (no Julia method there, or one whose body uses no global).
    pub sophia_without_julia: usize,
    /// Each identified definition's id, by (file, first line, name).
    pub ids: BTreeMap<(String, usize, String), [u8; 32]>,
    /// Each function with a single definition that a caller refers to by
    /// content (a `KnowsOnlyDefinition` target): its name and place, how
    /// many definitions refer to it directly, and how many contain its id
    /// transitively. A second method makes the direct ones change their
    /// claim and moves all of their ids (part 4, `IdMoved`).
    pub dependants: Vec<(String, usize, usize)>,
}

impl Ledger {
    pub fn tally(&self) -> BTreeMap<Outcome, usize> {
        let mut t = BTreeMap::new();
        for r in &self.rows {
            *t.entry(r.outcome).or_default() += 1;
        }
        t
    }
}

/// One line of `calls.jl`'s output.
struct Use {
    name: String,
    binding: String,
    kind: String,
    owner: String,
    nmethods: usize,
    here: Vec<(String, usize)>,
}

/// Compares the package at `dir` (with `Project.toml` and `src/`) with the
/// output of `calls.jl` for it.
pub fn dispatch_ledger(dir: &Path, truth: &str) -> Result<Ledger, ModuleError> {
    let (_, root) = module::package_root(dir).ok_or_else(|| ModuleError::Io("no Project.toml and src/<name>.jl".into()))?;
    let src = dir.canonicalize().map_err(|e| ModuleError::Io(format!("{}: {e}", dir.display())))?.join("src");
    ledger_of(&module::discover(&root)?, &src, truth)
}

/// As `dispatch_ledger`, on files already discovered under `src`.
fn ledger_of(d: &module::Discovered, src: &Path, truth: &str) -> Result<Ledger, ModuleError> {
    let r = module::ingest(d)?;
    let jm = &r.module;
    let rel: Vec<String> =
        d.files.iter().map(|f| Path::new(&f.path).strip_prefix(src).unwrap_or(Path::new(&f.path)).to_string_lossy().to_string()).collect();
    // Line of a byte offset, by binary search over each file's newlines.
    let newlines: Vec<Vec<usize>> = d.files.iter().map(|f| f.text.match_indices('\n').map(|(i, _)| i).collect()).collect();
    let line = |f: usize, byte: usize| newlines[f].partition_point(|&i| i < byte) + 1;

    // Sophia's identified definitions, as (file, first line, last line).
    let identified: Vec<usize> = (0..jm.definitions.len()).filter(|&i| jm.definitions[i].identity.is_ok()).collect();
    let spans: Vec<(usize, usize, usize)> =
        jm.definitions.iter().map(|x| (x.key.0 as usize, line(x.key.0 as usize, x.key.1), line(x.key.0 as usize, x.key.2))).collect();
    let span = |i: usize| spans[i];
    // Each generic use, under the innermost definition around it (as
    // `module::ingest` attributes them).
    // Whether a use at this span is written `$name` (an interpolation).
    let spliced = |f: usize, start: usize| d.files[f].text[..start].ends_with('$');
    let mut generic_of: HashMap<usize, Vec<(&str, bool)>> = HashMap::new();
    for (gf, s, n) in &jm.calls_generic {
        let around = (0..jm.definitions.len())
            .filter(|&i| {
                let k = jm.definitions[i].key;
                k.0 == *gf && k.1 <= s.0 && s.1 <= k.2
            })
            .min_by_key(|&i| jm.definitions[i].key.2 - jm.definitions[i].key.1);
        if let Some(i) = around {
            generic_of.entry(i).or_default().push((n.as_str(), spliced(*gf as usize, s.0)));
        }
    }
    let by_key: HashMap<_, usize> = jm.definitions.iter().enumerate().map(|(i, x)| (x.key, i)).collect();
    let qualified = |x: &moonkale_content_id::julia::JuliaDefinition, name: &str| {
        let mut p = x.module.clone();
        p.push(name.to_string());
        p.join(".")
    };

    // Julia: each method's uses, attributed to the innermost identified
    // definition whose lines contain the method's line.
    let mut julia: HashMap<usize, Vec<Use>> = HashMap::new();
    let mut seen_methods: HashMap<usize, bool> = HashMap::new();
    let mut ledger = Ledger::default();
    let mut outside: std::collections::HashSet<(String, usize)> = std::collections::HashSet::new();
    let mut ambiguous: std::collections::HashSet<(String, usize)> = std::collections::HashSet::new();
    for l in truth.lines() {
        let c: Vec<&str> = l.split('\t').collect();
        if c.len() != 9 {
            continue;
        }
        let (file, at) = (c[0], c[1].parse::<usize>().unwrap_or(0));
        let mut within: Vec<usize> = identified
            .iter()
            .copied()
            .filter(|&i| {
                let (f, a, b) = span(i);
                rel[f] == file && a <= at && at <= b
            })
            .collect();
        let size = |i: usize| span(i).2 - span(i).1;
        let least = within.iter().map(|&i| size(i)).min();
        within.retain(|&i| Some(size(i)) == least);
        // Several definitions on one line: Julia's line cannot tell them
        // apart, so neither is compared.
        if within.len() > 1 {
            ambiguous.insert((file.to_string(), at));
            continue;
        }
        let Some(&i) = within.first() else {
            outside.insert((file.to_string(), at));
            continue;
        };
        seen_methods.insert(i, true);
        let here = if c[8] == "-" {
            Vec::new()
        } else {
            c[8].split(',').filter_map(|x| x.rsplit_once(':')).map(|(f, n)| (f.to_string(), n.parse().unwrap_or(0))).collect()
        };
        julia.entry(i).or_default().push(Use {
            name: c[2].to_string(),
            binding: c[3].to_string(),
            kind: c[4].to_string(),
            owner: c[5].to_string(),
            nmethods: c[7].parse().unwrap_or(0),
            here,
        });
    }
    ledger.julia_method_lines_outside = outside.len();
    ledger.julia_lines_ambiguous = ambiguous.len();

    // Functions with one definition that some caller refers to by content.
    let mut only: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for &i in &identified {
        let x = &jm.definitions[i];
        let (f, first, _) = span(i);
        if let Ok(id) = &x.identity {
            ledger.ids.insert((rel[f].clone(), first, x.name.clone().unwrap_or_else(|| "?".into())), id.digest);
        }
        if !seen_methods.contains_key(&i) {
            ledger.sophia_without_julia += 1;
            continue;
        }
        let uses = julia.remove(&i).unwrap_or_default();
        let caller = x.name.clone().unwrap_or_else(|| "?".into());
        // Sophia's claims, by the binding they name (`Disp.h`), so that a
        // body using `h` and `Inner.h` keeps them apart. A claim is matched
        // to Julia's use of that binding, else to a use of the same name.
        let mut claimed: Vec<String> = Vec::new();
        let find = |name: &str, mine: &str| uses.iter().find(|u| u.binding == mine).or_else(|| uses.iter().find(|u| u.name == name));
        let mut row = |name: &str, outcome, detail: String| {
            ledger.rows.push(Row { file: rel[f].clone(), line: first, caller: caller.clone(), name: name.to_string(), outcome, detail });
        };
        // Content references, one claim per target.
        let mut targets: Vec<usize> = x.references.iter().filter_map(|(_, t)| by_key.get(t).copied()).collect();
        targets.sort_unstable();
        targets.dedup();
        // A claim with no Julia use: wrong if every use of it is spliced.
        let absent = |all_spliced: bool| if all_spliced { Outcome::WrongInterpolated } else { Outcome::NoJuliaUse };
        for t in targets {
            let all_spliced = x.references.iter().filter(|(_, k)| by_key.get(k) == Some(&t)).all(|(sp, _)| spliced(f, sp.0));
            let tx = &jm.definitions[t];
            let Some(name) = tx.name.clone() else { continue };
            let (tf, tl, te) = span(t);
            let mine = qualified(tx, &name);
            claimed.push(mine.clone());
            let Some(u) = find(&name, &mine) else {
                row(&name, absent(all_spliced), format!("sophia {mine} ({}:{tl})", rel[tf]));
                continue;
            };
            claimed.push(u.binding.clone());
            let detail = format!("sophia {mine} ({}:{tl}) / julia {} {} with {} methods", rel[tf], u.kind, u.owner, u.nmethods);
            // Julia's methods inside the target's lines (Julia's line of a
            // multi-line short form is not always the first).
            let inside = u.here.iter().filter(|(hf, hl)| *hf == rel[tf] && tl <= *hl && *hl <= te).count();
            let outcome = match u.kind.as_str() {
                "function" if u.owner == mine && inside > 0 && inside == u.nmethods => Outcome::KnowsOnlyDefinition,
                "function" if u.owner == mine && inside > 0 => Outcome::WrongOneOfMany,
                "function" => Outcome::WrongTarget,
                "type" | "module" | "other" if u.owner == mine => Outcome::KnowsBinding,
                _ => Outcome::WrongTarget,
            };
            if outcome == Outcome::KnowsOnlyDefinition {
                only.insert(t);
            }
            row(&name, outcome, detail);
        }
        // Generic uses: the crate resolves them in the user's module.
        let found = generic_of.remove(&i).unwrap_or_default();
        let mut generic: Vec<&str> = found.iter().map(|(n, _)| *n).collect();
        generic.sort_unstable();
        generic.dedup();
        for name in generic {
            let mine = qualified(x, name);
            claimed.push(mine.clone());
            let all_spliced = found.iter().filter(|(n, _)| *n == name).all(|(_, s)| *s);
            claimed.extend(find(name, &mine).map(|u| u.binding.clone()));
            let outcome = match find(name, &mine) {
                None => absent(all_spliced),
                Some(u) if u.kind == "function" && u.owner == mine => Outcome::KnowsFunction,
                Some(_) => Outcome::WrongTarget,
            };
            let julia = find(name, &mine).map(|u| format!("{} {}", u.kind, u.owner)).unwrap_or_default();
            row(name, outcome, format!("sophia generic {mine} / julia {julia}"));
        }
        // Julia's uses Sophia states nothing about: functions and types only.
        let mut open: Vec<&Use> =
            uses.iter().filter(|u| matches!(u.kind.as_str(), "function" | "type") && !claimed.contains(&u.binding)).collect();
        open.sort_by(|a, b| a.binding.cmp(&b.binding));
        open.dedup_by(|a, b| a.binding == b.binding);
        for u in open {
            // Compiler-made names (closures, keyword sorters) are not in the source.
            if u.name.starts_with('#') {
                continue;
            }
            let outcome = if u.here.is_empty() { Outcome::OpenOutside } else { Outcome::OpenHere };
            row(&u.name.clone(), outcome, format!("julia {} {} via {}", u.kind, u.owner, u.binding));
        }
    }
    // Whose id contains each such function's id: the definitions that
    // reach it through content references (Merkle: the id of a referrer
    // contains the id of what it refers to).
    let mut referrers: HashMap<usize, Vec<usize>> = HashMap::new();
    for &i in &identified {
        for (_, k) in &jm.definitions[i].references {
            if let Some(&t) = by_key.get(k) {
                referrers.entry(t).or_default().push(i);
            }
        }
    }
    for t in only {
        let (mut seen, mut stack) = (std::collections::HashSet::from([t]), vec![t]);
        while let Some(n) = stack.pop() {
            for &r in referrers.get(&n).into_iter().flatten() {
                if seen.insert(r) {
                    stack.push(r);
                }
            }
        }
        let direct = referrers.get(&t).map_or(0, |v| v.iter().collect::<std::collections::HashSet<_>>().len());
        let x = &jm.definitions[t];
        let (f, l, _) = span(t);
        let name = format!("{} ({}:{l})", qualified(x, x.name.as_deref().unwrap_or("?")), rel[f]);
        ledger.dependants.push((name, direct, seen.len() - 1));
    }
    Ok(ledger)
}

/// What a change to the world did to one of Sophia's facts (part 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldOutcome {
    /// Sophia's claim is the same and true before and after, and the
    /// caller's id did not move: nothing needed invalidating (knows).
    StillTrue,
    /// Julia's fact changed and Sophia's claim changed with it on
    /// re-ingest: a different outcome or target, true after the change, or
    /// a claim withdrawn (knows; the caller's id may move with it).
    Followed,
    /// True or unstated before, false after: the change made a fact wrong
    /// and Sophia still states it (wrong).
    MadeStale,
    /// Wrong before and after (part 3's wrongs).
    StillWrong,
    /// The claim is the same and true (or open) before and after, but the
    /// caller's id moved, through a definition it refers to by content
    /// (approximates: invalidated more than needed).
    IdMoved,
    /// Open (or unchecked) before and after, the id unmoved.
    StillOpen,
    /// Julia lists no use on one side (the method left the package, a
    /// use appeared): the oracle cannot compare, whatever Sophia states.
    Unchecked,
}

/// One (caller, name) of a world change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRow {
    pub file: String,
    pub line: usize,
    pub caller: String,
    pub name: String,
    pub before: Option<Outcome>,
    pub after: Option<Outcome>,
    pub outcome: WorldOutcome,
}

/// The comparison of one change.
#[derive(Debug, Default)]
pub struct WorldLedger {
    pub rows: Vec<WorldRow>,
    /// Whether Sophia ingested the change (it lies under the package's
    /// `src/`), or only Julia saw it (another package, `eval` at run time).
    pub ingested: bool,
    /// The package's definitions (outside the change) whose id differs
    /// after the change, by (file, first line, name).
    pub ids_moved: Vec<(String, usize, String)>,
    /// The part 3 ledgers before and after.
    pub before: Ledger,
    pub after: Ledger,
}

impl WorldLedger {
    pub fn tally(&self) -> BTreeMap<WorldOutcome, usize> {
        let mut t = BTreeMap::new();
        for r in &self.rows {
            *t.entry(r.outcome).or_default() += 1;
        }
        t
    }
}

fn knows(o: Option<Outcome>) -> bool {
    matches!(o, Some(Outcome::KnowsOnlyDefinition | Outcome::KnowsFunction | Outcome::KnowsBinding))
}

fn wrong(o: Option<Outcome>) -> bool {
    matches!(o, Some(Outcome::WrongOneOfMany | Outcome::WrongTarget | Outcome::WrongInterpolated))
}

/// Part 4 (world change): the package at `dir`, before and after `change`,
/// a Julia file evaluated after the package is loaded. `before` and
/// `after` are `calls.jl`'s output without and with it. A change under the
/// package's `src/` is evaluated into the package's top module (as if
/// included at the end of its root file), and Sophia ingests it; anywhere
/// else it is another package's code, which only Julia sees.
pub fn world_ledger(dir: &Path, change: &Path, before: &str, after: &str) -> Result<WorldLedger, ModuleError> {
    let canon = |p: &Path| p.canonicalize().map_err(|e| ModuleError::Io(format!("{}: {e}", p.display())));
    // Canonical paths, so that the files' names relative to `src/` match
    // Julia's however the arguments are written (review B-073, Codex 2).
    let (dir, change) = (canon(dir)?, canon(change)?);
    let (name, root) = module::package_root(&dir).ok_or_else(|| ModuleError::Io("no Project.toml and src/<name>.jl".into()))?;
    let src = dir.join("src");
    let d0 = module::discover(&root)?;
    let mut d1 = d0.clone();
    let ingested = change.starts_with(&src);
    // The change and the files it includes (review B-073, Codex 1), in the
    // package's top module.
    let mut changed_files: Vec<String> = Vec::new();
    if ingested {
        for mut f in module::discover(&change)?.files {
            changed_files.push(Path::new(&f.path).strip_prefix(&src).unwrap_or(Path::new(&f.path)).to_string_lossy().to_string());
            f.base_module.insert(0, name.clone());
            d1.files.push(f);
        }
    }
    let l0 = ledger_of(&d0, &src, before)?;
    let l1 = ledger_of(&d1, &src, after)?;
    let mut w = WorldLedger { ingested, ..Default::default() };
    for (k, id) in &l0.ids {
        if l1.ids.get(k).is_some_and(|i| i != id) {
            w.ids_moved.push(k.clone());
        }
    }
    let moved = |file: &str, line: usize, caller: &str| w.ids_moved.iter().any(|(f, l, c)| f == file && *l == line && c == caller);
    // Rows keyed by (file, line, caller, name, n-th): a name under two
    // bindings (`h`, `Inner.h`) gives two rows, paired in order.
    type Key = (String, usize, String, String, usize);
    // The outcome, and Sophia's side of the detail (its target), so that a
    // claim moved to another target with the same outcome is not "the same".
    let keyed = |l: &Ledger| -> BTreeMap<Key, (Outcome, String)> {
        let mut m = BTreeMap::new();
        let mut n: HashMap<(String, usize, String, String), usize> = HashMap::new();
        for r in &l.rows {
            let c = n.entry((r.file.clone(), r.line, r.caller.clone(), r.name.clone())).or_default();
            let target = if r.detail.starts_with("sophia ") { r.detail.split(" / ").next().unwrap_or("") } else { "" };
            m.insert((r.file.clone(), r.line, r.caller.clone(), r.name.clone(), *c), (r.outcome, target.to_string()));
            *c += 1;
        }
        m
    };
    let (k0, k1) = (keyed(&l0), keyed(&l1));
    let mut keys: Vec<&Key> = k0.keys().chain(k1.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut rows = Vec::new();
    for k in keys {
        // The change's own definitions are new code: part 3's subject.
        if changed_files.contains(&k.0) {
            continue;
        }
        let (b, a) = (k0.get(k), k1.get(k));
        let id_moved = moved(&k.0, k.1, &k.2);
        let (b_, a_) = (b.map(|x| x.0), a.map(|x| x.0));
        // Open (or unchecked) on both sides counts as the same: Sophia
        // states nothing either way.
        let stated = |o: Option<Outcome>| knows(o) || wrong(o);
        let same = b == a || (!stated(b_) && !stated(a_));
        let (b, a) = (b_, a_);
        let outcome = if a.is_none() || (b.is_none() && !wrong(a)) {
            // Julia lists no use on one side (review B-073, Codex 3).
            WorldOutcome::Unchecked
        } else if wrong(a) {
            if same { WorldOutcome::StillWrong } else { WorldOutcome::MadeStale }
        } else if same {
            match (knows(a), id_moved) {
                (_, true) => WorldOutcome::IdMoved,
                (true, false) => WorldOutcome::StillTrue,
                (false, false) => WorldOutcome::StillOpen,
            }
        } else {
            WorldOutcome::Followed
        };
        rows.push(WorldRow { file: k.0.clone(), line: k.1, caller: k.2.clone(), name: k.3.clone(), before: b, after: a, outcome });
    }
    w.rows = rows;
    w.before = l0;
    w.after = l1;
    Ok(w)
}

/// What part 5 (macros) found for one definition of Sophia's, or one thing
/// a module-level form that calls a macro made in Julia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MacroOutcome {
    /// Sophia's definition, and Julia's form made methods of that function
    /// at its lines (or the type, constant or macro it names).
    KnowsDefinition,
    /// As `KnowsDefinition`, but the definition is written with `$`: an
    /// `@eval` template, one definition for as many methods as the loop
    /// has values (approximates).
    Template,
    /// Sophia's definition, and Julia's form made nothing of it: a branch
    /// of `@static` not taken on this platform (approximates).
    NotMade,
    /// Julia's form made methods or a binding Sophia records no definition
    /// for (leaves open).
    Open,
    /// Constructors of a type the form defines, at the type's lines: the
    /// default ones Julia adds to every `struct`, and `@kwdef`'s keyword
    /// one; Sophia records the type, not its constructors (leaves open).
    OpenConstructors,
    /// A macro around a definition changes its lowered code, and Sophia's
    /// id changes when the macro is removed (knows).
    IdSeesMacro,
    /// A macro around a definition changes its lowered code, and Sophia's
    /// id is the same without the macro: two different definitions, one id
    /// (wrong).
    IdIgnoresMacro,
    /// The macro changes nothing, and neither does the id (knows).
    IdNoChange,
    /// The macro changes nothing, and the id changes (approximates: two
    /// ids for one definition).
    IdSplits,
}

/// One row of part 5: in the form at `file:line` that calls `macro_name`,
/// `name` (a function, type or binding, `Module.name`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroRow {
    pub file: String,
    pub line: usize,
    pub macro_name: String,
    pub name: String,
    pub outcome: MacroOutcome,
    pub detail: String,
}

/// The whole comparison of part 5.
#[derive(Debug, Default)]
pub struct MacroLedger {
    pub rows: Vec<MacroRow>,
    /// Julia's forms compared.
    pub forms: usize,
    /// Julia's forms at a line where Sophia's parse has no module-level
    /// form (`file`, `line`): not compared.
    pub unmatched: Vec<(String, usize)>,
    /// Lines with several forms, in Julia's output or Sophia's parse
    /// (`f(x) = 1; @m g(x) = 2`): not compared.
    pub ambiguous: Vec<(String, usize)>,
    /// Julia's methods made by the forms compared: inside one of Sophia's
    /// definitions (`KnowsDefinition`, `Template`), or left open.
    pub methods_seen: usize,
    pub methods_open: usize,
}

impl MacroLedger {
    pub fn tally(&self) -> BTreeMap<MacroOutcome, usize> {
        let mut t = BTreeMap::new();
        for r in &self.rows {
            *t.entry(r.outcome).or_default() += 1;
        }
        t
    }
}

/// One form of `macros.jl`'s output, with what it made.
#[derive(Default)]
struct JuliaForm {
    macro_name: String,
    changes: String,
    /// (owner, file, line) of each method.
    methods: Vec<(String, String, usize)>,
    /// (kind, `Module.name`) of each binding added.
    bindings: Vec<(String, String)>,
}

/// A module-level form of Sophia's parse: its span; for a macro call whose
/// last argument is a definition, the span of the macro call before the
/// definition (`@m args… `) and the definition's span.
struct SophiaForm {
    start: usize,
    end: usize,
    wraps: Option<((usize, usize), (usize, usize))>,
}

/// The module-level forms of a file, inside `module … end` too; a string
/// followed by another form is that form's docstring.
fn module_forms(root: moonkale_content_id::Syntax) -> Vec<SophiaForm> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let mut cursor = node.walk();
        let children: Vec<_> = node.named_children(&mut cursor).filter(|c| !c.kind().ends_with("comment")).collect();
        let mut doc: Option<usize> = None;
        for c in children {
            match c.kind() {
                "module_definition" => {
                    // A docstring before a module documents the module
                    // (review B-074, 9).
                    doc = None;
                    let mut k = c.walk();
                    stack.extend(c.named_children(&mut k).filter(|b| b.kind() == "block"));
                }
                "string_literal" if doc.is_none() => doc = Some(c.start_byte()),
                _ => {
                    let mut k = c.walk();
                    let args = (c.kind() == "macrocall_expression")
                        .then(|| c.named_children(&mut k).find(|a| a.kind() == "macro_argument_list"))
                        .flatten();
                    let def = args.and_then(|l| {
                        let mut k = l.walk();
                        l.named_children(&mut k).last()
                    });
                    let wraps = def
                        .filter(|d| moonkale_content_id::is_definition("julia", *d))
                        .map(|d| ((c.start_byte(), d.start_byte()), (d.start_byte(), d.end_byte())));
                    out.push(SophiaForm { start: doc.take().unwrap_or(c.start_byte()), end: c.end_byte(), wraps });
                }
            }
        }
    }
    out
}

/// The node with exactly this span (the outermost one).
fn node_at(tree: &moonkale_content_id::Tree, start: usize, end: usize) -> Option<moonkale_content_id::Syntax<'_>> {
    let mut n = tree.root_node().descendant_for_byte_range(start, end)?;
    while n.start_byte() != start || n.end_byte() != end {
        n = n.parent()?;
    }
    while let Some(p) = n.parent().filter(|p| p.start_byte() == start && p.end_byte() == end) {
        n = p;
    }
    Some(n)
}

/// Whether the definition node with this span is a template: it holds a
/// `$` splice outside a quote of its own (`@eval f(x::$T) = …`), not one
/// inside a string (`"…$x…"`, a string interpolation).
fn template_at(tree: &moonkale_content_id::Tree, start: usize, end: usize) -> bool {
    let Some(n) = node_at(tree, start, end) else { return false };
    let mut stack = vec![n];
    while let Some(x) = stack.pop() {
        match x.kind() {
            "interpolation_expression" => return true,
            "quote_expression" | "quote_statement" => continue,
            _ => {}
        }
        let mut c = x.walk();
        stack.extend(x.named_children(&mut c));
    }
    false
}

/// Compares the package at `dir` with the output of `macros.jl` for it
/// (part 5): for each module-level form that calls a macro, what Julia's
/// form made against the definitions Sophia records in it; and, for a
/// definition a macro wraps, whether Sophia's id sees the macro (the macro
/// is blanked out and the package ingested again).
pub fn macro_ledger(dir: &Path, truth: &str) -> Result<MacroLedger, ModuleError> {
    // Canonical first, so file names relative to `src/` match Julia's
    // however `dir` is written (review B-074, 1).
    let dir = &dir.canonicalize().map_err(|e| ModuleError::Io(format!("{}: {e}", dir.display())))?;
    let (_, root) = module::package_root(dir).ok_or_else(|| ModuleError::Io("no Project.toml and src/<name>.jl".into()))?;
    let src = dir.join("src");
    let d = module::discover(&root)?;
    let r = module::ingest(&d)?;
    let jm = &r.module;
    let rel: Vec<String> =
        d.files.iter().map(|f| Path::new(&f.path).strip_prefix(&src).unwrap_or(Path::new(&f.path)).to_string_lossy().to_string()).collect();
    let newlines: Vec<Vec<usize>> = d.files.iter().map(|f| f.text.match_indices('\n').map(|(i, _)| i).collect()).collect();
    let line = |f: usize, byte: usize| newlines[f].partition_point(|&i| i < byte) + 1;
    let mut trees = Vec::new();
    for f in &d.files {
        trees.push(moonkale_content_id::parse("julia", &f.text, &mut moonkale_content_id::Budget::new())?);
    }
    let forms: Vec<Vec<SophiaForm>> = trees.iter().map(|t| module_forms(t.root_node())).collect();
    // Sophia's view of every definition, refused ones too (kind, extended
    // function), as `module::ingest` derives it.
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
    for x in &jm.definitions {
        if let Some(def) = defs.get_mut(&(x.key.0 as usize, x.key.1, x.key.2)) {
            def.module = x.module.clone();
            crate::code::julia_extends(def, &scopes);
        }
    }

    // Julia's forms, by (file, line).
    let mut julia: BTreeMap<(String, usize), JuliaForm> = BTreeMap::new();
    let mut forms_at: HashMap<(String, usize), usize> = HashMap::new();
    for l in truth.lines() {
        let c: Vec<&str> = l.split('\t').collect();
        if c.len() < 5 {
            continue;
        }
        let key = (c[1].to_string(), c[2].parse::<usize>().unwrap_or(0));
        if c[0] == "form" {
            *forms_at.entry(key.clone()).or_default() += 1;
        }
        let e = julia.entry(key).or_default();
        match c[0] {
            "form" if c.len() == 6 => {
                e.macro_name = c[4].to_string();
                e.changes = c[5].to_string();
            }
            "method" => {
                let (mf, ml) = c[4].rsplit_once(':').unwrap_or((c[4], "0"));
                e.methods.push((c[3].to_string(), mf.to_string(), ml.parse().unwrap_or(0)));
            }
            "binding" => e.bindings.push((c[3].to_string(), c[4].to_string())),
            _ => {}
        }
    }

    // The same package with each macro call that wraps a definition blanked
    // out up to the definition (spaces, so every span stays where it was),
    // for the id comparison.
    let mut wrapped: Vec<(usize, (usize, usize))> = Vec::new();
    let mut ledger = MacroLedger::default();
    let mut matched: Vec<((String, usize), usize, usize)> = Vec::new(); // (key, file, form index)
    for (key, jf) in &julia {
        let found =
            rel.iter().position(|r| *r == key.0).and_then(|f| forms[f].iter().position(|s| line(f, s.start) == key.1).map(|i| (f, i)));
        let Some((f, i)) = found else {
            ledger.unmatched.push(key.clone());
            continue;
        };
        // Several forms on one line: Julia's line cannot tell them apart
        // (review B-074, 2).
        let sophia_here = forms[f].iter().filter(|s| line(f, s.start) == key.1).count();
        if forms_at.get(key).copied().unwrap_or(0) > 1 || sophia_here > 1 {
            ledger.ambiguous.push(key.clone());
            continue;
        }
        if matches!(jf.changes.as_str(), "changed" | "same") {
            if let Some((h, _)) = forms[f][i].wraps {
                wrapped.push((f, h));
            }
        }
        matched.push((key.clone(), f, i));
    }
    let mut blank = d.clone();
    for &(f, (a, b)) in &wrapped {
        let t = &mut blank.files[f].text;
        // One space per byte, so no offset moves (review B-074, 6).
        let spaces: String = t[a..b].chars().map(|c| if c == '\n' { "\n".to_string() } else { " ".repeat(c.len_utf8()) }).collect();
        t.replace_range(a..b, &spaces);
    }
    let blank_ids: HashMap<(u32, usize, usize), [u8; 32]> = if wrapped.is_empty() {
        HashMap::new()
    } else {
        module::ingest(&blank)?.module.definitions.iter().filter_map(|x| x.identity.as_ref().ok().map(|i| (x.key, i.digest))).collect()
    };

    let qualified = |m: &[String], name: &str| {
        let mut p = m.to_vec();
        p.push(name.to_string());
        p.join(".")
    };
    for ((file, at), f, i) in matched {
        let jf = &julia[&(file.clone(), at)];
        let form = &forms[f][i];
        ledger.forms += 1;
        ledger.methods_seen += jf.methods.len();
        let mut used_m = vec![false; jf.methods.len()];
        let mut used_b = vec![false; jf.bindings.len()];
        let mut row = |name: String, outcome, detail: String| {
            ledger.rows.push(MacroRow { file: file.clone(), line: at, macro_name: jf.macro_name.clone(), name, outcome, detail });
        };
        let mut inside: Vec<&moonkale_content_id::julia::JuliaDefinition> =
            jm.definitions.iter().filter(|x| x.key.0 as usize == f && form.start <= x.key.1 && x.key.2 <= form.end).collect();
        inside.sort_by_key(|x| x.key);
        for x in inside {
            let (a, b) = (x.key.1, x.key.2);
            let (l0, l1) = (line(f, a), line(f, b));
            let def = defs.get(&(f, a, b));
            let kind = def.map(|x| x.kind.clone()).or_else(|| node_at(&trees[f], a, b).map(|n| n.kind().to_string())).unwrap_or_default();
            // A macro is a function too: `@m` has one method per `macro m(…)`.
            let function = matches!(kind.as_str(), "function_definition" | "assignment" | "short_function_definition" | "macro_definition");
            let owner = match (def.and_then(|x| x.extends.clone()), &x.name) {
                (Some(p), _) => Some(p.join(".")),
                (None, Some(n)) if kind == "macro_definition" => Some(qualified(&x.module, &format!("@{}", n.trim_start_matches('@')))),
                (None, Some(n)) => Some(qualified(&x.module, n)),
                (None, None) => None,
            };
            let text = &d.files[f].text[a..b];
            let label = owner.clone().unwrap_or_else(|| text.lines().next().unwrap_or("").to_string());
            let refused = if x.identity.is_err() { ", refused" } else { "" };
            if function {
                // Julia's methods at the definition's lines, of its function
                // (or of a function of that name elsewhere: `Base.cbrt` is
                // `Base.Math.cbrt`, an alias, SP-013).
                let last = |o: &str| o.rsplit('.').next().unwrap_or(o).to_string();
                let mine = |o: &str| owner.as_deref().is_none_or(|w| w == o || last(w) == last(o));
                let mut n = 0;
                let mut alias = Vec::new();
                for (k, (o, mf, ml)) in jf.methods.iter().enumerate() {
                    if !used_m[k] && mine(o) && *mf == rel[f] && l0 <= *ml && *ml <= l1 {
                        if owner.as_deref().is_some_and(|w| w != o) && !alias.contains(o) {
                            alias.push(o.clone());
                        }
                        used_m[k] = true;
                        n += 1;
                        for (kb, (_, bn)) in jf.bindings.iter().enumerate() {
                            if bn == o {
                                used_b[kb] = true;
                            }
                        }
                    }
                }
                // `function f end`: no method, the binding is the definition.
                let bound = jf.bindings.iter().position(|(k, bn)| (k == "function" || k == "macro") && Some(bn) == owner.as_ref());
                if let Some(k) = bound {
                    used_b[k] = true;
                }
                // `function f end` declares the function and makes no method.
                let declaration = text.split_whitespace().count() == 3 && text.starts_with("function") && text.ends_with("end");
                let outcome = match n {
                    0 if declaration => MacroOutcome::KnowsDefinition,
                    0 => MacroOutcome::NotMade,
                    _ if template_at(&trees[f], a, b) => MacroOutcome::Template,
                    _ => MacroOutcome::KnowsDefinition,
                };
                let alias = if alias.is_empty() { String::new() } else { format!(" of {}", alias.join(", ")) };
                let what = if declaration && n == 0 { " (a declaration)".to_string() } else { String::new() };
                row(label.clone(), outcome, format!("sophia {kind} at {}:{l0}{refused}{what} / julia methods there: {n}{alias}", rel[f]));
            } else {
                let hit = jf.bindings.iter().position(|(_, bn)| Some(bn) == owner.as_ref());
                if let Some(k) = hit {
                    used_b[k] = true;
                }
                // A type or constant written with `$`: the loop binds one per value.
                let template = hit.is_none() && template_at(&trees[f], a, b);
                let mut made = 0;
                if template {
                    let want = if kind == "const_statement" { "value" } else { "type" };
                    for (k, (bk, _)) in jf.bindings.iter().enumerate() {
                        if !used_b[k] && bk == want {
                            used_b[k] = true;
                            made += 1;
                        }
                    }
                }
                let outcome = match (hit, made) {
                    (Some(_), _) => MacroOutcome::KnowsDefinition,
                    (None, 1..) => MacroOutcome::Template,
                    _ => MacroOutcome::NotMade,
                };
                row(label.clone(), outcome, format!("sophia {kind} at {}:{l0}{refused}", rel[f]));
            }
            // A definition the form's macro wraps: does the id see the macro?
            // The definition the form's macro call wraps (`@m args… def`).
            let direct = matches!(jf.changes.as_str(), "changed" | "same") && form.wraps.is_some_and(|(_, d)| d == (a, b));
            if let (true, Ok(id)) = (direct, &x.identity) {
                let same = blank_ids.get(&x.key) == Some(&id.digest);
                let outcome = match (jf.changes.as_str(), same) {
                    ("changed", true) => MacroOutcome::IdIgnoresMacro,
                    ("changed", false) => MacroOutcome::IdSeesMacro,
                    (_, true) => MacroOutcome::IdNoChange,
                    (_, false) => MacroOutcome::IdSplits,
                };
                let julia = if jf.changes == "changed" { "changes" } else { "does not change" };
                let sophia = if same { "is the same without it" } else { "changes without it" };
                row(label, outcome, format!("julia: the macro {julia} the lowered code; sophia's id {sophia}"));
            }
        }
        // What Julia's form made that no definition of Sophia's covers;
        // methods of a type defined in the form, at its lines, are its
        // constructors.
        let types: Vec<(String, usize, usize)> = jm
            .definitions
            .iter()
            .filter(|x| x.key.0 as usize == f && form.start <= x.key.1 && x.key.2 <= form.end)
            .filter(|x| defs.get(&(f, x.key.1, x.key.2)).is_some_and(|d| d.kind == "struct_definition"))
            .filter_map(|x| x.name.as_ref().map(|n| (qualified(&x.module, n), line(f, x.key.1), line(f, x.key.2))))
            .collect();
        let mut open: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for (k, (o, mf, ml)) in jf.methods.iter().enumerate() {
            if !used_m[k] {
                open.entry(o.as_str()).or_default().push(format!("{mf}:{ml}"));
            }
        }
        ledger.methods_open += open.values().map(Vec::len).sum::<usize>();
        for (o, at) in &open {
            for (kb, (_, bn)) in jf.bindings.iter().enumerate() {
                if bn == o {
                    used_b[kb] = true;
                }
            }
            let constructors = types.iter().any(|(t, l0, l1)| {
                t == o
                    && at.iter().all(|x| {
                        x.rsplit_once(':').is_some_and(|(mf, ml)| mf == rel[f] && ml.parse().is_ok_and(|l: usize| *l0 <= l && l <= *l1))
                    })
            });
            let outcome = if constructors { MacroOutcome::OpenConstructors } else { MacroOutcome::Open };
            row(o.to_string(), outcome, format!("julia methods: {} at {}", at.len(), at.join(",")));
        }
        for (kb, (kind, bn)) in jf.bindings.iter().enumerate() {
            if !used_b[kb] {
                row(bn.clone(), MacroOutcome::Open, format!("julia {kind}"));
            }
        }
    }
    Ok(ledger)
}

/// What part 6 (closures) found for one name inside one closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClosureOutcome {
    /// Julia captures the variable (a field of the closure type, or a
    /// static parameter of the enclosing method), and Sophia's encoding
    /// binds the name to a variable outside the closure (knows).
    KnowsCaptured,
    /// Julia: an argument or local variable of the closure; Sophia binds the
    /// name inside the closure (knows).
    KnowsOwn,
    /// Julia: a global binding; Sophia reads a module name (knows; which
    /// one is part 3's question).
    KnowsGlobal,
    /// Julia: a variable (captured or the closure's own); Sophia reads a
    /// module name, by content reference or unresolved (wrong).
    WrongGlobal,
    /// Julia: a global binding; Sophia binds the name as a local (wrong).
    WrongLocal,
    /// Julia captures the variable; Sophia makes it the closure's own, so
    /// the id does not link it to the variable outside (wrong).
    WrongCapturedAsOwn,
    /// Julia: the closure's own variable; Sophia binds it to a variable
    /// outside the closure (wrong).
    WrongOwnAsCaptured,
    /// Sophia reads some occurrences of the name as a module name and
    /// others as a local, where Julia has one variable (approximates; the
    /// detail says which).
    Mixed,
    /// The name is a local in Sophia's encoding, but its spelling is part
    /// of the encoding (a keyword parameter), so renaming cannot tell
    /// whether it is bound inside or outside the closure (leaves open).
    Undecided,
}

/// One row of part 6: in the definition `caller` at `file:line`, inside
/// the closure Julia calls `closure`, the name `name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureRow {
    pub file: String,
    pub line: usize,
    pub caller: String,
    pub closure: String,
    pub name: String,
    /// Julia's class: `captured`, `captured:box`, `captured:sparam`, `own`
    /// or `global`.
    pub julia: String,
    pub outcome: ClosureOutcome,
    pub detail: String,
}

/// The whole comparison of part 6.
#[derive(Debug, Default)]
pub struct ClosureLedger {
    pub rows: Vec<ClosureRow>,
    /// Julia's closure methods read, and those matched to a closure in
    /// Sophia's parse.
    pub julia_closures: usize,
    pub matched: usize,
    /// Julia closures inside a definition Sophia refuses (no id: an inner
    /// `function … end`, `global` in a body), left open: (file, line,
    /// closure).
    pub refused: Vec<(String, usize, String)>,
    /// Julia closures inside no definition of Sophia's (macro output).
    pub outside: Vec<(String, usize, String)>,
    /// Julia closures in a definition where no closure of Sophia's parse
    /// matches (line and argument names), or several do.
    pub unmatched: Vec<(String, usize, String)>,
    pub ambiguous: Vec<(String, usize, String)>,
    /// Keyword arguments of a call whose name is not in the encoding: the
    /// normaliser reads `atol = x` in a call's argument list as an
    /// assignment to a local, so `f(1; atol = x)` and `f(1; rtol = x)`
    /// share an id (wrong). In every identified definition, not only in
    /// closures: (file, line, definition, name).
    pub keywords_unspelled: Vec<(String, usize, String, String)>,
    /// Names in a matched closure that Julia's lowered closure does not
    /// list (field and keyword names, names lowering folds away): not
    /// compared.
    pub unlisted: usize,
    /// Lines of the truth that are not a closure line (not 8 columns):
    /// a truncated or foreign output, not compared (review B-076, 4).
    pub malformed: usize,
}

impl ClosureLedger {
    pub fn tally(&self) -> BTreeMap<ClosureOutcome, usize> {
        let mut t = BTreeMap::new();
        for r in &self.rows {
            *t.entry(r.outcome).or_default() += 1;
        }
        t
    }
}

/// One closure in Julia's lowered code (a line of `closures.jl`), merged
/// over the closure types Julia makes for one source closure (a keyword
/// body, default arguments).
#[derive(Default, Clone)]
struct JuliaClosure {
    names: Vec<String>,
    args: Vec<String>,
    captured: BTreeMap<String, String>,
    locals: Vec<String>,
    globals: BTreeMap<String, String>,
}

fn list(s: &str) -> Vec<String> {
    if s == "-" { Vec::new() } else { s.split(',').map(str::to_string).collect() }
}

/// The names a parameter list (or a `for` binding) binds.
fn binders(node: moonkale_content_id::Syntax, text: &str, out: &mut Vec<String>) {
    let mut c = node.walk();
    let children: Vec<_> = node.named_children(&mut c).collect();
    match node.kind() {
        // `_` binds nothing (Julia leaves the argument unnamed).
        "identifier" if &text[node.byte_range()] != "_" => out.push(text[node.byte_range()].to_string()),
        "typed_expression" | "assignment" | "splat_expression" => {
            if let Some(first) = children.first().filter(|f| f.kind() != "operator") {
                binders(*first, text, out);
            }
        }
        "argument_list" | "tuple_expression" | "parenthesized_expression" => {
            for ch in children {
                binders(ch, text, out);
            }
        }
        _ => {}
    }
}

/// The argument list of a call head, through `where` and a return type.
fn call_args(mut node: moonkale_content_id::Syntax) -> Option<moonkale_content_id::Syntax> {
    loop {
        match node.kind() {
            "call_expression" => {
                let mut c = node.walk();
                return node.named_children(&mut c).find(|a| a.kind() == "argument_list");
            }
            "where_expression" | "typed_expression" | "signature" => node = node.named_child(0)?,
            _ => return None,
        }
    }
}

/// A closure of Sophia's parse: its node, the line Julia gives its method,
/// and the names its parameters bind. Inner named functions, anonymous
/// functions, `do` blocks, generators and comprehensions.
struct SophiaClosure<'t> {
    node: moonkale_content_id::Syntax<'t>,
    generator: bool,
    params: Vec<String>,
}

fn closures_in<'t>(def: moonkale_content_id::Syntax<'t>, text: &str) -> Vec<SophiaClosure<'t>> {
    let mut out = Vec::new();
    let mut stack: Vec<_> = {
        let mut c = def.walk();
        def.named_children(&mut c).collect()
    };
    while let Some(n) = stack.pop() {
        let mut c = n.walk();
        let children: Vec<_> = n.named_children(&mut c).collect();
        let mut params = Vec::new();
        let found = match n.kind() {
            "arrow_function_expression" => {
                if let Some(p) = children.first() {
                    binders(*p, text, &mut params);
                }
                Some(false)
            }
            "do_clause" => {
                for ch in children.iter().filter(|ch| ch.kind() != "block") {
                    binders(*ch, text, &mut params);
                }
                Some(false)
            }
            "function_definition" => {
                if let Some(a) = children.first().and_then(|s| call_args(*s)) {
                    binders(a, text, &mut params);
                }
                Some(false)
            }
            "assignment" if moonkale_content_id::is_definition("julia", n) => {
                if let Some(a) = children.first().and_then(|s| call_args(*s)) {
                    binders(a, text, &mut params);
                }
                Some(false)
            }
            "generator" | "comprehension_expression" => {
                let mut stack = children.clone();
                while let Some(x) = stack.pop() {
                    if x.kind() == "for_binding" {
                        if let Some(first) = x.named_child(0) {
                            binders(first, text, &mut params);
                        }
                    } else if x.kind() == "for_clause" {
                        let mut k = x.walk();
                        stack.extend(x.named_children(&mut k));
                    }
                }
                Some(true)
            }
            _ => None,
        };
        if let Some(generator) = found {
            out.push(SophiaClosure { node: n, generator, params });
        }
        stack.extend(children);
    }
    out.sort_by_key(|c| c.node.start_byte());
    out
}

/// Whether an identifier is a name the normaliser keeps as written rather
/// than as a variable: a field after `.`, a keyword argument's name in a
/// call (`f(x; atol = atol)`, the first `atol`).
fn spelled(node: moonkale_content_id::Syntax) -> bool {
    let Some(p) = node.parent() else { return false };
    if p.kind() == "field_expression" {
        return p.named_child(0).is_some_and(|f| f.id() != node.id());
    }
    if p.kind() == "assignment" && p.named_child(0).is_some_and(|f| f.id() == node.id()) {
        if let Some(list) = p.parent().filter(|l| l.kind() == "argument_list") {
            // A call's argument list, not a signature's or a closure's
            // parameters.
            if let Some(call) = list.parent().filter(|c| c.kind() == "call_expression") {
                let mut head = call;
                while let Some(up) = head.parent().filter(|u| matches!(u.kind(), "where_expression" | "typed_expression")) {
                    head = up;
                }
                let signature = head.parent().is_some_and(|u| {
                    u.kind() == "signature"
                        || (u.kind() == "assignment"
                            && u.named_child(0).is_some_and(|f| f.id() == head.id())
                            && moonkale_content_id::is_definition("julia", u))
                });
                return !signature;
            }
        }
    }
    false
}

/// Identifier nodes under `node` that name variables or module bindings
/// (not field or keyword names), in source order.
fn identifiers<'t>(node: moonkale_content_id::Syntax<'t>) -> Vec<moonkale_content_id::Syntax<'t>> {
    let mut out = Vec::new();
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if n.kind() == "identifier" {
            if !spelled(n) {
                out.push(n);
            }
            continue;
        }
        if matches!(n.kind(), "quote_expression" | "quote_statement" | "string_literal" | "macro_identifier") {
            continue;
        }
        let mut c = n.walk();
        stack.extend(n.named_children(&mut c));
    }
    out.sort_by_key(|n| n.start_byte());
    out
}

/// The keyword names of calls under `node` (`atol` in `f(x; atol = 1)`).
fn keyword_names<'t>(node: moonkale_content_id::Syntax<'t>) -> Vec<moonkale_content_id::Syntax<'t>> {
    let mut out = Vec::new();
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if n.kind() == "identifier" {
            if spelled(n) && n.parent().is_some_and(|p| p.kind() == "assignment") {
                out.push(n);
            }
            continue;
        }
        if matches!(n.kind(), "quote_expression" | "quote_statement" | "string_literal" | "macro_identifier") {
            continue;
        }
        let mut c = n.walk();
        stack.extend(n.named_children(&mut c));
    }
    out
}

/// A definition's encoding as bytes, from its own text (no references).
fn encoding(text: &str) -> Option<Vec<Vec<u8>>> {
    let mut budget = moonkale_content_id::Budget::new();
    let tree = moonkale_content_id::parse("julia", text, &mut budget).ok()?;
    let root = tree.root_node();
    let node = root.named_child(0)?;
    let parts = moonkale_content_id::definition("julia", node, text, &HashMap::<(usize, usize), u8>::new(), &mut budget).ok()?;
    parts
        .into_iter()
        .map(|p| match p {
            moonkale_content_id::Part::Bytes(b) => Some(b),
            moonkale_content_id::Part::Reference(_) => None,
        })
        .collect()
}

/// `text` with each span replaced by `with` (spans sorted, not overlapping).
fn replaced(text: &str, spans: &[(usize, usize)], with: &str) -> String {
    let mut s = spans.to_vec();
    s.sort_unstable();
    let mut out = String::new();
    let mut at = 0;
    for (a, b) in s {
        out.push_str(&text[at..a]);
        out.push_str(with);
        at = b;
    }
    out.push_str(&text[at..]);
    out
}

/// Compares the package at `dir` with the output of `closures.jl` for it
/// (part 6): for each closure Julia's lowering made, each name in the
/// matching closure of Sophia's parse, classified by Julia (captured, the
/// closure's own, global) and by Sophia's encoding of the definition. A
/// name is a module name in Sophia's encoding when the normaliser reads it
/// as non-local (`definition_and_uses`); otherwise it is local, and it is
/// the closure's own when renaming it inside the closure leaves the
/// definition's encoding unchanged (alpha-renaming), captured when it
/// changes it but renaming it in the whole definition does not.
pub fn closure_ledger(dir: &Path, truth: &str) -> Result<ClosureLedger, ModuleError> {
    let dir = &dir.canonicalize().map_err(|e| ModuleError::Io(format!("{}: {e}", dir.display())))?;
    let (_, root) = module::package_root(dir).ok_or_else(|| ModuleError::Io("no Project.toml and src/<name>.jl".into()))?;
    let src = dir.join("src");
    let d = module::discover(&root)?;
    let r = module::ingest(&d)?;
    let jm = &r.module;
    let rel: Vec<String> =
        d.files.iter().map(|f| Path::new(&f.path).strip_prefix(&src).unwrap_or(Path::new(&f.path)).to_string_lossy().to_string()).collect();
    let newlines: Vec<Vec<usize>> = d.files.iter().map(|f| f.text.match_indices('\n').map(|(i, _)| i).collect()).collect();
    let line = |f: usize, byte: usize| newlines[f].partition_point(|&i| i < byte) + 1;
    let mut trees = Vec::new();
    for f in &d.files {
        trees.push(moonkale_content_id::parse("julia", &f.text, &mut moonkale_content_id::Budget::new())?);
    }

    // Julia's closures, by (file, line, placed), in output order.
    let mut ledger_malformed = 0;
    let mut julia: Vec<((String, usize, bool), JuliaClosure)> = Vec::new();
    for l in truth.lines() {
        let c: Vec<&str> = l.split('\t').collect();
        if c.len() != 8 {
            ledger_malformed += usize::from(!l.trim().is_empty());
            continue;
        }
        let mut j = JuliaClosure { names: vec![c[2].to_string()], args: list(c[4]), locals: list(c[6]), ..Default::default() };
        for x in list(c[5]) {
            let (n, how) = x.split_once(':').map_or((x.as_str(), ""), |(n, h)| (n, h));
            j.captured.insert(n.to_string(), if how.is_empty() { "captured".into() } else { format!("captured:{how}") });
        }
        for x in list(c[7]) {
            if let Some((n, b)) = x.split_once('=') {
                j.globals.insert(n.to_string(), b.to_string());
            }
        }
        julia.push(((c[0].to_string(), c[1].parse().unwrap_or(0), c[3] == "creator"), j));
    }

    let identified: Vec<usize> = (0..jm.definitions.len()).filter(|&i| jm.definitions[i].identity.is_ok()).collect();
    // The outermost identified definitions around a line: several when
    // definitions share it (`h(a) = …; k(b) = …`).
    let around = |file: &str, at: usize| -> Vec<usize> {
        let inside: Vec<usize> = identified
            .iter()
            .copied()
            .filter(|&i| {
                let k = jm.definitions[i].key;
                let f = k.0 as usize;
                rel[f] == file && line(f, k.1) <= at && at <= line(f, k.2)
            })
            .collect();
        let within = |i: usize, o: usize| {
            let (a, b) = (jm.definitions[i].key, jm.definitions[o].key);
            i != o && a.0 == b.0 && b.1 <= a.1 && a.2 <= b.2
        };
        inside.iter().copied().filter(|&i| !inside.iter().any(|&o| within(i, o))).collect()
    };

    let mut ledger = ClosureLedger { julia_closures: julia.len(), malformed: ledger_malformed, ..Default::default() };
    // Julia's closures merged per Sophia closure: (definition, closure start) → info.
    let mut merged: BTreeMap<(usize, usize), (String, usize, JuliaClosure)> = BTreeMap::new();
    // Julia's closures with the same place and arguments, in the order
    // lowering numbered them (`#f##0…` before `#f##2…`): lowering numbers
    // closures in source order, an outer one before those inside it.
    let gensym = |n: &str| n.split_once("##").and_then(|(_, r)| r.split(|c: char| !c.is_ascii_digit()).next()?.parse::<usize>().ok());
    let peers = |key: &(String, usize, bool), args: &[String]| {
        let mut p: Vec<usize> = julia
            .iter()
            .filter(|(k, o)| k == key && o.args.iter().collect::<std::collections::BTreeSet<_>>() == args.iter().collect())
            .map(|(_, o)| gensym(&o.names[0]).unwrap_or(usize::MAX))
            .collect();
        p.sort_unstable();
        p
    };
    for (key, j) in &julia {
        let (file, at, creator) = key;
        let tag = (file.clone(), *at, j.names[0].clone());
        let outer = around(file, *at);
        if outer.is_empty() {
            let refused = jm.definitions.iter().any(|x| {
                let f = x.key.0 as usize;
                x.identity.is_err() && rel[f] == *file && line(f, x.key.1) <= *at && *at <= line(f, x.key.2)
            });
            if refused {
                ledger.refused.push(tag)
            } else {
                ledger.outside.push(tag)
            };
            continue;
        }
        // Julia's line is the closure's first line, or for a `do` block the
        // first line of its body (review B-076, 1): the closures at that
        // line, else the innermost one around it. Looked for in each
        // outermost definition around the line; when several have one,
        // Julia's line cannot tell them apart (review B-076, 2).
        let mut found: Vec<(usize, Vec<SophiaClosure>)> = Vec::new();
        for &i in &outer {
            let k = jm.definitions[i].key;
            let f = k.0 as usize;
            let Some(node) = node_at(&trees[f], k.1, k.2) else { continue };
            let text = &d.files[f].text;
            let kind: Vec<SophiaClosure> = closures_in(node, text).into_iter().filter(|c| c.generator == *creator).collect();
            let julia_line = |c: &SophiaClosure| {
                let body = (c.node.kind() == "do_clause")
                    .then(|| {
                        let mut w = c.node.walk();
                        let b = c.node.named_children(&mut w).find(|b| b.kind() == "block");
                        b.and_then(|b| b.named_child(0))
                    })
                    .flatten();
                line(f, body.map_or(c.node.start_byte(), |b| b.start_byte()))
            };
            let lines = |c: &SophiaClosure| (line(f, c.node.start_byte()), line(f, c.node.end_byte()));
            let starting = kind.iter().filter(|c| *creator || julia_line(c) == *at).count();
            let least =
                kind.iter().filter(|c| lines(c).0 <= *at && *at <= lines(c).1).map(|c| c.node.end_byte() - c.node.start_byte()).min();
            let mut candidates: Vec<SophiaClosure> = Vec::new();
            for c in kind {
                let (a, b) = lines(&c);
                let take = if starting > 0 {
                    *creator || julia_line(&c) == *at
                } else {
                    a <= *at && *at <= b && Some(c.node.end_byte() - c.node.start_byte()) == least
                };
                if take {
                    candidates.push(c);
                }
            }
            if !candidates.is_empty() {
                found.push((i, candidates));
            }
        }
        if found.len() > 1 {
            ledger.ambiguous.push(tag);
            continue;
        }
        let Some((i, candidates)) = found.pop() else {
            ledger.unmatched.push(tag);
            continue;
        };
        let args: std::collections::BTreeSet<&String> = j.args.iter().collect();
        // The same parameters; a destructured argument (`((a, b), c) -> …`)
        // is unnamed in Julia, its names are locals.
        let exact: Vec<&SophiaClosure> = candidates
            .iter()
            .filter(|c| {
                let params: std::collections::BTreeSet<&String> = c.params.iter().collect();
                params == args
                    || (!params.is_empty() && args.is_subset(&params) && params.iter().all(|p| j.locals.contains(*p) || args.contains(p)))
            })
            .collect();
        let wider: Vec<&SophiaClosure> = candidates.iter().filter(|c| args.iter().all(|a| c.params.contains(a))).collect();
        // Several closures of Sophia's and as many of Julia's with these
        // arguments at this place: paired in source order.
        let p = peers(key, &j.args);
        let rank = p.iter().position(|&g| Some(g) == gensym(&j.names[0]));
        let chosen = match (exact.len(), wider.len()) {
            (1, _) => exact[0],
            (0, 1) => wider[0],
            (n, _) if n > 1 && n == p.len() && rank.is_some() => exact[rank.unwrap_or(0)],
            (0, 0) => {
                ledger.unmatched.push(tag);
                continue;
            }
            _ => {
                ledger.ambiguous.push(tag);
                continue;
            }
        };
        ledger.matched += 1;
        let e = merged.entry((i, chosen.node.start_byte())).or_insert_with(|| (file.clone(), *at, JuliaClosure::default()));
        e.2.names.extend(j.names.iter().cloned());
        e.2.args.extend(j.args.iter().cloned());
        e.2.locals.extend(j.locals.iter().cloned());
        e.2.captured.extend(j.captured.clone());
        e.2.globals.extend(j.globals.clone());
    }

    // Per definition: which identifiers its encoding reads as module names
    // (keyword names included, which the normaliser should keep as
    // written).
    let mut hits_of: HashMap<usize, std::collections::HashSet<(usize, usize)>> = HashMap::new();
    for &i in &identified {
        let k = jm.definitions[i].key;
        let f = k.0 as usize;
        let text = &d.files[f].text;
        let Some(node) = node_at(&trees[f], k.1, k.2) else { continue };
        let mut probes: std::collections::HashSet<(usize, usize)> =
            identifiers(node).iter().map(|n| (n.start_byte(), n.end_byte())).collect();
        let keywords = keyword_names(node);
        probes.extend(keywords.iter().map(|n| (n.start_byte(), n.end_byte())));
        let refs: HashMap<(usize, usize), u8> = HashMap::new();
        let hits: std::collections::HashSet<(usize, usize)> =
            moonkale_content_id::definition_and_uses("julia", node, text, &refs, &probes, &mut moonkale_content_id::Budget::new())
                .map(|(_, h)| h.into_iter().collect())
                .unwrap_or_default();
        // A keyword name the encoding keeps is read as a name (a hit);
        // otherwise it is a local or a binding, and its spelling is lost.
        for n in keywords {
            if !hits.contains(&(n.start_byte(), n.end_byte())) {
                let name = jm.definitions[i].name.clone().unwrap_or_else(|| "?".into());
                ledger.keywords_unspelled.push((rel[f].clone(), line(f, n.start_byte()), name, text[n.byte_range()].to_string()));
            }
        }
        hits_of.insert(i, hits);
    }
    for ((i, start), (file, at, j)) in merged {
        let x = &jm.definitions[i];
        let k = x.key;
        let f = k.0 as usize;
        let text = &d.files[f].text;
        let Some(node) = node_at(&trees[f], k.1, k.2) else { continue };
        let all = identifiers(node);
        let Some(hits) = hits_of.get(&i) else { continue };
        let Some(closure) = closures_in(node, text).into_iter().find(|c| c.node.start_byte() == start) else { continue };
        let (cs, ce) = (closure.node.start_byte(), closure.node.end_byte());
        let def_text = &text[k.1..k.2];
        let original = encoding(def_text);
        let content: std::collections::HashSet<(usize, usize)> = x.references.iter().map(|(s, _)| *s).collect();
        let caller = x.name.clone().unwrap_or_else(|| "?".into());
        let closure_name = j.names.iter().find(|n| !n.is_empty()).cloned().unwrap_or_default();
        // Names in the closure, in order of first occurrence.
        let mut names: Vec<String> = Vec::new();
        for n in identifiers(closure.node) {
            let s = text[n.byte_range()].to_string();
            if !names.contains(&s) {
                names.push(s);
            }
        }
        for name in names {
            let class = if let Some(c) = j.captured.get(&name) {
                c.clone()
            } else if j.args.contains(&name) || j.locals.contains(&name) {
                "own".to_string()
            } else if j.globals.contains_key(&name) {
                "global".to_string()
            } else {
                ledger.unlisted += 1;
                continue;
            };
            let spans = |a: usize, b: usize| -> Vec<(usize, usize)> {
                all.iter()
                    .filter(|n| a <= n.start_byte() && n.end_byte() <= b && text[n.byte_range()] == *name)
                    .map(|n| (n.start_byte(), n.end_byte()))
                    .collect()
            };
            let inside = spans(cs, ce);
            let module_uses = inside.iter().filter(|s| hits.contains(s)).count();
            let fresh = {
                let mut f = format!("ζ{name}ζ");
                while def_text.contains(&f) {
                    f.push('ζ');
                }
                f
            };
            let local_spans = |v: Vec<(usize, usize)>| -> Vec<(usize, usize)> {
                v.into_iter().filter(|s| !hits.contains(s)).map(|(a, b)| (a - k.1, b - k.1)).collect()
            };
            let sophia = if module_uses == inside.len() {
                "global"
            } else if module_uses > 0 {
                "mixed"
            } else if original.is_none() {
                // The definition's text does not encode on its own: the
                // rename test cannot run (review B-076, 5).
                "undecided"
            } else {
                let same_inside = encoding(&replaced(def_text, &local_spans(inside.clone()), &fresh)) == original;
                let same_everywhere = encoding(&replaced(def_text, &local_spans(spans(k.1, k.2)), &fresh)) == original;
                match (same_inside, same_everywhere) {
                    (true, _) => "own",
                    (false, true) => "captured",
                    (false, false) => "undecided",
                }
            };
            let outcome = match (class.split(':').next().unwrap_or(""), sophia) {
                ("captured", "captured") => ClosureOutcome::KnowsCaptured,
                ("own", "own") => ClosureOutcome::KnowsOwn,
                ("global", "global") => ClosureOutcome::KnowsGlobal,
                (_, "global") => ClosureOutcome::WrongGlobal,
                ("global", _) if sophia != "mixed" && sophia != "undecided" => ClosureOutcome::WrongLocal,
                ("captured", "own") => ClosureOutcome::WrongCapturedAsOwn,
                ("own", "captured") => ClosureOutcome::WrongOwnAsCaptured,
                (_, "mixed") => ClosureOutcome::Mixed,
                _ => ClosureOutcome::Undecided,
            };
            let referenced = inside.iter().any(|s| content.contains(s));
            let how = match sophia {
                "global" | "mixed" if referenced => format!("{sophia} (a content reference)"),
                _ => sophia.to_string(),
            };
            let julia_detail = j.globals.get(&name).map(|b| format!(" {b}")).unwrap_or_default();
            ledger.rows.push(ClosureRow {
                file: file.clone(),
                line: at,
                caller: caller.clone(),
                closure: closure_name.clone(),
                name: name.clone(),
                julia: class.clone(),
                outcome,
                detail: format!(
                    "julia {class}{julia_detail} / sophia {how}, {} of {} uses read as module names",
                    module_uses,
                    inside.len()
                ),
            });
        }
    }
    Ok(ledger)
}

/// What part 7 (mutable globals) found for one use of a global binding of
/// the package in one definition, or for one binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GlobalOutcome {
    /// Julia: a constant; Sophia: a content reference to its `const`
    /// definition (a use), or a `const` definition with an id (a binding).
    KnowsConstant,
    /// Julia: a mutable global; Sophia states no content reference and no
    /// generic use, so the id holds only the name's spelling, as it must:
    /// the value is state, read when the code runs (a use); or no
    /// definition with an id for it (a binding).
    KnowsState,
    /// Julia: a constant whose value is a mutable object (`Dict`, `Ref`);
    /// Sophia: a content reference to its definition. The binding is
    /// content, what the object holds is state (approximates).
    ApproxMutableObject,
    /// Julia: a constant; Sophia states nothing (a qualified name, a
    /// constant defined twice, `const a, b = …`) (leaves open).
    OpenConstant,
    /// The definition using the binding has no id (`global x = v` in a
    /// body is refused) (leaves open).
    OpenRefused,
    /// Julia: the method writes the binding (`Core.setglobal!`,
    /// `setproperty!` on a module); Sophia's id holds the code, and no fact
    /// says it changes state (leaves open).
    OpenWrite,
    /// Julia: a mutable global; Sophia: a content reference to a
    /// definition of it, so the id claims a value the binding need not hold
    /// when the code runs (wrong). Also for a binding: a mutable global
    /// Sophia gives a definition with an id.
    WrongContentOfMutable,
    /// Julia: a mutable global; Sophia: a generic use, as if the name were
    /// a function with several methods (wrong).
    WrongGenericOfMutable,
    /// Julia: an argument or local of the method spelled like a data
    /// binding; Sophia's encoding reads it as a local too (no content
    /// reference, no generic use).
    KnowsLocal,
    /// Julia: an argument or local of the method; Sophia: a content
    /// reference to (or a generic use of) the module's binding of that
    /// name, so the id depends on a definition the method does not use
    /// (wrong).
    WrongLocalAsGlobal,
    /// Sophia: a content reference to a global binding Julia's lowered code
    /// of that definition does not read.
    NoJuliaUse,
}

/// One row of part 7: in the definition `caller` at `file:line`, the
/// binding `binding` (`Glob.level`), read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalRow {
    pub file: String,
    pub line: usize,
    pub caller: String,
    pub binding: String,
    /// `read`, `write`, or `local` (Julia: an argument or local of the
    /// method spelled like the binding).
    pub access: String,
    /// Julia's kind of the binding: `const`, `global` or `declared`.
    pub julia: String,
    pub outcome: GlobalOutcome,
    pub detail: String,
}

/// One global binding of the package that holds data (a constant whose
/// value is not a function, type or module, or any mutable global), and
/// the definitions Sophia identifies for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingRow {
    pub binding: String,
    /// `const`, `global` or `declared`.
    pub julia: String,
    /// Sophia's identified definitions of that name in that module, by
    /// their form:
    /// `const` (a `const` statement) or `assignment` (`x::T = v`, which
    /// the normaliser reads as a short function form).
    pub sophia: Vec<String>,
    pub outcome: GlobalOutcome,
}

/// The whole comparison of part 7.
#[derive(Debug, Default)]
pub struct GlobalLedger {
    pub rows: Vec<GlobalRow>,
    pub bindings: Vec<BindingRow>,
    /// Julia uses at lines inside no definition of Sophia's (macro output),
    /// and at lines where several definitions start and end: not compared.
    pub outside: Vec<(String, usize, String)>,
    pub ambiguous: Vec<(String, usize, String)>,
    /// Truth lines that are neither a `binding` (5 columns) nor a `use`
    /// (7 columns) line: not compared.
    pub malformed: usize,
}

impl GlobalLedger {
    pub fn tally(&self) -> BTreeMap<GlobalOutcome, usize> {
        let mut t = BTreeMap::new();
        for r in &self.rows {
            *t.entry(r.outcome).or_default() += 1;
        }
        t
    }
    pub fn binding_tally(&self) -> BTreeMap<GlobalOutcome, usize> {
        let mut t = BTreeMap::new();
        for r in &self.bindings {
            *t.entry(r.outcome).or_default() += 1;
        }
        t
    }
}

/// Compares the package at `dir` with the output of `globals.jl` for it
/// (part 7): for each global binding of the package that holds data,
/// whether Sophia gives it a definition with an id, and for each use of one
/// in a method, whether Sophia's id treats it as content (a content
/// reference), as a generic function, or as state (only the spelling).
/// Julia's kinds are `const`, `global` (mutable) and `declared`.
pub fn global_ledger(dir: &Path, truth: &str) -> Result<GlobalLedger, ModuleError> {
    let dir = &dir.canonicalize().map_err(|e| ModuleError::Io(format!("{}: {e}", dir.display())))?;
    let (_, root) = module::package_root(dir).ok_or_else(|| ModuleError::Io("no Project.toml and src/<name>.jl".into()))?;
    let src = dir.join("src");
    let d = module::discover(&root)?;
    let r = module::ingest(&d)?;
    let jm = &r.module;
    let rel: Vec<String> =
        d.files.iter().map(|f| Path::new(&f.path).strip_prefix(&src).unwrap_or(Path::new(&f.path)).to_string_lossy().to_string()).collect();
    let newlines: Vec<Vec<usize>> = d.files.iter().map(|f| f.text.match_indices('\n').map(|(i, _)| i).collect()).collect();
    let line = |f: usize, byte: usize| newlines[f].partition_point(|&i| i < byte) + 1;
    let span = |i: usize| {
        let k = jm.definitions[i].key;
        (k.0 as usize, line(k.0 as usize, k.1), line(k.0 as usize, k.2))
    };
    let qualified = |x: &moonkale_content_id::julia::JuliaDefinition, name: &str| {
        let mut p = x.module.clone();
        p.push(name.to_string());
        p.join(".")
    };
    let by_key: HashMap<_, usize> = jm.definitions.iter().enumerate().map(|(i, x)| (x.key, i)).collect();

    // Julia's side.
    let mut ledger = GlobalLedger::default();
    let mut kinds: BTreeMap<String, (String, bool)> = BTreeMap::new();
    let mut uses: Vec<(String, usize, String, String, String, String)> = Vec::new();
    let mut conflicting: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    const KINDS: [&str; 3] = ["const", "global", "declared"];
    for l in truth.lines() {
        let c: Vec<&str> = l.split('\t').collect();
        match (c.first().copied(), c.len()) {
            (Some("binding"), 5) if KINDS.contains(&c[2]) && ["mutable", "immutable", "-"].contains(&c[4]) => {
                let k = (c[2].to_string(), c[4] == "mutable");
                // The same binding with two kinds: Julia's side contradicts itself.
                if kinds.get(c[1]).is_some_and(|old| *old != k) {
                    conflicting.insert(c[1].to_string());
                }
                kinds.insert(c[1].to_string(), k);
            }
            (Some("use"), 7) if ["read", "write", "local"].contains(&c[5]) && KINDS.contains(&c[6]) => {
                let Ok(at) = c[2].parse::<usize>() else {
                    ledger.malformed += 1;
                    continue;
                };
                let u = (c[1].to_string(), at, c[3].to_string(), c[4].to_string(), c[5].to_string(), c[6].to_string());
                if !uses.contains(&u) {
                    uses.push(u);
                }
            }
            _ => ledger.malformed += 1,
        }
    }
    for b in &conflicting {
        kinds.remove(b);
        ledger.malformed += 1;
    }

    // Sophia's identified definitions of each binding, by form (a refused
    // one has no id, so it claims nothing: review B-078, 5).
    let mut units: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut unit_binding: HashMap<usize, String> = HashMap::new();
    for (i, x) in jm.definitions.iter().enumerate() {
        let Some(name) = &x.name else { continue };
        if x.identity.is_err() {
            continue;
        }
        let b = qualified(x, name);
        if !kinds.contains_key(&b) {
            continue;
        }
        let text = &d.files[x.key.0 as usize].text[x.key.1..x.key.2];
        let form = if text.starts_with("const") { "const" } else { "assignment" };
        units.entry(b.clone()).or_default().push(form.to_string());
        unit_binding.insert(i, b);
    }
    for (b, (k, _)) in &kinds {
        let sophia = units.get(b).cloned().unwrap_or_default();
        let outcome = match (k.as_str(), sophia.is_empty()) {
            ("const", false) if sophia.iter().all(|s| s == "const") => GlobalOutcome::KnowsConstant,
            ("const", true) => GlobalOutcome::OpenConstant,
            (_, true) => GlobalOutcome::KnowsState,
            _ => GlobalOutcome::WrongContentOfMutable,
        };
        ledger.bindings.push(BindingRow { binding: b.clone(), julia: k.clone(), sophia, outcome });
    }

    // Each use, under the innermost definition (identified or not) whose
    // lines contain the method's line. A local that Julia also reads as the
    // global in the same method (a closure reading it, say) is not
    // compared: Sophia's claim may be about either.
    let read_too: std::collections::HashSet<(&str, usize, &str)> =
        uses.iter().filter(|u| u.4 == "read").map(|u| (u.0.as_str(), u.1, u.3.as_str())).collect();
    let mut matched: std::collections::HashSet<(usize, String)> = std::collections::HashSet::new();
    // Whether definition `a` lies inside definition `o` (bytes, one file).
    let inside = |a: usize, o: usize| {
        let (ka, ko) = (jm.definitions[a].key, jm.definitions[o].key);
        a != o && ka.0 == ko.0 && ko.1 <= ka.1 && ka.2 <= ko.2
    };
    for (file, at, _name, b, access, kind) in &uses {
        if access == "local" && read_too.contains(&(file.as_str(), *at, b.as_str())) {
            continue;
        }
        // A use of a binding with no `binding` line, or with another kind.
        let Some((julia, mutable_object)) = kinds.get(b).cloned().filter(|k| k.0 == *kind) else {
            ledger.malformed += 1;
            continue;
        };
        let mut within: Vec<usize> = (0..jm.definitions.len())
            .filter(|&i| {
                let (f, a, e) = span(i);
                rel[f] == *file && a <= *at && *at <= e
            })
            .collect();
        // The innermost: those containing no other candidate. Several (on
        // one line, nested or not) are not compared, and their claims about
        // `b` are not reported as `NoJuliaUse` (review B-078, 2 and 3).
        let all = within.clone();
        within.retain(|&o| !all.iter().any(|&a| inside(a, o)));
        if within.len() > 1 {
            ledger.ambiguous.push((file.clone(), *at, b.clone()));
            for &i in &within {
                matched.insert((i, b.clone()));
            }
            continue;
        }
        let Some(&i) = within.first() else {
            ledger.outside.push((file.clone(), *at, b.clone()));
            continue;
        };
        let x = &jm.definitions[i];
        let (f, first, _) = span(i);
        let caller = x.name.clone().unwrap_or_else(|| "?".into());
        let mut row = |outcome, detail: String| {
            ledger.rows.push(GlobalRow {
                file: rel[f].clone(),
                line: first,
                caller: caller.clone(),
                binding: b.clone(),
                access: access.clone(),
                julia: julia.clone(),
                outcome,
                detail,
            });
        };
        if x.identity.is_err() {
            row(GlobalOutcome::OpenRefused, format!("{:?}", x.identity.as_ref().err()));
            continue;
        }
        if access == "write" {
            row(GlobalOutcome::OpenWrite, String::new());
            continue;
        }
        // Sophia's claim about `b` in this definition.
        let content: Vec<usize> =
            x.references.iter().filter_map(|(_, t)| by_key.get(t).copied()).filter(|t| unit_binding.get(t) == Some(b)).collect();
        let short = b.rsplit('.').next().unwrap_or(b);
        // Inside the definition's bytes, not its lines: another definition
        // may share its last line (review B-078, 4).
        let generic = jm
            .calls_generic
            .iter()
            .any(|(gf, s, n)| *gf == x.key.0 && n == short && qualified(x, n) == *b && x.key.1 <= s.0 && s.1 <= x.key.2);
        let (outcome, detail) = if access == "local" {
            matched.insert((i, b.clone()));
            match (content.first(), generic) {
                (Some(&t), _) => (GlobalOutcome::WrongLocalAsGlobal, format!("content reference to {}:{}", rel[span(t).0], span(t).1)),
                (None, true) => (GlobalOutcome::WrongLocalAsGlobal, "generic use".to_string()),
                (None, false) => (GlobalOutcome::KnowsLocal, "local".to_string()),
            }
        } else if let Some(&t) = content.first() {
            matched.insert((i, b.clone()));
            let (tf, tl, _) = span(t);
            let to = format!("content reference to {}:{tl}", rel[tf]);
            match julia.as_str() {
                "const" if mutable_object => (GlobalOutcome::ApproxMutableObject, to),
                "const" => (GlobalOutcome::KnowsConstant, to),
                _ => (GlobalOutcome::WrongContentOfMutable, to),
            }
        } else if generic {
            (GlobalOutcome::WrongGenericOfMutable, "generic use".to_string())
        } else if julia == "const" {
            (GlobalOutcome::OpenConstant, "spelling only".to_string())
        } else {
            (GlobalOutcome::KnowsState, "spelling only".to_string())
        };
        row(outcome, detail);
    }
    // Content references to a data binding Julia's method does not read.
    // In a definition that is itself a data binding (`const B = 2 * A`)
    // there is no method: Julia evaluated it once, at load, so only the
    // target's kind is compared.
    for (i, x) in jm.definitions.iter().enumerate() {
        if x.identity.is_err() {
            continue;
        }
        let mut targets: Vec<&String> = x.references.iter().filter_map(|(_, t)| by_key.get(t).and_then(|t| unit_binding.get(t))).collect();
        targets.sort_unstable();
        targets.dedup();
        for b in targets {
            if matched.contains(&(i, b.clone())) {
                continue;
            }
            let (f, first, _) = span(i);
            let (julia, mutable_object) = kinds[b].clone();
            let (outcome, detail) = match (unit_binding.contains_key(&i), julia.as_str()) {
                (false, _) => (GlobalOutcome::NoJuliaUse, "content reference"),
                (true, "const") if mutable_object => (GlobalOutcome::ApproxMutableObject, "content reference in an initializer"),
                (true, "const") => (GlobalOutcome::KnowsConstant, "content reference in an initializer"),
                (true, _) => (GlobalOutcome::WrongContentOfMutable, "content reference in an initializer"),
            };
            ledger.rows.push(GlobalRow {
                file: rel[f].clone(),
                line: first,
                caller: x.name.clone().unwrap_or_else(|| "?".into()),
                binding: b.clone(),
                access: "read".into(),
                julia,
                outcome,
                detail: detail.into(),
            });
        }
    }
    ledger.rows.sort_by(|a, b| (&a.file, a.line, &a.binding, &a.access).cmp(&(&b.file, b.line, &b.binding, &b.access)));
    Ok(ledger)
}

/// Source parser agreement, independently of evaluation and macro expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseOutcome {
    Knows,
    Approximates,
    LeavesOpen,
    Wrong,
}

/// A definition matched by source file and the byte offset of its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRow {
    pub file: String,
    pub line: usize,
    pub name_byte: usize,
    pub name: String,
    pub outcome: ParseOutcome,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct ParseLedger {
    /// Oracle files not reached by literal include discovery; not compared.
    pub undiscovered: Vec<String>,
    pub rows: Vec<ParseRow>,
    pub malformed: usize,
    /// JuliaSyntax diagnostics, kept apart from tree-sitter disagreements.
    pub julia_errors: Vec<(String, usize, usize)>,
}

impl ParseLedger {
    pub fn tally(&self) -> BTreeMap<ParseOutcome, usize> {
        let mut tally = BTreeMap::new();
        for row in &self.rows {
            *tally.entry(row.outcome).or_default() += 1;
        }
        tally
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParseFacts {
    form: String,
    name: String,
    npos: Option<usize>,
    kwnames: String,
    callkw: usize,
}

fn parse_children(n: moonkale_content_id::Syntax<'_>) -> Vec<moonkale_content_id::Syntax<'_>> {
    let mut cursor = n.walk();
    n.named_children(&mut cursor).collect()
}

fn parse_signature(mut n: moonkale_content_id::Syntax<'_>) -> moonkale_content_id::Syntax<'_> {
    while matches!(n.kind(), "signature" | "where_expression" | "typed_expression") {
        let Some(c) = n.named_child(0) else { break };
        n = c;
    }
    n
}

fn parse_keyword_name(n: moonkale_content_id::Syntax<'_>, src: &str) -> String {
    let mut n = n;
    while !matches!(n.kind(), "identifier" | "operator") {
        let Some(c) = n.named_child(0) else { break };
        n = c;
    }
    src[n.byte_range()].to_string()
}

/// The byte offset of the last component of a written name: `Base.show`
/// → `show`, `Base.:+` → `+`, `Base.:(==)` → `==`. A dot inside the last component (`.+`, an
/// operator) is kept when nothing precedes it.
fn last_component_offset(written: &str) -> usize {
    match written.rfind('.') {
        Some(i) if i > 0 => {
            let rest = &written[i + 1..];
            // `Base.:+` → `+`; `Base.:(==)` → `==`.
            let quoted = if rest.starts_with(":(") && rest.len() > 2 {
                2
            } else if rest.starts_with(':') && rest.len() > 1 {
                1
            } else {
                0
            };
            i + 1 + quoted
        }
        _ => 0,
    }
}

/// The last component of a name as Sophia records it (`Base.show` → `show`).
fn last_component(name: &str) -> &str {
    let at = last_component_offset(name);
    let rest = &name[at..];
    if name[..at].ends_with(":(") { rest.strip_suffix(')').unwrap_or(rest) } else { rest }
}

fn parse_callkw(n: moonkale_content_id::Syntax<'_>) -> usize {
    // Deliberately count the grammar's keyword nodes, not assignments:
    // treating an assignment as a keyword would conceal the known loss.
    let mut count = 0;
    let children = parse_children(n);
    let mut stack = if matches!(n.kind(), "function_definition" | "macro_definition" | "assignment") {
        children.into_iter().skip(1).collect()
    } else {
        children
    };
    while let Some(c) = stack.pop() {
        if moonkale_content_id::is_definition("julia", c) {
            continue;
        }
        if c.kind() == "named_argument" && c.parent().is_some_and(|p| p.kind() == "argument_list") {
            count += 1;
        }
        stack.extend(parse_children(c));
    }
    count
}

fn parse_facts(d: &crate::code::Def, n: moonkale_content_id::Syntax<'_>, src: &str) -> ParseFacts {
    let form = match d.kind.as_str() {
        "function_definition" => "function",
        "assignment" => "short",
        "macro_definition" => "macro",
        "struct_definition" => "struct", // This kind also covers mutable_struct.
        "abstract_definition" => "abstract",
        "primitive_definition" => "primitive",
        "module_definition" => "module",
        "const_statement" => "const",
        other => other,
    };
    let method = matches!(form, "function" | "short" | "macro");
    let mut npos = method.then_some(0);
    let mut names = Vec::new();
    if method {
        let sig = parse_signature(n.named_child(0).unwrap_or(n));
        let args = parse_children(sig).into_iter().find(|c| c.kind() == "argument_list");
        if let Some(args) = args {
            let mut keyword = false;
            let mut cursor = args.walk();
            for c in args.children(&mut cursor) {
                if c.kind() == ";" {
                    keyword = true;
                } else if c.is_named() {
                    if keyword {
                        names.push(parse_keyword_name(c, src));
                    } else {
                        *npos.as_mut().unwrap() += 1;
                    }
                }
            }
        }
    }
    names.sort();
    ParseFacts {
        form: form.into(),
        name: if d.name.is_empty() { "-".into() } else { d.name.clone() },
        npos,
        kwnames: if names.is_empty() { "-".into() } else { names.join(",") },
        callkw: parse_callkw(n),
    }
}

/// Compare JuliaSyntax's source facts with file-mode identities from the pinned
/// Moonkale parser. Truth offsets are zero based; last_byte is inclusive.
/// Only files reached by literal package includes are compared.
pub fn parse_ledger(dir: &Path, truth: &str) -> Result<ParseLedger, ModuleError> {
    use moonkale_content_id::{Budget, parse};
    use std::collections::{BTreeSet, HashSet};
    let dir = dir.canonicalize().map_err(|e| ModuleError::Io(e.to_string()))?;
    let (_, root) = module::package_root(&dir).ok_or_else(|| ModuleError::Io("no Project.toml and src/<name>.jl".into()))?;
    let discovered = module::discover(&root)?;
    let srcdir = dir.join("src");
    let mut ledger = ParseLedger::default();
    let mut oracle = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    const FORMS: [&str; 10] =
        ["function", "short", "macro", "struct", "mutable_struct", "abstract", "primitive", "const", "typed_global", "module"];
    for line in truth.lines() {
        let c: Vec<_> = line.split('\t').collect();
        let valid = || -> Option<_> {
            if c.len() != 10 || c[0] != "def" || !FORMS.contains(&c[5]) || c[1].is_empty() || c[6].is_empty() {
                return None;
            }
            let (at, first, last) = (c[2].parse::<usize>().ok()?, c[3].parse::<usize>().ok()?, c[4].parse::<usize>().ok()?);
            if first > at || at > last {
                return None;
            }
            if let Some(file) =
                discovered.files.iter().find(|f| Path::new(&f.path).strip_prefix(&srcdir).is_ok_and(|p| p.to_string_lossy() == c[1]))
            {
                if last >= file.text.len() || !file.text.is_char_boundary(at) || !file.text.is_char_boundary(first) {
                    return None;
                }
            }
            let npos = if c[7] == "-" { None } else { Some(c[7].parse::<usize>().ok()?) };
            if matches!(c[5], "function" | "short" | "macro") != npos.is_some() {
                return None;
            }
            if c[8].is_empty() {
                return None;
            }
            let kws: Vec<_> = c[8].split(',').collect();
            if kws.iter().any(|k| k.is_empty()) || (kws.len() > 1 && kws.contains(&"-")) || kws.windows(2).any(|w| w[0] >= w[1]) {
                return None;
            }
            Some((
                (c[1].to_string(), at),
                (first, last, ParseFacts { form: c[5].into(), name: c[6].into(), npos, kwnames: c[8].into(), callkw: c[9].parse().ok()? }),
            ))
        };
        if c.len() == 4 && c[0] == "error" {
            match (c[2].parse::<usize>(), c[3].parse::<usize>()) {
                (Ok(first), Ok(last)) if !c[1].is_empty() && first <= last.saturating_add(1) => {
                    ledger.julia_errors.push((c[1].into(), first, last));
                }
                _ => ledger.malformed += 1,
            }
        } else if let Some((key, facts)) = valid() {
            if oracle.insert(key.clone(), facts).is_some() {
                duplicates.insert(key);
                ledger.malformed += 1;
            }
        } else {
            ledger.malformed += 1;
        }
    }
    for key in duplicates {
        oracle.remove(&key);
    }
    let mut matched = HashSet::new();
    let mut reached = HashSet::new();
    for file in &discovered.files {
        let rel = Path::new(&file.path).strip_prefix(&srcdir).unwrap_or(Path::new(&file.path)).to_string_lossy().to_string();
        reached.insert(rel.clone());
        let tree = parse("julia", &file.text, &mut Budget::new())?;
        let found = crate::code::definitions("julia", tree.root_node(), &file.text);
        let identified = crate::code::file("julia", &file.text)?;
        let ids: HashSet<_> = identified.defs.iter().map(|d| (d.start, d.end)).collect();
        let refused: HashMap<_, _> = identified.refused.iter().map(|(d, why)| ((d.start, d.end), why)).collect();
        let mut errors = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(n) = stack.pop() {
            if n.is_error() || n.is_missing() {
                errors.push((n.start_byte(), n.end_byte()));
            }
            let mut cursor = n.walk();
            stack.extend(n.children(&mut cursor));
        }
        let row = |at, name, outcome, detail| ParseRow {
            file: rel.clone(),
            line: file.text[..at].bytes().filter(|&b| b == b'\n').count() + 1,
            name_byte: at,
            name,
            outcome,
            detail,
        };
        for (d, n) in &found {
            // JuliaSyntax names `Base.show` by its last component; Moonkale's
            // name span covers the whole qualified name. Join on the last one.
            let at = d.name_span.map(|(a, b)| a + last_component_offset(&file.text[a..b])).unwrap_or_else(|| {
                let sig = parse_signature(n.named_child(0).unwrap_or(*n));
                let mut head = sig.named_child(0).unwrap_or(sig);
                while head.kind() == "parenthesized_expression" {
                    let Some(child) = head.named_child(0) else { break };
                    head = child;
                }
                head.start_byte()
            });
            let key = (rel.clone(), at);
            let Some((_, _, julia)) = oracle.get(&key) else {
                if ids.contains(&(d.start, d.end)) {
                    ledger.rows.push(row(
                        at,
                        d.name.clone(),
                        ParseOutcome::Wrong,
                        "extra: identified definition absent from JuliaSyntax".into(),
                    ));
                }
                continue;
            };
            matched.insert(key);
            if let Some(why) = refused.get(&(d.start, d.end)) {
                ledger.rows.push(row(at, julia.name.clone(), ParseOutcome::LeavesOpen, format!("refused: {why}")));
                continue;
            }
            let sophia = parse_facts(d, *n, &file.text);
            let mut differences = Vec::new();
            let approximate = d.kind == "struct_definition" && matches!(julia.form.as_str(), "struct" | "mutable_struct");
            if sophia.form != julia.form && !approximate {
                differences.push(format!("form: Julia={} Sophia={}", julia.form, sophia.form));
            }
            // A head Moonkale's naming leaves unnamed (`(\\)(F, B)`,
            // `Tridiagonal{T}(A)`) states no name: open, not a disagreement.
            let unnamed = sophia.name == "-" && julia.name != "-";
            if !unnamed && last_component(&sophia.name) != julia.name {
                differences.push(format!("name: Julia={} Sophia={}", julia.name, sophia.name));
            }
            if sophia.npos != julia.npos {
                differences.push(format!("npos: Julia={:?} Sophia={:?}", julia.npos, sophia.npos));
            }
            if sophia.kwnames != julia.kwnames {
                differences.push(format!("kwnames: Julia={} Sophia={}", julia.kwnames, sophia.kwnames));
            }
            if sophia.callkw != julia.callkw {
                differences.push(format!("callkw: Julia={} Sophia={}", julia.callkw, sophia.callkw));
            }
            if errors.iter().any(|&(a, b)| {
                ((a == b && d.start <= a && a < d.end) || (a < d.end && d.start < b))
                    && !ledger.julia_errors.iter().any(|(f, x, y)| *f == rel && *x <= b && a <= y.saturating_add(1))
            }) {
                differences.push("tree-sitter ERROR/MISSING without JuliaSyntax error".into());
            }
            let outcome = if !differences.is_empty() {
                ParseOutcome::Wrong
            } else if unnamed {
                differences.push("unnamed: Moonkale's naming gives this head no name".into());
                ParseOutcome::LeavesOpen
            } else if approximate {
                differences.push("struct_definition covers struct and mutable_struct".into());
                ParseOutcome::Approximates
            } else {
                ParseOutcome::Knows
            };
            ledger.rows.push(row(at, julia.name.clone(), outcome, differences.join("; ")));
        }
        for ((f, at), (_, _, facts)) in &oracle {
            if *f == rel && !matched.contains(&(f.clone(), *at)) {
                let detail = "missed: JuliaSyntax definition absent from Sophia".to_string();
                ledger.rows.push(row(*at, facts.name.clone(), ParseOutcome::Wrong, detail));
                matched.insert((f.clone(), *at));
            }
        }
    }
    // Oracle files outside the include graph are not claims about discovered files.
    // Report them explicitly, rather than folding them into the parser tally.
    for (file, _) in oracle.keys().filter(|(file, _)| !reached.contains(file)) {
        if !ledger.undiscovered.contains(file) {
            ledger.undiscovered.push(file.clone());
        }
    }
    ledger.rows.sort_by(|a, b| (&a.file, a.name_byte, &a.name).cmp(&(&b.file, b.name_byte, &b.name)));
    Ok(ledger)
}
