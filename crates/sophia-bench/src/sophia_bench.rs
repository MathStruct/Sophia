//! The benchmark suite's shared parts: a seeded generator of synthetic
//! sophialang programs, closure walks, and timing helpers. See
//! `sophia_bench.md` and `vault/Plans/Benchmark Plan.md`.
//!
//! The generator's distributions are **assumptions** (Benchmark Plan,
//! "Principles"): definition sizes roughly log-normal, references per
//! definition roughly Poisson, about 1 % of definitions in mutually recursive
//! pairs. Results on synthetic stores say how the store scales, not how big
//! real code is; B-07 measures real code once there is some.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use sophia_hash::{Digest, Record};
use sophia_store::{Backend, ReadTx, Store, Visibility};
use sophialang_core::samples::builtin;
use sophialang_core::{Binder, Definition, Lit, Term};

/// SplitMix64: small, fast, seedable, good enough for workloads.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Approximately normal (sum of uniforms).
    pub fn normal(&mut self) -> f64 {
        (0..12).map(|_| self.unit()).sum::<f64>() - 6.0
    }

    /// Poisson by inversion (fine for small means).
    pub fn poisson(&mut self, mean: f64) -> u64 {
        let l = (-mean).exp();
        let (mut k, mut p) = (0, 1.0);
        loop {
            p *= self.unit();
            if p <= l {
                return k;
            }
            k += 1;
        }
    }
}

/// Parameters of a synthetic program.
#[derive(Debug, Clone, Copy)]
pub struct Shape {
    /// Median number of term nodes per definition.
    pub median_nodes: f64,
    /// Log-normal sigma of the node count.
    pub sigma: f64,
    /// Mean number of references to earlier definitions.
    pub mean_refs: f64,
    /// Fraction of definitions in mutually recursive pairs.
    pub recursive_fraction: f64,
}

impl Default for Shape {
    fn default() -> Self {
        Shape { median_nodes: 60.0, sigma: 0.9, mean_refs: 3.0, recursive_fraction: 0.01 }
    }
}

struct Gen<'a> {
    rng: &'a mut Rng,
    depth: u32,
    refs: Vec<u32>,
}

impl Gen<'_> {
    fn term(&mut self, budget: &mut i64) -> Term {
        *budget -= 1;
        let leaf = *budget <= 0 || self.depth > 24;
        if leaf {
            return match self.rng.below(5) {
                0 if self.depth > 0 => Term::Var(self.rng.below(self.depth.into()) as u32),
                1 => Term::Lit(Lit::Int64(self.rng.below(1000) as i64)),
                2 => Term::Nominal(builtin(["+", "*", "-", "==", "getindex"][self.rng.below(5) as usize])),
                3 if !self.refs.is_empty() => Term::Draft(self.refs.pop().unwrap()),
                _ => Term::Prim(["add_int", "mul_int", "load_int64"][self.rng.below(3) as usize].into()),
            };
        }
        match self.rng.below(10) {
            0..=4 => {
                let n = 1 + self.rng.below(3) as usize;
                let head = if !self.refs.is_empty() && self.rng.below(2) == 0 {
                    Term::Draft(self.refs.pop().unwrap())
                } else {
                    Term::Nominal(builtin(["+", "*", "f", "g", "map"][self.rng.below(5) as usize]))
                };
                let args = (0..n).map(|_| self.term(budget)).collect();
                Term::App(Box::new(head), args)
            }
            5..=6 => {
                self.depth += 1;
                let body = self.term(budget);
                self.depth -= 1;
                Term::Lam { binder: Binder::named("x"), domain: None, body: Box::new(body) }
            }
            7..=8 => {
                let value = self.term(budget);
                self.depth += 1;
                let body = self.term(budget);
                self.depth -= 1;
                Term::Let { binder: Binder::named("y"), ty: None, value: Box::new(value), body: Box::new(body) }
            }
            _ => Term::Hole(Box::new(Term::Nominal(builtin("Int64")))),
        }
    }
}

/// A synthetic program of `n` definitions, as (name, definition) pairs with
/// `Term::Draft` references to earlier definitions (and, for recursive
/// pairs, to each other).
pub fn generate(seed: u64, n: usize, shape: Shape) -> Vec<(String, Definition)> {
    let mut rng = Rng::new(seed);
    let int64 = Term::Nominal(builtin("Int64"));
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let nodes = (shape.median_nodes * (shape.sigma * rng.normal()).exp()).clamp(3.0, 5000.0) as i64;
        let mut refs: Vec<u32> = if i == 0 {
            Vec::new()
        } else {
            (0..rng.poisson(shape.mean_refs)).map(|_| rng.below(i as u64) as u32).collect()
        };
        // Mutually recursive pairs: i references i+1 and i+1 references i.
        if i + 1 < n && i % 2 == 0 && rng.unit() < shape.recursive_fraction {
            refs.push(i as u32 + 1);
        }
        if i % 2 == 1 && out.len() == i {
            let prev: &Definition = &out.last().map(|p: &(String, Definition)| p.1.clone()).unwrap();
            let mut refers_to_me = false;
            prev.walk(&mut |t| refers_to_me |= *t == Term::Draft(i as u32));
            if refers_to_me {
                refs.push(i as u32 - 1);
            }
        }
        let mut budget = nodes;
        let mut g = Gen { rng: &mut rng, depth: 1, refs };
        let mut body = g.term(&mut budget);
        // Every planned reference appears somewhere.
        let leftover: Vec<u32> = std::mem::take(&mut g.refs);
        if !leftover.is_empty() {
            body = Term::App(Box::new(body), leftover.into_iter().map(Term::Draft).collect());
        }
        let def = Definition::method(
            Term::arrow(int64.clone(), int64.clone()),
            Term::Lam { binder: Binder::named("x"), domain: None, body: Box::new(body) },
        );
        out.push((format!("gen.m{:07}", i), def));
    }
    out
}

/// Loads `(name, definition)` pairs into a store in batches of `batch`
/// definitions. Recursive pairs never straddle a batch boundary because
/// batches are even-sized. Returns the records in input order.
pub fn ingest<B: Backend>(store: &Store<B>, program: &[(String, Definition)], batch: usize) -> Vec<Record> {
    assert!(batch % 2 == 0, "even batches keep recursive pairs together");
    let mut ids: Vec<Digest> = Vec::with_capacity(program.len());
    let mut all = Vec::with_capacity(program.len());
    for (start, chunk) in program.chunks(batch).enumerate().map(|(k, c)| (k * batch, c)) {
        // Rewrite drafts that point into earlier batches into refs.
        let defs: Vec<Definition> = chunk
            .iter()
            .map(|(_, d)| {
                d.clone().map_terms(&mut |t| match t {
                    Term::Draft(j) if (j as usize) < start => Term::Ref(ids[j as usize]),
                    Term::Draft(j) => Term::Draft(j - start as u32),
                    t => t,
                })
            })
            .collect();
        let records = sophialang_core::normalise_batch(&defs, &mut Default::default()).expect("normalise");
        let names: Vec<(String, usize, Visibility)> =
            chunk.iter().enumerate().map(|(i, (n, _))| (n.clone(), i, Visibility::Public)).collect();
        store.add(&records, &names).expect("add");
        ids.extend(records.iter().map(|r| r.id));
        all.extend(records);
    }
    all
}

/// The transitive dependencies of `root` (B-04).
pub fn closure(tx: &dyn ReadTx, root: &Digest) -> sophia_store::backend::Result<usize> {
    let mut seen: HashSet<Digest> = HashSet::new();
    let mut stack = vec![*root];
    while let Some(d) = stack.pop() {
        if !seen.insert(d) {
            continue;
        }
        if let Some(r) = tx.record(&d)? {
            stack.extend(r.refs);
        }
    }
    Ok(seen.len())
}

/// Runs `f` `runs` times and returns the median duration.
pub fn median_time(runs: usize, mut f: impl FnMut()) -> Duration {
    let mut v: Vec<Duration> = (0..runs.max(1))
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed()
        })
        .collect();
    v.sort();
    v[v.len() / 2]
}
