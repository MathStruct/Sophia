//! Derived programs with an input address before they are built (experiment
//! E-5): the output of `tests/julia-semantics/derivatives.jl` (for each
//! function of a package variant and an argument type, the pieces of the
//! derived program Zygote builds, and the rule tables read from Julia's method
//! table) against three candidate keys for id(D(f)), each computable without
//! running D. A key is **sound** if two rows with equal keys have equal
//! derived programs, and **precise** to the extent that equal programs get
//! equal keys. See `derivative.md` and [[Semantic Boundary Experiments]] §E-5.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use sophia_hash::Digest;
use sophia_hash::canonical::Writer as Canonical;

use crate::expansion::owners;
use crate::module::{self, ModuleError};

/// A method of one of the rule tables whose signature names one of the
/// package's functions (`rule` rows).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuleFact {
    /// `rrule`, `frule`, `no_rrule`, `no_frule` or `_pullback`.
    pub table: String,
    /// The module that defined the method.
    pub module: String,
    pub sig: String,
    /// Digest of its lowered body.
    pub body: String,
}

/// One gradient: a function of the package at one argument type, in one
/// variant (`label`), and the derived program Zygote built for it.
#[derive(Debug, Clone)]
pub struct Derived {
    pub label: String,
    pub target: String,
    pub argtype: String,
    /// Whether the walk named every instance that runs (`all`); not if it
    /// met a call it could not infer (`dynamic`) or stopped at its limit.
    pub complete: bool,
    /// The first gradient call (transformation, compilation, one run).
    pub ns_first: u64,
    /// Finding the reached instances (inference of each `_pullback`).
    pub ns_walk: u64,
    /// Zygote's generators for every reached instance, its decomposition
    /// cache emptied first: the transformation alone.
    pub ns_gen: u64,
    /// The same on the filled cache.
    pub ns_cached: u64,
    pub gradient: String,
    /// Per reached `_pullback` instance: its signature, the kind of its piece
    /// (`ir`, `rule`, `expr`, `dynamic`) and the digest of the piece's text.
    pub pieces: Vec<(String, String, String)>,
}

impl Derived {
    /// The derived program's digest: its root (the gradient's own
    /// instance, the walk's first) and its pieces as a set. Without the root,
    /// two functions that reach the same instances (`p` and `q` calling each
    /// other) would have one program.
    pub fn output(&self) -> Digest {
        let mut p: Vec<_> = self.pieces.iter().collect();
        p.sort();
        let mut c = Canonical::new();
        c.str("root", self.pieces.first().map(|x| x.0.as_str()).unwrap_or(""));
        for (sig, kind, d) in p {
            let o = c.open("piece");
            c.str("sig", sig);
            c.str("kind", kind);
            c.str("text", d);
            c.close(o);
        }
        Digest::of(&c.finish())
    }
}

/// What `derivatives.jl` printed, for any number of variants.
#[derive(Debug, Default)]
pub struct Oracle {
    /// Per label: `julia` and each package, with its version, as the run
    /// that printed the label declared them.
    pub versions: BTreeMap<String, BTreeSet<(String, String)>>,
    /// Labels printed by runs that declared different versions.
    pub inconsistent: BTreeSet<String>,
    /// Per label: each rule table's (name, number of methods, digest).
    pub tables: BTreeMap<String, Vec<(String, usize, String)>>,
    pub rules: BTreeMap<String, Vec<RuleFact>>,
    pub derived: Vec<Derived>,
    /// (label, what was called, how often the counting rule ran).
    pub calls: Vec<(String, String, u64)>,
    /// Piece texts by digest.
    pub texts: HashMap<String, String>,
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

pub fn parse_derivatives(tsv: &str) -> Oracle {
    let mut o = Oracle::default();
    let mut at: HashMap<(String, String, String), usize> = HashMap::new();
    // The versions of the run being read: a `julia` line starts a run.
    let mut current: BTreeSet<(String, String)> = BTreeSet::new();
    for line in tsv.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if let ["table" | "rule" | "derived" | "instance" | "calls", label, ..] = f.as_slice() {
            match o.versions.get(*label) {
                None => {
                    o.versions.insert(label.to_string(), current.clone());
                }
                Some(v) if *v != current => {
                    o.inconsistent.insert(label.to_string());
                }
                Some(_) => {}
            }
        }
        match f.as_slice() {
            ["julia", v] => {
                current = BTreeSet::from([("julia".to_string(), v.to_string())]);
            }
            ["package", p, v] => {
                current.insert((p.to_string(), v.to_string()));
            }
            ["table", label, name, n, d] => {
                o.tables.entry(label.to_string()).or_default().push((name.to_string(), n.parse().unwrap_or(0), d.to_string()))
            }
            ["rule", label, table, module, sig, body] => o.rules.entry(label.to_string()).or_default().push(RuleFact {
                table: table.to_string(),
                module: module.to_string(),
                sig: unescape(sig),
                body: body.to_string(),
            }),
            ["derived", label, target, ty, _n, all, first, walk, generate, cached, grad] => {
                at.insert((label.to_string(), target.to_string(), ty.to_string()), o.derived.len());
                o.derived.push(Derived {
                    label: label.to_string(),
                    target: target.to_string(),
                    argtype: ty.to_string(),
                    complete: *all == "all",
                    ns_first: first.parse().unwrap_or(0),
                    ns_walk: walk.parse().unwrap_or(0),
                    ns_gen: generate.parse().unwrap_or(0),
                    ns_cached: cached.parse().unwrap_or(0),
                    gradient: grad.to_string(),
                    pieces: Vec::new(),
                });
            }
            ["instance", label, target, ty, sig, kind, d] => {
                if let Some(&i) = at.get(&(label.to_string(), target.to_string(), ty.to_string())) {
                    o.derived[i].pieces.push((unescape(sig), kind.to_string(), d.to_string()));
                }
            }
            ["calls", label, n] => o.calls.push((label.to_string(), "rows".into(), n.parse().unwrap_or(0))),
            ["calls", label, what, n] => o.calls.push((label.to_string(), what.to_string(), n.parse().unwrap_or(0))),
            ["piece", d, text] => {
                o.texts.insert(d.to_string(), unescape(text));
            }
            _ => {}
        }
    }
    o
}

/// The three candidate keys, from least to most of the environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// From the store alone: D's versions and mode, the argument type, the
    /// ids of f's closure in the package (every method of every function it
    /// uses, transitively), and the package's own rule definitions for those
    /// functions.
    Store,
    /// `Store` plus the rule methods of every loaded module whose signature
    /// names a function of the closure (method-table facts, citing a world).
    Rules,
    /// `Store` plus a digest of each whole rule table.
    Tables,
}

pub const KEYS: [Key; 3] = [Key::Store, Key::Rules, Key::Tables];

/// One gradient with its keys.
#[derive(Debug, Clone)]
pub struct Row {
    pub label: String,
    pub target: String,
    pub argtype: String,
    pub output: Digest,
    pub keys: BTreeMap<Key, Digest>,
    /// The package functions in f's closure (last name component).
    pub closure: BTreeSet<String>,
    /// The number of definitions in the closure.
    pub definitions: usize,
    /// The ids of f's own methods (or `refused:` and a digest), sorted.
    pub roots: Vec<String>,
    /// The ids of the closure's definitions, sorted.
    pub closure_ids: Vec<String>,
    /// The package's rule definitions the `Store` key covers, as `owner`.
    pub package_rules: usize,
    pub gradient: String,
    /// Whether the oracle named every instance of the program.
    pub complete: bool,
}

#[derive(Debug, Default)]
pub struct DerivativeLedger {
    pub rows: Vec<Row>,
}

/// Per key, over pairs of rows with complete programs:
/// `unsound`, equal keys with different programs (E-5's failure condition);
/// `missed`, different keys with equal programs (reuse lost).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Score {
    pub pairs: usize,
    pub unsound: Vec<(String, String, String, String)>,
    pub missed: usize,
}

impl DerivativeLedger {
    pub fn row(&self, label: &str, target: &str, argtype: &str) -> Option<&Row> {
        self.rows.iter().find(|r| r.label == label && r.target == target && r.argtype == argtype)
    }

    /// Scores a key over every pair of rows whose programs are complete
    /// (any two functions: a key must tell functions apart too).
    pub fn score(&self, key: Key) -> Score {
        let mut s = Score::default();
        let rows: Vec<&Row> = self.rows.iter().filter(|r| r.complete).collect();
        for (i, a) in rows.iter().enumerate() {
            for b in &rows[i + 1..] {
                s.pairs += 1;
                let same_key = a.keys[&key] == b.keys[&key];
                let same_out = a.output == b.output;
                if same_key && !same_out {
                    s.unsound.push((a.label.clone(), b.label.clone(), a.target.clone(), a.argtype.clone()));
                }
                if !same_key && same_out {
                    s.missed += 1;
                }
            }
        }
        s
    }

    /// The (target, type) whose derived program differs from `base`'s in
    /// variant `label` (complete programs only).
    pub fn changed(&self, base: &str, label: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for r in self.rows.iter().filter(|r| r.label == label && r.complete) {
            if let Some(b) = self.row(base, &r.target, &r.argtype)
                && b.output != r.output
            {
                out.push((r.target.clone(), r.argtype.clone()));
            }
        }
        out
    }

    /// The (target, type) whose `key` differs from `base`'s in `label`
    /// (complete programs only).
    pub fn rekeyed(&self, key: Key, base: &str, label: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for r in self.rows.iter().filter(|r| r.label == label && r.complete) {
            if let Some(b) = self.row(base, &r.target, &r.argtype)
                && b.keys[&key] != r.keys[&key]
            {
                out.push((r.target.clone(), r.argtype.clone()));
            }
        }
        out
    }
}

/// A package's definitions as E-5 needs them: by span, the function each
/// defines, its id, its source text, the definitions it references and the
/// generic functions it uses.
struct Package {
    defs: BTreeMap<(usize, usize, usize), Def>,
}

struct Def {
    owner: Option<String>,
    /// The id, or `refused:` and the digest of the source text.
    id: String,
    text: String,
    refs: Vec<(usize, usize, usize)>,
    generic: Vec<String>,
}

fn package(dir: &Path) -> Result<Package, ModuleError> {
    let (_, root) = module::package_root(dir).ok_or_else(|| ModuleError::Io(format!("no package at {}", dir.display())))?;
    let d = module::discover(&root)?;
    let (seen, _, r) = owners(&d)?;
    let mut defs = BTreeMap::new();
    for x in &r.module.definitions {
        let key = (x.key.0 as usize, x.key.1, x.key.2);
        let Some(s) = seen.get(&key) else { continue };
        let text = d.files[key.0].text[key.1..key.2].to_string();
        let id = match s.id {
            Some(i) => i.to_string(),
            None => format!("refused:{}", Digest::of(text.as_bytes())),
        };
        let generic = s.id.and_then(|i| r.uses_generic.get(&i)).map(|v| v.iter().map(|p| p.join(".")).collect()).unwrap_or_default();
        let refs = x.references.iter().map(|(_, k)| (k.0 as usize, k.1, k.2)).collect();
        defs.insert(key, Def { owner: s.owner.clone(), id, text, refs, generic });
    }
    Ok(Package { defs })
}

fn last(o: &str) -> &str {
    o.rsplit('.').next().unwrap_or(o)
}

const RULE_TABLES: [&str; 5] = ["rrule", "frule", "no_rrule", "no_frule", "_pullback"];

impl Package {
    /// The definitions of `f`'s closure: its methods, what they reference,
    /// and every method of every generic function they use, transitively.
    fn closure(&self, module: &str, f: &str) -> BTreeSet<(usize, usize, usize)> {
        let start = format!("{module}.{f}");
        let mut todo: Vec<_> = self.defs.iter().filter(|(_, d)| d.owner.as_deref() == Some(&start)).map(|(k, _)| *k).collect();
        let mut seen = BTreeSet::new();
        while let Some(k) = todo.pop() {
            if !seen.insert(k) {
                continue;
            }
            let Some(d) = self.defs.get(&k) else { continue };
            todo.extend(d.refs.iter().copied());
            for g in &d.generic {
                todo.extend(self.defs.iter().filter(|(_, x)| x.owner.as_deref() == Some(g.as_str())).map(|(k, _)| *k));
            }
        }
        seen
    }

    /// The package's rule definitions (methods of a rule table) whose text
    /// names one of `names` as `typeof(name)`.
    fn rules_for(&self, names: &BTreeSet<String>) -> Vec<&Def> {
        self.defs
            .values()
            .filter(|d| d.owner.as_deref().is_some_and(|o| RULE_TABLES.contains(&last(o))))
            .filter(|d| names.iter().any(|n| d.text.contains(&format!("typeof({n})"))))
            .collect()
    }
}

/// Whether a rule fact's signature names `typeof(<module>.<n>)` for one of
/// `names`.
fn names_one(sig: &str, module: &str, names: &BTreeSet<String>) -> bool {
    names.iter().any(|n| sig.contains(&format!("typeof(Main.{module}.{n})")))
}

/// Computes the keys of every gradient of `tsv`. `sources` maps each label
/// to the package directory its source is (a session label to the variant
/// whose source the session ended with); `module` is the package's name.
pub fn derivative_ledger(module: &str, sources: &[(&str, PathBuf)], tsv: &str) -> Result<(Oracle, DerivativeLedger), ModuleError> {
    let o = parse_derivatives(tsv);
    let mut packages = HashMap::new();
    for (label, dir) in sources {
        packages.insert(label.to_string(), package(dir)?);
    }
    let mut ledger = DerivativeLedger::default();
    for d in &o.derived {
        let Some(p) = packages.get(&d.label) else { continue };
        let closure = p.closure(module, &d.target);
        let names: BTreeSet<String> = closure.iter().filter_map(|k| p.defs[k].owner.as_deref()).map(|o| last(o).to_string()).collect();
        let mut ids: Vec<&str> = closure.iter().map(|k| p.defs[k].id.as_str()).collect();
        ids.sort();
        // The root: f's own methods. Two functions can share a closure (`p`
        // and `q` calling each other) and differ in their derivative.
        let root = format!("{module}.{}", d.target);
        let mut roots: Vec<&str> = p.defs.values().filter(|x| x.owner.as_deref() == Some(root.as_str())).map(|x| x.id.as_str()).collect();
        roots.sort();
        let rules = p.rules_for(&names);
        let mut rule_ids: Vec<&str> = rules.iter().map(|d| d.id.as_str()).collect();
        rule_ids.sort();
        let mut store = Canonical::new();
        for (n, v) in o.versions.get(&d.label).into_iter().flatten() {
            store.str(n, v);
        }
        for i in &roots {
            store.str("root", i);
        }
        store.str("mode", "Zygote reverse Context{false}");
        store.str("argtype", &d.argtype);
        for i in &ids {
            store.str("closure", i);
        }
        for i in &rule_ids {
            store.str("rule", i);
        }
        let store = store.finish();
        let mut keys = BTreeMap::new();
        keys.insert(Key::Store, Digest::of(&store));
        let mut c = Canonical::new();
        c.raw(&store);
        let mut facts: Vec<&RuleFact> = o.rules.get(&d.label).into_iter().flatten().filter(|r| names_one(&r.sig, module, &names)).collect();
        facts.sort();
        for r in facts {
            let x = c.open("fact");
            c.str("table", &r.table);
            c.str("module", &r.module);
            c.str("sig", &r.sig);
            c.str("body", &r.body);
            c.close(x);
        }
        keys.insert(Key::Rules, Digest::of(&c.finish()));
        let mut c = Canonical::new();
        c.raw(&store);
        for (name, n, dg) in o.tables.get(&d.label).into_iter().flatten() {
            c.str("table", name);
            c.u64("methods", *n as u64);
            c.str("digest", dg);
        }
        keys.insert(Key::Tables, Digest::of(&c.finish()));
        ledger.rows.push(Row {
            label: d.label.clone(),
            target: d.target.clone(),
            argtype: d.argtype.clone(),
            output: d.output(),
            keys,
            definitions: closure.len(),
            roots: roots.iter().map(|x| x.to_string()).collect(),
            closure_ids: ids.iter().map(|x| x.to_string()).collect(),
            closure: names,
            package_rules: rules.len(),
            gradient: d.gradient.clone(),
            complete: d.complete,
        });
    }
    Ok((o, ledger))
}
