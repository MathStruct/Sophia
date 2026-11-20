//! E-10 (B-029): Mathlib's repeated statements, and what statements and
//! proofs depend on. A measurement on a lean4export NDJSON file (format 3.1),
//! not a reader for the store. See `lean_statements.md`.
//!
//! Usage: cargo run --release -p sophia-ingest --example lean_statements -- FILE.ndjson OUTDIR
//!
//! 1. **Repeated statements at four levels of equality** of a theorem's type:
//!    `exact` (the export's own node: names of bound variables, binder
//!    annotations and universe parameter names all count), `alpha` (bound
//!    variable names erased), `binders` (also binder annotations: implicit,
//!    instance-implicit, … erased), `universes` (also universe parameters
//!    renamed by their position in the declaration's parameter list). The
//!    first three are hashes over the shared graph; the last is computed per
//!    theorem for the candidates that agree with universe names erased.
//! 2. **Each group's cause**, at the `universes` level: `generated` (at most one
//!    member has a name a person wrote; the others are made by Lean or by an
//!    attribute), `alias` (the hand-named members are one proof cited under
//!    several names: a member's proof is another member applied to its own
//!    arguments), `duplicate` (two or more hand-named members with proofs that
//!    do not cite each other).
//! 3. **Dependencies:** per theorem, the constants its statement mentions and
//!    the constants its proof mentions; over the library, the declarations a
//!    statement needs (closure through statements and definition bodies) and
//!    those only proofs reach; the axioms behind each theorem.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::time::Instant;

const NONE: u32 = u32::MAX;
type H = [u8; 16];

// Expression kinds.
const BVAR: u8 = 1;
const SORT: u8 = 2;
const CONST: u8 = 3;
const APP: u8 = 4;
const LAM: u8 = 5;
const PI: u8 = 6;
const LET: u8 = 7;
const NAT: u8 = 8;
const STR: u8 = 9;
const MDATA: u8 = 10;
const PROJ: u8 = 11;

/// One expression node. `a`, `b`, `c` are children or fields by kind:
/// bvar a = index; sort a = level; const x = name, a..a+b = universe list in
/// the pool; app a = fn, b = arg; lam/pi a = type, b = body, x = binder name,
/// bi = binder info; let a = type, b = value, c = body, x = name;
/// nat/str x = literal pool index; mdata a = expr; proj x = type name, a = struct, b = idx.
#[derive(Clone, Copy, Default)]
struct Node {
    kind: u8,
    bi: u8,
    a: u32,
    b: u32,
    c: u32,
    x: u32,
}

impl Node {
    fn children(&self) -> [u32; 3] {
        match self.kind {
            APP | LAM | PI => [self.a, self.b, NONE],
            LET => [self.a, self.b, self.c],
            MDATA | PROJ => [self.a, NONE, NONE],
            _ => [NONE; 3],
        }
    }
}

#[derive(Clone, Copy)]
enum Level {
    Zero,
    Succ(u32),
    Max(u32, u32),
    IMax(u32, u32),
    Param(u32),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Kind {
    Theorem,
    Definition,
    Opaque,
    Axiom,
    Quotient,
    Inductive,
    Constructor,
    Recursor,
}

struct Decl {
    name: u32,
    kind: Kind,
    lparams: Vec<u32>,
    ty: u32,
    value: u32,
    /// For an inductive type, constructor or recursor: the other declarations of its group.
    group: Vec<u32>,
}

/// The integer after `"key":` in `s`.
fn int_after(s: &str, key: &str) -> Option<u32> {
    let pat = format!("\"{key}\":");
    let i = s.find(&pat)? + pat.len();
    let b = s.as_bytes();
    let mut n: u32 = 0;
    let mut j = i;
    while j < b.len() && b[j].is_ascii_digit() {
        n = n * 10 + (b[j] - b'0') as u32;
        j += 1;
    }
    (j > i).then_some(n)
}

/// The string after `"key":"` in `s` (no escapes expected).
fn str_after<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":\"");
    let i = s.find(&pat)? + pat.len();
    let e = s[i..].find('"')?;
    Some(&s[i..i + e])
}

/// The integers of the array after `"key":[`.
fn ints_in(s: &str, key: &str) -> Vec<u32> {
    let pat = format!("\"{key}\":[");
    let Some(i) = s.find(&pat).map(|i| i + pat.len()) else { return Vec::new() };
    let e = s[i..].find(']').unwrap_or(0);
    s[i..i + e].split(',').filter_map(|t| t.trim().parse().ok()).collect()
}

/// The line's kind: its first key other than `ie`.
fn kind(line: &str) -> &str {
    let mut rest = line.strip_prefix("{\"").unwrap_or(line);
    if let Some(r) = rest.strip_prefix("ie\":") {
        let r = r.trim_start_matches(|c: char| c.is_ascii_digit());
        rest = r.strip_prefix(",\"").unwrap_or(r);
    }
    rest.split('"').next().unwrap_or("")
}

fn h(parts: &[&[u8]]) -> H {
    let mut hs = blake3::Hasher::new();
    for p in parts {
        hs.update(&(p.len() as u32).to_le_bytes());
        hs.update(p);
    }
    hs.finalize().as_bytes()[..16].try_into().unwrap()
}

struct Lib {
    /// name index → (prefix, component); a numeric component is stored as `#n`.
    names: Vec<(u32, String)>,
    name_hash: Vec<H>,
    levels: Vec<Level>,
    nodes: Vec<Node>,
    pool: Vec<u32>,
    lits: Vec<String>,
    decls: Vec<Decl>,
    by_name: HashMap<u32, u32>,
}

impl Lib {
    fn name(&self, n: u32) -> String {
        let mut parts = Vec::new();
        let mut i = n;
        while i != 0 && (i as usize) < self.names.len() {
            let (pre, s) = &self.names[i as usize];
            parts.push(s.strip_prefix('#').unwrap_or(s).to_string());
            i = *pre;
        }
        parts.reverse();
        parts.join(".")
    }
    fn components(&self, n: u32) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut i = n;
        while i != 0 && (i as usize) < self.names.len() {
            let (pre, s) = &self.names[i as usize];
            parts.push(s.as_str());
            i = *pre;
        }
        parts.reverse();
        parts
    }

    /// A level's hash. `params`: None keeps the parameter's name; Some(&[]) erases
    /// it; Some(list) replaces it by its position in the list.
    fn level_hash(&self, l: u32, params: Option<&[u32]>) -> H {
        match self.levels.get(l as usize).copied().unwrap_or(Level::Zero) {
            Level::Zero => h(&[b"z"]),
            Level::Succ(a) => h(&[b"s", &self.level_hash(a, params)]),
            Level::Max(a, b) => h(&[b"m", &self.level_hash(a, params), &self.level_hash(b, params)]),
            Level::IMax(a, b) => h(&[b"i", &self.level_hash(a, params), &self.level_hash(b, params)]),
            Level::Param(n) => match params {
                None => h(&[b"p", &self.name_hash[n as usize]]),
                Some([]) => h(&[b"p"]),
                Some(ps) => match ps.iter().position(|&p| p == n) {
                    Some(i) => h(&[b"p#", &(i as u32).to_le_bytes()]),
                    None => h(&[b"p", &self.name_hash[n as usize]]),
                },
            },
        }
    }

    /// A node's hash from its children's. `mode`: 0 exact, 1 alpha, 2 binders,
    /// 3 binders with universe names erased, 4 binders with universes by
    /// position (`params`). `lh` hashes a level under the mode.
    fn node_hash(&self, n: &Node, mode: u8, child: impl Fn(u32) -> H, lh: impl Fn(u32) -> H) -> H {
        let name = |x: u32| if mode == 0 { self.name_hash[x as usize] } else { [0; 16] };
        let bi = if mode <= 1 { n.bi } else { 0 };
        match n.kind {
            BVAR => h(&[b"v", &n.a.to_le_bytes()]),
            SORT => h(&[b"S", &lh(n.a)]),
            CONST => {
                let us: Vec<H> = self.pool[n.a as usize..(n.a + n.b) as usize].iter().map(|&l| lh(l)).collect();
                let mut parts: Vec<&[u8]> = vec![b"c", &self.name_hash[n.x as usize]];
                parts.extend(us.iter().map(|u| u.as_slice()));
                h(&parts)
            }
            APP => h(&[b"a", &child(n.a), &child(n.b)]),
            LAM | PI => h(&[if n.kind == LAM { b"l" } else { b"P" }, &child(n.a), &child(n.b), &name(n.x), &[bi]]),
            LET => h(&[b"L", &child(n.a), &child(n.b), &child(n.c), &name(n.x), &[n.bi]]),
            NAT => h(&[b"n", self.lits[n.x as usize].as_bytes()]),
            STR => h(&[b"t", self.lits[n.x as usize].as_bytes()]),
            // Metadata does not change a term's meaning to the kernel.
            MDATA => child(n.a),
            PROJ => h(&[b"j", &self.name_hash[n.x as usize], &n.b.to_le_bytes(), &child(n.a)]),
            _ => h(&[b"?"]),
        }
    }

    /// Distinct nodes reachable from `root`, ascending (children before parents).
    fn reach(&self, root: u32, stamp: &mut [u32], epoch: u32) -> Vec<u32> {
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(x) = stack.pop() {
            if x == NONE || x as usize >= self.nodes.len() || stamp[x as usize] == epoch {
                continue;
            }
            stamp[x as usize] = epoch;
            out.push(x);
            stack.extend(self.nodes[x as usize].children().into_iter().filter(|&c| c != NONE));
        }
        out.sort_unstable();
        out
    }

    /// Constants (as declaration indices) mentioned under `root`, and whether any
    /// mentioned name has no declaration.
    fn consts(&self, root: u32, stamp: &mut [u32], epoch: u32) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        let mut stack = vec![root];
        while let Some(x) = stack.pop() {
            if x == NONE || x as usize >= self.nodes.len() || stamp[x as usize] == epoch {
                continue;
            }
            stamp[x as usize] = epoch;
            let n = &self.nodes[x as usize];
            // A constant, or a projection's structure type.
            if (n.kind == CONST || n.kind == PROJ)
                && let Some(&d) = self.by_name.get(&n.x)
            {
                out.push(d);
            }
            stack.extend(n.children().into_iter().filter(|&c| c != NONE));
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The statement hash of a theorem with universes by position (mode 4).
    fn positional_hash(&self, d: &Decl, stamp: &mut [u32], epoch: u32) -> H {
        let order = self.reach(d.ty, stamp, epoch);
        let mut memo: HashMap<u32, H> = HashMap::with_capacity(order.len());
        for &x in &order {
            let hx = self.node_hash(&self.nodes[x as usize], 4, |c| memo[&c], |l| self.level_hash(l, Some(&d.lparams)));
            memo.insert(x, hx);
        }
        h(&[&memo[&d.ty], &(d.lparams.len() as u32).to_le_bytes()])
    }
}

/// A name component made by Lean or an attribute rather than written by a person.
fn generated_component(c: &str) -> bool {
    c.starts_with('#')
        || c.starts_with('_')
        || matches!(c, "eq_def" | "injEq" | "sizeOf_spec" | "ext_iff")
        || (c.starts_with("eq_") && c[3..].chars().all(|ch| ch.is_ascii_digit()))
        || (c.starts_with("proof_") && c[6..].chars().all(|ch| ch.is_ascii_digit()))
        || (c.starts_with("match_") && c[6..].chars().all(|ch| ch.is_ascii_digit()))
}

fn read(path: &str) -> Lib {
    let f = BufReader::with_capacity(1 << 24, std::fs::File::open(path).unwrap());
    let mut lib = Lib {
        names: vec![(0, String::new())],
        name_hash: vec![h(&[b"anon"])],
        levels: vec![Level::Zero],
        nodes: Vec::new(),
        pool: Vec::new(),
        lits: Vec::new(),
        decls: Vec::new(),
        by_name: HashMap::new(),
    };
    let decl_from = |v: &serde_json::Value, kind: Kind| -> Decl {
        let u = |k: &str| v.get(k).and_then(|x| x.as_u64()).map(|x| x as u32).unwrap_or(NONE);
        Decl {
            name: u("name"),
            kind,
            lparams: v["levelParams"].as_array().map(|a| a.iter().map(|x| x.as_u64().unwrap() as u32).collect()).unwrap_or_default(),
            ty: u("type"),
            value: u("value"),
            group: Vec::new(),
        }
    };
    for line in f.lines() {
        let line = line.unwrap();
        let k = kind(&line);
        match k {
            "in" => {
                let v: serde_json::Value = serde_json::from_str(&line).unwrap();
                let i = v["in"].as_u64().unwrap() as usize;
                let (pre, comp) = if let Some(s) = v.get("str") {
                    (s["pre"].as_u64().unwrap() as u32, s["str"].as_str().unwrap().to_string())
                } else {
                    let s = &v["num"];
                    (s["pre"].as_u64().unwrap() as u32, format!("#{}", s["i"]))
                };
                let tag: &[u8] = if v.get("str").is_some() { b"s" } else { b"n" };
                let hh = h(&[&lib.name_hash[pre as usize], tag, comp.as_bytes()]);
                if lib.names.len() <= i {
                    lib.names.resize(i + 1, (0, String::new()));
                    lib.name_hash.resize(i + 1, [0; 16]);
                }
                lib.names[i] = (pre, comp);
                lib.name_hash[i] = hh;
            }
            "il" => {
                let i = int_after(&line, "il").unwrap() as usize;
                let l = if line.contains("\"succ\"") {
                    Level::Succ(int_after(&line, "succ").unwrap())
                } else if line.contains("\"imax\"") {
                    let v = ints_in(&line, "imax");
                    Level::IMax(v[0], v[1])
                } else if line.contains("\"max\"") {
                    let v = ints_in(&line, "max");
                    Level::Max(v[0], v[1])
                } else {
                    Level::Param(int_after(&line, "param").unwrap())
                };
                if lib.levels.len() <= i {
                    lib.levels.resize(i + 1, Level::Zero);
                }
                lib.levels[i] = l;
            }
            "thm" | "def" | "opaque" | "axiom" | "quot" | "inductive" => {
                let v: serde_json::Value = serde_json::from_str(&line).unwrap();
                let body = &v[k];
                let push = |d: Decl, lib: &mut Lib| {
                    lib.by_name.insert(d.name, lib.decls.len() as u32);
                    lib.decls.push(d);
                };
                match k {
                    "thm" => push(decl_from(body, Kind::Theorem), &mut lib),
                    "def" => push(decl_from(body, Kind::Definition), &mut lib),
                    "opaque" => push(decl_from(body, Kind::Opaque), &mut lib),
                    "axiom" => push(decl_from(body, Kind::Axiom), &mut lib),
                    "quot" => push(decl_from(body, Kind::Quotient), &mut lib),
                    _ => {
                        let first = lib.decls.len() as u32;
                        for (key, kind) in [("types", Kind::Inductive), ("ctors", Kind::Constructor), ("recs", Kind::Recursor)] {
                            for t in body[key].as_array().into_iter().flatten() {
                                push(decl_from(t, kind), &mut lib);
                            }
                        }
                        let group: Vec<u32> = (first..lib.decls.len() as u32).collect();
                        for &g in &group {
                            lib.decls[g as usize].group = group.clone();
                        }
                    }
                }
            }
            "meta" => {}
            _ => {
                let Some(ie) = int_after(&line, "ie") else { continue };
                let mut n = Node { a: NONE, b: NONE, c: NONE, x: NONE, ..Default::default() };
                match k {
                    "bvar" => (n.kind, n.a) = (BVAR, int_after(&line, "bvar").unwrap()),
                    "sort" => (n.kind, n.a) = (SORT, int_after(&line, "sort").unwrap()),
                    "const" => {
                        let us = ints_in(&line, "us");
                        n.kind = CONST;
                        n.x = int_after(&line, "name").unwrap();
                        n.a = lib.pool.len() as u32;
                        n.b = us.len() as u32;
                        lib.pool.extend(us);
                    }
                    "app" => (n.kind, n.a, n.b) = (APP, int_after(&line, "fn").unwrap(), int_after(&line, "arg").unwrap()),
                    "lam" | "forallE" => {
                        n.kind = if k == "lam" { LAM } else { PI };
                        n.a = int_after(&line, "type").unwrap();
                        n.b = int_after(&line, "body").unwrap();
                        n.x = int_after(&line, "name").unwrap();
                        n.bi = match str_after(&line, "binderInfo") {
                            Some("implicit") => 1,
                            Some("strictImplicit") => 2,
                            Some("instImplicit") => 3,
                            _ => 0,
                        };
                    }
                    "letE" => {
                        n.kind = LET;
                        n.a = int_after(&line, "type").unwrap();
                        n.b = int_after(&line, "value").unwrap();
                        n.c = int_after(&line, "body").unwrap();
                        n.x = int_after(&line, "name").unwrap();
                        n.bi = line.contains("\"nondep\":true") as u8;
                    }
                    "natVal" | "strVal" => {
                        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
                        n.kind = if k == "natVal" { NAT } else { STR };
                        n.x = lib.lits.len() as u32;
                        lib.lits.push(v[k].as_str().unwrap().to_string());
                    }
                    "mdata" => {
                        // The child is the last `"expr":` key; the metadata may contain one.
                        let i = line.rfind("\"expr\":").unwrap();
                        (n.kind, n.a) = (MDATA, int_after(&line[i..], "expr").unwrap());
                    }
                    "proj" => {
                        n.kind = PROJ;
                        n.x = int_after(&line, "typeName").unwrap();
                        n.a = int_after(&line, "struct").unwrap();
                        n.b = int_after(&line, "idx").unwrap();
                    }
                    other => panic!("unknown expression kind {other}: {line}"),
                }
                let i = ie as usize;
                if lib.nodes.len() <= i {
                    lib.nodes.resize(i + 1, Node::default());
                }
                lib.nodes[i] = n;
            }
        }
    }
    lib
}

fn pct(a: usize, b: usize) -> String {
    format!("{:.1} %", 100.0 * a as f64 / b.max(1) as f64)
}

fn median(v: &mut [usize]) -> usize {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    v[v.len() / 2]
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("FILE.ndjson");
    let outdir = std::path::PathBuf::from(args.next().expect("OUTDIR"));
    std::fs::create_dir_all(&outdir).unwrap();
    let t = Instant::now();
    let lib = read(&path);
    let n = lib.nodes.len();
    println!(
        "{path}: read in {:.1?}: {} names, {} levels, {n} nodes, {} declarations",
        t.elapsed(),
        lib.names.len(),
        lib.levels.len(),
        lib.decls.len()
    );

    // Sanity: declaration names are unique as strings (the name table is shared).
    let mut seen: HashSet<H> = HashSet::new();
    let dup_names = lib.decls.iter().filter(|d| !seen.insert(lib.name_hash[d.name as usize])).count();
    println!("declaration names that repeat as strings: {dup_names}");

    let thms: Vec<u32> = (0..lib.decls.len() as u32).filter(|&i| lib.decls[i as usize].kind == Kind::Theorem).collect();

    // ---- 1. Statement hashes in modes 0..=3 over the nodes theorem types reach.
    let t1 = Instant::now();
    let mut stamp = vec![0u32; n];
    let mut epoch = 1u32;
    let mut dense = vec![NONE; n];
    let mut order: Vec<u32> = Vec::new();
    {
        let mut stack: Vec<u32> = thms.iter().map(|&d| lib.decls[d as usize].ty).collect();
        while let Some(x) = stack.pop() {
            if x == NONE || x as usize >= n || dense[x as usize] != NONE {
                continue;
            }
            dense[x as usize] = 0;
            order.push(x);
            stack.extend(lib.nodes[x as usize].children().into_iter().filter(|&c| c != NONE));
        }
    }
    order.sort_unstable();
    // The hash pass needs every child numbered below its parent (lean4export
    // writes and numbers children first); checked, not assumed.
    let backward = order.iter().filter(|&&x| lib.nodes[x as usize].children().iter().any(|&c| c != NONE && c >= x)).count();
    assert_eq!(backward, 0, "{backward} statement nodes have a child numbered at or above them");
    for (i, &x) in order.iter().enumerate() {
        dense[x as usize] = i as u32;
    }
    let lcache: [Vec<H>; 2] = [
        (0..lib.levels.len() as u32).map(|l| lib.level_hash(l, None)).collect(),
        (0..lib.levels.len() as u32).map(|l| lib.level_hash(l, Some(&[]))).collect(),
    ];
    let mut hs: Vec<Vec<H>> = vec![vec![[0; 16]; order.len()]; 4];
    for mode in 0..4u8 {
        let li = if mode == 3 { 1 } else { 0 };
        for (i, &x) in order.iter().enumerate() {
            let v = {
                let hm = &hs[mode as usize];
                lib.node_hash(&lib.nodes[x as usize], mode, |c| hm[dense[c as usize] as usize], |l| lcache[li][l as usize])
            };
            hs[mode as usize][i] = v;
        }
    }
    println!("statement graph: {} nodes hashed in 4 modes in {:.1?}", order.len(), t1.elapsed());
    // Distinct hashes per mode: if the export shares every repeated subterm, mode 0
    // has one hash per node; each further mode can only merge.
    for (m, label) in ["exact", "alpha", "binders", "universes erased"].iter().enumerate() {
        let d: HashSet<&H> = hs[m].iter().collect();
        println!("  distinct {label:<17} {:>10} ({} nodes merged)", d.len(), order.len() - d.len());
    }

    let groups_of = |key: &dyn Fn(u32) -> H| -> Vec<Vec<u32>> {
        let mut m: HashMap<H, Vec<u32>> = HashMap::new();
        for &d in &thms {
            m.entry(key(d)).or_default().push(d);
        }
        let mut g: Vec<Vec<u32>> = m.into_values().filter(|v| v.len() > 1).collect();
        g.sort_by_key(|v| v[0]);
        g
    };
    let report = |label: &str, g: &[Vec<u32>]| {
        let members: usize = g.iter().map(|v| v.len()).sum();
        println!(
            "  {label:<10} {:>7} groups, {:>7} theorems in groups, {:>7} beyond one per group ({} of theorems)",
            g.len(),
            members,
            members - g.len(),
            pct(members - g.len(), thms.len())
        );
    };
    println!("repeated statements ({} theorems):", thms.len());
    let by_node = groups_of(&|d| {
        let mut x = [0u8; 16];
        x[..4].copy_from_slice(&lib.decls[d as usize].ty.to_le_bytes());
        x
    });
    report("node", &by_node);
    let (libr, hsr, denser) = (&lib, &hs, &dense);
    let mode_key = |m: usize| {
        move |d: u32| -> H {
            let (lib, hs, dense) = (libr, hsr, denser);
            let dd = &lib.decls[d as usize];
            h(&[&hs[m][dense[dd.ty as usize] as usize], &(dd.lparams.len() as u32).to_le_bytes()])
        }
    };
    let exact = groups_of(&|d| {
        let dd = &lib.decls[d as usize];
        let ps: Vec<u8> = dd.lparams.iter().flat_map(|p| lib.name_hash[*p as usize]).collect();
        h(&[&hs[0][dense[dd.ty as usize] as usize], &ps])
    });
    report("exact", &exact);
    report("alpha", &groups_of(&mode_key(1)));
    report("binders", &groups_of(&mode_key(2)));
    let erased = groups_of(&mode_key(3));
    // Mode 4 (universes by position) refines the universe-erased candidates.
    let mut uni: Vec<Vec<u32>> = Vec::new();
    for g in &erased {
        let mut m: HashMap<H, Vec<u32>> = HashMap::new();
        for &d in g {
            epoch += 1;
            m.entry(lib.positional_hash(&lib.decls[d as usize], &mut stamp, epoch)).or_default().push(d);
        }
        uni.extend(m.into_values().filter(|v| v.len() > 1));
    }
    uni.sort_by_key(|v| v[0]);
    report("universes", &uni);

    // ---- 2. Causes, at the `universes` level.
    // A member is an alias of another if its proof, under its lambdas, is that
    // member's constant applied to bound variables only.
    let alias_target = |d: u32| -> Option<u32> {
        let mut x = lib.decls[d as usize].value;
        while x != NONE && lib.nodes[x as usize].kind == LAM {
            x = lib.nodes[x as usize].b;
        }
        while x != NONE && lib.nodes[x as usize].kind == APP {
            if lib.nodes[lib.nodes[x as usize].b as usize].kind != BVAR {
                return None;
            }
            x = lib.nodes[x as usize].a;
        }
        (x != NONE && lib.nodes[x as usize].kind == CONST).then(|| lib.by_name.get(&lib.nodes[x as usize].x).copied()).flatten()
    };
    let generated = |d: u32| lib.components(lib.decls[d as usize].name).iter().any(|c| generated_component(c));
    let mut causes: HashMap<&str, (usize, usize)> = HashMap::new();
    let mut extra_by_cause: HashMap<&str, usize> = HashMap::new();
    let mut gen_patterns: HashMap<String, usize> = HashMap::new();
    let mut same_proof = 0usize;
    let mut dup_citing = 0usize;
    let mut dup_independent = 0usize;
    let mut out = BufWriter::new(std::fs::File::create(outdir.join("groups.tsv")).unwrap());
    writeln!(out, "group\tcause\tsize\tmembers (name [g]=generated [→x]=alias of x)").unwrap();
    for (gi, g) in uni.iter().enumerate() {
        let user: Vec<u32> = g.iter().copied().filter(|&d| !generated(d)).collect();
        for &d in g.iter().filter(|&&d| generated(d)) {
            let pat: Vec<String> = lib
                .components(lib.decls[d as usize].name)
                .iter()
                .filter(|c| generated_component(c))
                .map(|c| if c.starts_with('#') { "#n".to_string() } else { c.to_string() })
                .collect();
            *gen_patterns.entry(pat.join(".")).or_default() += 1;
        }
        // Union all members (generated ones included, so a chain through a
        // generated member counts) along alias edges.
        let mut parent: HashMap<u32, u32> = g.iter().map(|&d| (d, d)).collect();
        fn find(p: &mut HashMap<u32, u32>, x: u32) -> u32 {
            let mut r = x;
            while p[&r] != r {
                r = p[&r];
            }
            p.insert(x, r);
            r
        }
        for &d in g {
            if let Some(t) = alias_target(d) {
                if parent.contains_key(&t) {
                    let (a, b) = (find(&mut parent, d), find(&mut parent, t));
                    parent.insert(a, b);
                }
            }
        }
        let roots: HashSet<u32> = user.clone().into_iter().map(|d| find(&mut parent, d)).collect();
        let cause = if user.len() <= 1 {
            "generated"
        } else if roots.len() == 1 {
            "alias"
        } else {
            "duplicate"
        };
        if cause == "duplicate" {
            // Exclusive sub-classes: two hand-named members with the same proof
            // node; else a proof that mentions another member other than as a
            // bare alias; else independent proofs.
            let vals: HashSet<u32> = user.iter().map(|&d| lib.decls[d as usize].value).collect();
            let cites = user.iter().any(|&d| {
                epoch += 1;
                let bare = alias_target(d);
                lib.consts(lib.decls[d as usize].value, &mut stamp, epoch).iter().any(|c| *c != d && g.contains(c) && Some(*c) != bare)
            });
            if vals.len() < user.len() {
                same_proof += 1;
            } else if cites {
                dup_citing += 1;
            } else {
                dup_independent += 1;
            }
        }
        let e = causes.entry(cause).or_default();
        e.0 += 1;
        e.1 += g.len();
        *extra_by_cause.entry(cause).or_default() += g.len() - 1;
        let members: Vec<String> = g
            .iter()
            .map(|&d| {
                let mut s = lib.name(lib.decls[d as usize].name);
                if generated(d) {
                    s.push_str(" [g]");
                }
                if let Some(t) = alias_target(d).filter(|t| g.contains(t)) {
                    s.push_str(&format!(" [→{}]", lib.name(lib.decls[t as usize].name)));
                }
                s
            })
            .collect();
        writeln!(out, "{gi}\t{cause}\t{}\t{}", g.len(), members.join(" | ")).unwrap();
    }
    out.flush().unwrap();
    println!("causes at the `universes` level (groups, theorems, theorems beyond one per group):");
    for c in ["generated", "alias", "duplicate"] {
        let (gs, ts) = causes.get(c).copied().unwrap_or_default();
        println!("  {c:<10} {gs:>7} {ts:>7} {:>7}", extra_by_cause.get(c).copied().unwrap_or(0));
    }
    println!(
        "  duplicate groups: {same_proof} with two hand-named members of the same proof node, {dup_citing} others where a proof mentions another member (not as a bare alias), {dup_independent} with independent proofs"
    );
    let mut gp: Vec<_> = gen_patterns.into_iter().collect();
    gp.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
    println!(
        "  generated-name components in groups (top 15): {}",
        gp.iter().take(15).map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", ")
    );

    // ---- 3. Dependencies.
    let t3 = Instant::now();
    let nd = lib.decls.len();
    let mut ty_refs: Vec<Vec<u32>> = Vec::with_capacity(nd);
    let mut val_refs: Vec<Vec<u32>> = Vec::with_capacity(nd);
    for d in &lib.decls {
        epoch += 1;
        ty_refs.push(lib.consts(d.ty, &mut stamp, epoch));
        epoch += 1;
        val_refs.push(if d.value == NONE { Vec::new() } else { lib.consts(d.value, &mut stamp, epoch) });
    }
    let (mut s_counts, mut p_counts, mut p_only_counts, mut p_thm_counts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut stmt_subset = 0usize;
    for &d in &thms {
        let (s, p) = (&ty_refs[d as usize], &val_refs[d as usize]);
        let sset: HashSet<u32> = s.iter().copied().collect();
        let only = p.iter().filter(|x| !sset.contains(x)).count();
        if s.iter().all(|x| p.binary_search(x).is_ok()) {
            stmt_subset += 1;
        }
        s_counts.push(s.len());
        p_counts.push(p.len());
        p_only_counts.push(only);
        p_thm_counts.push(p.iter().filter(|&&x| lib.decls[x as usize].kind == Kind::Theorem).count());
    }
    let mean = |v: &[usize]| v.iter().sum::<usize>() as f64 / v.len().max(1) as f64;
    println!("dependencies per theorem (distinct constants mentioned directly), mean / median / max:");
    for (label, v) in [
        ("statement", &mut s_counts),
        ("proof", &mut p_counts),
        ("proof, not in statement", &mut p_only_counts),
        ("proof, theorems cited", &mut p_thm_counts),
    ] {
        let m = mean(v);
        let max = v.iter().copied().max().unwrap_or(0);
        println!("  {label:<25} {m:>7.1} {:>5} {max:>6}", median(v));
    }
    println!("  theorems whose proof mentions every constant of their statement: {} of {}", stmt_subset, thms.len());

    // Closure a statement needs: from every theorem's type, follow types of
    // everything reached, bodies of definitions (not of theorems or opaques),
    // and every declaration of a reached inductive group.
    let mut in_stmt = vec![false; nd];
    let mut stack: Vec<u32> = thms.iter().flat_map(|&d| ty_refs[d as usize].iter().copied()).collect();
    while let Some(x) = stack.pop() {
        if in_stmt[x as usize] {
            continue;
        }
        in_stmt[x as usize] = true;
        let d = &lib.decls[x as usize];
        stack.extend(ty_refs[x as usize].iter().copied());
        if d.kind == Kind::Definition {
            stack.extend(val_refs[x as usize].iter().copied());
        }
        stack.extend(d.group.iter().copied());
    }
    // Reached from proofs: the same closure started from theorem values.
    let mut in_proof = vec![false; nd];
    let mut stack: Vec<u32> = thms.iter().flat_map(|&d| val_refs[d as usize].iter().copied()).collect();
    while let Some(x) = stack.pop() {
        if in_proof[x as usize] {
            continue;
        }
        in_proof[x as usize] = true;
        let d = &lib.decls[x as usize];
        stack.extend(ty_refs[x as usize].iter().copied());
        if d.kind == Kind::Definition {
            stack.extend(val_refs[x as usize].iter().copied());
        }
        stack.extend(d.group.iter().copied());
    }
    let mut by_kind: HashMap<Kind, [usize; 4]> = HashMap::new();
    for i in 0..nd {
        let e = by_kind.entry(lib.decls[i].kind).or_default();
        e[0] += 1;
        e[1] += in_stmt[i] as usize;
        e[2] += (in_proof[i] && !in_stmt[i]) as usize;
        e[3] += (!in_proof[i] && !in_stmt[i]) as usize;
    }
    println!("declarations by what reaches them (closure: types, definition bodies, inductive groups):");
    println!("  {:<12} {:>8} {:>16} {:>16} {:>16}", "kind", "all", "statements", "only proofs", "neither");
    let mut ks: Vec<_> = by_kind.into_iter().collect();
    ks.sort_by_key(|(_, v)| std::cmp::Reverse(v[0]));
    for (k, v) in ks {
        println!("  {:<12} {:>8} {:>16} {:>16} {:>16}", format!("{k:?}"), v[0], v[1], v[2], v[3]);
    }

    // Axioms behind each declaration, through types and values of everything
    // (theorems included: a proof that uses an axiom rests on it).
    let axioms: Vec<u32> = (0..nd as u32).filter(|&i| lib.decls[i as usize].kind == Kind::Axiom).collect();
    assert!(axioms.len() <= 64, "{} axioms: the bit set holds 64", axioms.len());
    let mut bits = vec![0u64; nd];
    for (j, &a) in axioms.iter().enumerate() {
        bits[a as usize] |= 1 << j;
    }
    // A declaration rests on what its type and value mention and on its
    // inductive group (a constructor's type). Iterated to a fixpoint: the first
    // round settles everything that refers only backwards in the file.
    let forward = (0..nd)
        .map(|i| ty_refs[i].iter().chain(&val_refs[i]).filter(|&&r| r as usize >= i && !lib.decls[i].group.contains(&r)).count())
        .sum::<usize>();
    let mut rounds = 0;
    loop {
        rounds += 1;
        let mut changed = false;
        for i in 0..nd {
            let mut b = bits[i];
            for &r in ty_refs[i].iter().chain(&val_refs[i]).chain(&lib.decls[i].group) {
                b |= bits[r as usize];
            }
            if b != bits[i] {
                bits[i] = b;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    println!("axioms behind theorems ({forward} forward references outside inductive groups; fixpoint after {rounds} rounds):");
    for (j, &a) in axioms.iter().enumerate() {
        let c = thms.iter().filter(|&&d| bits[d as usize] & (1 << j) != 0).count();
        println!("  {:<24} {c:>7} theorems ({})", lib.name(lib.decls[a as usize].name), pct(c, thms.len()));
    }
    let none = thms.iter().filter(|&&d| bits[d as usize] == 0).count();
    println!("  {:<24} {none:>7} theorems ({})", "(no axiom)", pct(none, thms.len()));
    println!("dependencies in {:.1?}; total {:.1?}", t3.elapsed(), t.elapsed());
}
