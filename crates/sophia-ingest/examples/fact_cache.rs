//! B-035, the fact cache experiment of SPEC-001 §8a (SR-152): how often a
//! per-user cache of content-only facts would save a tool run, how large it
//! grows, and what a lookup costs. Nothing in any store is changed; the
//! cache stores built for the lookup measurement live in OUTDIR. See
//! `fact_cache.md`.
//!
//! ```sh
//! cargo run --release -p sophia-ingest --example fact_cache -- OUTDIR \
//!     [--julia-module] --history REPO [--history REPO ...] [--head DIR ...]
//! ```
//!
//! `--julia-module` (B-094) computes Julia ids as Moonkale's folder store
//! does: all Julia files of a folder resolved together by
//! `moonkale_content_id::julia::module`, so a change to one Julia file
//! re-identifies every Julia file in the folder. `--dump DIR` writes each
//! `--head` folder's Julia ids to `DIR/<nn>-<name>.julia-ids.tsv` (`nn`: the
//! folder's position among the arguments).

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write as _};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use moonkale_content_id::{Budget, julia, parse};
use sophia_hash::Digest;
use sophia_ingest::code;
use sophia_store::{Backend, Fact, Input, Store};

type R<T> = Result<T, Box<dyn std::error::Error>>;

/// The language of a path, by extension; `None` for files no frontend reads.
/// Paths with a hidden segment are skipped, as Moonkale's
/// `FolderStore::admits_path` never admits them.
fn lang_of(path: &str) -> Option<&'static str> {
    if path.split('/').any(|s| s.starts_with('.')) {
        return None;
    }
    match Path::new(path).extension()?.to_str()? {
        "jl" => Some("julia"),
        "rs" => Some("rust"),
        "py" => Some("python"),
        _ => None,
    }
}

/// The definition ids of each distinct (language, text), computed once; and
/// the time the frontend (parse, normalise, hash) spent on them.
#[derive(Default)]
struct Ids {
    /// `lang:blake3(text)` -> ids. The same key in both input modes.
    by_key: HashMap<String, Vec<Digest>>,
    frontend: Duration,
    defs: usize,
    /// Definitions the normal form refused (they get no id, so no fact).
    refused_defs: usize,
    /// Files the frontend refused whole (a parse error or its deadline).
    refused_files: usize,
    /// Per language: (texts, definitions with an id, definitions refused).
    per_lang: std::collections::BTreeMap<String, (usize, usize, usize)>,
    /// `--dump DIR`: where `head` writes each folder's Julia ids.
    dump: Option<PathBuf>,
    /// `--julia-module`: Julia ids per folder's whole Julia file set.
    julia_module: bool,
    /// Julia texts by key, kept for the module mode.
    texts: HashMap<String, String>,
    /// Module mode: set key -> done (its per-file keys are in `by_key`).
    module_sets: HashSet<String>,
    /// Module mode: time, definitions with an id, refused, sets refused whole.
    module_frontend: Duration,
    module_defs: usize,
    module_refused_defs: usize,
    module_refused_sets: usize,
}

impl Ids {
    /// The key of `text` read as `lang`, computing its ids on first sight.
    fn key(&mut self, lang: &str, text: &str) -> String {
        let key = format!("{lang}:{}", blake3::hash(text.as_bytes()).to_hex());
        if !self.by_key.contains_key(&key) {
            let t = Instant::now();
            let ids = match code::file(lang, text) {
                Ok(f) => {
                    self.refused_defs += f.refused.len();
                    self.per_lang.entry(lang.to_string()).or_default().2 += f.refused.len();
                    f.records.iter().map(|r| r.id).collect()
                }
                Err(_) => {
                    self.refused_files += 1;
                    Vec::new()
                }
            };
            self.frontend += t.elapsed();
            self.defs += ids.len();
            let e = self.per_lang.entry(lang.to_string()).or_default();
            e.0 += 1;
            e.1 += ids.len();
            self.by_key.insert(key.clone(), ids);
            if self.julia_module && lang == "julia" {
                self.texts.insert(key.clone(), text.to_string());
            }
        }
        key
    }

    /// Module mode: replaces the per-file keys of the Julia files in `tree`
    /// by keys whose ids come from resolving all of them as one set, as
    /// Moonkale's folder store does (`folder_store/producer.rs`,
    /// `julia_module`, at Moonkale `d0e784c7`). A set the crate refuses (a
    /// limit, a parse failure) gives no Julia ids at all, as there the
    /// folder's Julia ingest fails.
    fn module_keys(&mut self, tree: &mut HashMap<String, String>) -> R<()> {
        if !self.julia_module {
            return Ok(());
        }
        let mut jl: Vec<(String, String)> =
            tree.iter().filter(|(p, _)| lang_of(p) == Some("julia")).map(|(p, k)| (p.clone(), k.clone())).collect();
        if jl.is_empty() {
            return Ok(());
        }
        jl.sort();
        let mut h = blake3::Hasher::new();
        for (p, k) in &jl {
            h.update(p.as_bytes()).update(b"\0").update(k.as_bytes()).update(b"\0");
        }
        let set = h.finalize().to_hex().to_string();
        let key_of = |p: &str| format!("julia-module:{set}:{p}");
        if self.module_sets.insert(set.clone()) {
            let t = Instant::now();
            let per_file = self.resolve(&jl);
            self.module_frontend += t.elapsed();
            match per_file {
                Some((v, refused)) => {
                    self.module_refused_defs += refused;
                    for ((p, _), ids) in jl.iter().zip(v) {
                        self.module_defs += ids.len();
                        self.by_key.insert(key_of(p), ids);
                    }
                }
                None => {
                    // Moonkale keeps the folder's old records when its Julia
                    // ingest fails; this model has no such state, so a
                    // refused set ends the run rather than skew the counts.
                    self.module_refused_sets += 1;
                    let n = jl.len();
                    return Err(format!("module mode: the resolver refused a set of {n} Julia files").into());
                }
            }
        }
        for (p, _) in &jl {
            tree.insert(p.clone(), key_of(p));
        }
        Ok(())
    }

    /// Moonkale's `julia_module`: include bases from the include graph
    /// (roots are files nobody includes, then any file left), then the
    /// crate's module resolver. Returns each file's ids in order and the
    /// number of refused definitions.
    fn resolve(&self, jl: &[(String, String)]) -> Option<(Vec<Vec<Digest>>, usize)> {
        let texts: Vec<&str> = jl.iter().map(|(_, k)| self.texts[k].as_str()).collect();
        let mut trees = Vec::new();
        for t in &texts {
            trees.push(parse("julia", t, &mut Budget::new()).ok()?);
        }
        let paths: HashMap<&str, usize> = jl.iter().enumerate().map(|(i, (p, _))| (p.as_str(), i)).collect();
        let includes: Vec<_> = texts.iter().zip(&trees).map(|(t, tr)| julia::includes(t, tr)).collect();
        let target = |i: usize, inc: &julia::Include| -> Option<usize> {
            let p = include_path(&jl[i].0, inc.path.as_deref()?)?;
            paths.get(p.as_str()).copied()
        };
        let mut incoming = HashSet::new();
        for (i, incs) in includes.iter().enumerate() {
            for inc in incs {
                if let Some(j) = target(i, inc) {
                    incoming.insert(j);
                }
            }
        }
        let mut bases: Vec<Option<Vec<String>>> = vec![None; jl.len()];
        let mut dynamic = Vec::new();
        for root in (0..jl.len()).filter(|i| !incoming.contains(i)).chain(0..jl.len()) {
            if bases[root].is_some() {
                continue;
            }
            let mut pending = vec![(root, Vec::new())];
            while let Some((i, base)) = pending.pop() {
                if bases[i].is_some() {
                    continue;
                }
                bases[i] = Some(base.clone());
                for inc in includes[i].iter().rev() {
                    let mut module = base.clone();
                    module.extend(inc.module.clone());
                    match target(i, inc) {
                        Some(j) if bases[j].is_none() => pending.push((j, module)),
                        Some(j) if bases[j].as_ref() == Some(&module) => {}
                        _ => dynamic.push(module),
                    }
                }
            }
        }
        let files: Vec<julia::ModuleFile> = texts
            .iter()
            .zip(&trees)
            .enumerate()
            .map(|(i, (t, tr))| julia::ModuleFile { text: t, tree: tr, module: bases[i].clone().unwrap_or_default() })
            .collect();
        let m = julia::module(&files, &dynamic).ok()?;
        let mut out = vec![Vec::new(); jl.len()];
        let mut refused = 0;
        for d in &m.definitions {
            match &d.identity {
                Ok(id) => out[d.key.0 as usize].push(Digest(id.digest)),
                Err(_) => refused += 1,
            }
        }
        Some((out, refused))
    }
}

/// Moonkale's `include_path`: `relative` resolved against `path`'s
/// directory, within the folder; `None` for absolute or escaping paths.
fn include_path(path: &str, relative: &str) -> Option<String> {
    if Path::new(relative).is_absolute() || relative.contains('\\') {
        return None;
    }
    let mut parts: Vec<_> = path.split('/').collect();
    parts.pop();
    for p in relative.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

/// `git cat-file --batch`, kept open.
struct Blobs {
    _child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Blobs {
    fn new(repo: &Path) -> R<Self> {
        let mut child =
            Command::new("git").arg("-C").arg(repo).args(["cat-file", "--batch"]).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Ok(Blobs { _child: child, input, output })
    }
    fn text(&mut self, sha: &str) -> R<String> {
        writeln!(self.input, "{sha}")?;
        self.input.flush()?;
        let mut header = String::new();
        self.output.read_line(&mut header)?;
        let size: usize = header.split_whitespace().nth(2).ok_or("bad cat-file header")?.parse()?;
        let mut buf = vec![0u8; size + 1];
        self.output.read_exact(&mut buf)?;
        buf.pop();
        // Lossy, as `head` reads files: both modes see the same text.
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }
}

fn git(repo: &Path, args: &[&str]) -> R<String> {
    let out = Command::new("git").arg("-C").arg(repo).args(args).output()?;
    if !out.status.success() {
        return Err(format!("git {args:?} in {}: {}", repo.display(), String::from_utf8_lossy(&out.stderr)).into());
    }
    Ok(String::from_utf8(out.stdout)?)
}

/// What one tool run per definition would cost, counted per event.
#[derive(Default, Clone, Copy)]
struct Counts {
    /// Definitions in changed files: what a tool reruns with no reuse.
    defs: usize,
    /// Reused from the folder store itself: the id is live in the folder
    /// before the event, or already met earlier in the same event.
    local: usize,
    /// In the cache only; the id was live in this folder at some earlier
    /// event (where it was computed is not tracked).
    cache_same_folder: usize,
    /// In the cache only; the id was never live in this folder before.
    cache_other_folder: usize,
}

impl Counts {
    fn runs(&self) -> usize {
        self.defs - self.local - self.cache_same_folder - self.cache_other_folder
    }
    fn add(&mut self, o: Counts) {
        self.defs += o.defs;
        self.local += o.local;
        self.cache_same_folder += o.cache_same_folder;
        self.cache_other_folder += o.cache_other_folder;
    }
}

/// The cache under `all-content-only` with no eviction: every id ever
/// live, and the folders it was live in.
#[derive(Default)]
struct Cache {
    seen: HashMap<Digest, u32>, // id -> bitmask of folders (up to 31)
}

/// One folder store: the live ids by file, and their multiset.
#[derive(Default)]
struct Folder {
    files: HashMap<String, String>, // path -> key
    live: HashMap<Digest, u32>,
}

impl Folder {
    /// Applies one event (a commit, or a checkout's working tree) given as
    /// the full file list `path -> key`; returns its counts.
    fn apply(&mut self, folder: u32, tree: &HashMap<String, String>, ids: &Ids, cache: &mut Cache) -> Counts {
        let mut c = Counts::default();
        let mut this_event: HashSet<Digest> = HashSet::new();
        let mut changed: Vec<(&String, &String)> = Vec::new();
        for (path, key) in tree {
            if self.files.get(path) == Some(key) {
                continue;
            }
            for id in &ids.by_key[key] {
                c.defs += 1;
                if self.live.contains_key(id) || !this_event.insert(*id) {
                    c.local += 1;
                } else if let Some(mask) = cache.seen.get(id) {
                    if mask & (1 << folder) != 0 {
                        c.cache_same_folder += 1;
                    } else {
                        c.cache_other_folder += 1;
                    }
                }
            }
            changed.push((path, key));
        }
        // Update the live multiset: removed and replaced files leave, changed ones enter.
        let gone: Vec<String> = self.files.keys().filter(|p| !tree.contains_key(*p)).cloned().collect();
        for p in gone {
            let old = self.files.remove(&p).unwrap();
            self.unlive(&ids.by_key[&old]);
        }
        for (path, key) in changed {
            if let Some(old) = self.files.insert(path.clone(), key.clone()) {
                self.unlive(&ids.by_key[&old]);
            }
            for id in &ids.by_key[key] {
                *self.live.entry(*id).or_default() += 1;
                *cache.seen.entry(*id).or_default() |= 1 << folder;
            }
        }
        c
    }
    fn unlive(&mut self, v: &[Digest]) {
        for id in v {
            let n = self.live.get_mut(id).unwrap();
            *n -= 1;
            if *n == 0 {
                self.live.remove(id);
            }
        }
    }
}

/// A commit's files the frontends read, `path -> key`. `-z` keeps paths
/// unquoted; symlinks (mode 120000) and submodules are skipped.
fn commit_tree(
    repo: &Path,
    rev: &str,
    blobs: &mut Blobs,
    sha_key: &mut HashMap<String, String>,
    ids: &mut Ids,
) -> R<HashMap<String, String>> {
    let mut tree = HashMap::new();
    for entry in git(repo, &["ls-tree", "-r", "-z", "--full-tree", rev])?.split('\0') {
        let Some((meta, path)) = entry.split_once('\t') else { continue };
        let mut m = meta.split_whitespace();
        let (mode, kind, sha) = (m.next(), m.next(), m.next());
        let Some(lang) = lang_of(path) else { continue };
        if kind != Some("blob") || mode == Some("120000") {
            continue;
        }
        let sha = sha.ok_or("bad ls-tree line")?;
        let memo = format!("{lang}:{sha}");
        let key = match sha_key.get(&memo) {
            Some(k) => k.clone(),
            None => {
                let k = ids.key(lang, &blobs.text(sha)?);
                sha_key.insert(memo, k.clone());
                k
            }
        };
        tree.insert(path.to_string(), key);
    }
    ids.module_keys(&mut tree)?;
    Ok(tree)
}

fn history(repo: &Path, name: &str, folder: u32, ids: &mut Ids, cache: &mut Cache, out: &mut impl std::io::Write) -> R<Counts> {
    let revs = git(repo, &["rev-list", "--first-parent", "--reverse", "HEAD"])?;
    let mut blobs = Blobs::new(repo)?;
    let mut sha_key = HashMap::new();
    let mut f = Folder::default();
    let mut total = Counts::default();
    let mut tree = HashMap::new();
    for (i, rev) in revs.lines().enumerate() {
        tree = commit_tree(repo, rev, &mut blobs, &mut sha_key, ids)?;
        let c = f.apply(folder, &tree, ids, cache);
        writeln!(
            out,
            "{name}\t{i}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            &rev[..10],
            f.live.len(),
            c.defs,
            c.local,
            c.cache_same_folder,
            c.cache_other_folder,
            c.runs()
        )?;
        if i > 0 {
            total.add(c); // the first commit is a fresh build: reported apart
        } else {
            eprintln!("{name}: first commit {} live ids", f.live.len());
        }
    }
    eprintln!("{name}: {} commits, {} live ids at HEAD", revs.lines().count(), f.live.len());
    // Check of the replay's bookkeeping (not of the file selection, which
    // both sides share): the live multiset after every replace and delete
    // equals a fresh build of HEAD's tree.
    let mut fresh = Folder::default();
    fresh.apply(31, &tree, ids, &mut Cache::default());
    assert!(fresh.live == f.live, "{name}: replayed live set differs from a fresh build of HEAD");
    Ok(total)
}

/// A checkout's current files (tracked, with uncommitted changes), opened as
/// a fresh folder store: what a new worktree's first build would cost.
fn head(dir: &Path, folder: u32, ids: &mut Ids, cache: &mut Cache) -> R<Counts> {
    // A review copy may not be a git checkout: then every file below `dir`,
    // except hidden directories, `target/` and `node_modules/`.
    let listed = match git(dir, &["ls-files", "-z"]) {
        Ok(s) => s.split('\0').filter(|p| !p.is_empty()).map(String::from).collect::<Vec<_>>(),
        Err(_) => walk(dir, dir),
    };
    let mut tree = HashMap::new();
    for path in &listed {
        let Some(lang) = lang_of(path) else { continue };
        let p = dir.join(path);
        // Symlinks are skipped, as in `history`; a deleted tracked file too.
        if std::fs::symlink_metadata(&p).map(|m| !m.is_file()).unwrap_or(true) {
            continue;
        }
        let text = String::from_utf8_lossy(&std::fs::read(&p)?).into_owned();
        tree.insert(path.clone(), ids.key(lang, &text));
    }
    ids.module_keys(&mut tree)?;
    if let Some(dump) = &ids.dump {
        // `--dump DIR`: each head's Julia ids, for a check against Moonkale's folder store.
        let name = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
        let mut lines: Vec<String> = tree
            .iter()
            .filter(|(p, _)| lang_of(p) == Some("julia"))
            .flat_map(|(p, k)| ids.by_key[k].iter().map(move |id| format!("{p}\t{}", id.to_hex())))
            .collect();
        lines.sort();
        std::fs::write(dump.join(format!("{folder:02}-{name}.julia-ids.tsv")), lines.join("\n") + "\n")?;
    }
    Ok(Folder::default().apply(folder, &tree, ids, cache))
}

fn walk(root: &Path, dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let p = e.path();
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            if !name.starts_with('.') && name != "target" && name != "node_modules" {
                out.extend(walk(root, &p));
            }
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.to_string_lossy().to_string());
        }
    }
    out
}

/// Percentiles in µs.
fn pct(times: &mut [Duration]) -> String {
    times.sort();
    let q = |p: f64| times[((times.len() - 1) as f64 * p) as usize].as_secs_f64() * 1e6;
    format!("{:.2}\t{:.2}\t{:.2}", q(0.5), q(0.9), q(0.99))
}

/// Builds a fact-only store with one fact per id (value of `value_len`
/// bytes, content-only inputs), then measures lookups by subject: hits
/// (ids drawn with replacement) and misses, interleaved, reported apart.
fn lookup<B: Backend>(store: Store<B>, path: &Path, ids: &[Digest], value_len: usize) -> R<String> {
    let t = Instant::now();
    for chunk in ids.chunks(10_000) {
        let facts: Vec<Fact> = chunk
            .iter()
            .map(|id| Fact::new("probe", *id, vec![0xa5; value_len], ("fact-cache-probe", "0"), vec![Input::Content(*id)], 0))
            .collect();
        store.record_facts(&facts)?;
    }
    let write = t.elapsed();
    const N: usize = 50_000;
    let (mut hits, mut misses) = (Vec::with_capacity(N), Vec::with_capacity(N));
    let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
    for i in 0..2 * N {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let (id, hit) = if i % 2 == 0 { (ids[(x as usize) % ids.len()], true) } else { (Digest::of(&x.to_le_bytes()), false) };
        let t = Instant::now();
        let found =
            store.read(|tx| tx.facts_of(&id))?.into_iter().any(|f| f.kind == "probe" && f.by == "fact-cache-probe" && f.by_version == "0");
        let e = t.elapsed();
        assert_eq!(found, hit, "lookup result");
        if hit { hits.push(e) } else { misses.push(e) }
    }
    drop(store); // SQLite checkpoints its WAL on close
    let (len, alloc) = file_size(path);
    Ok(format!(
        "{}\t{value_len}\t{}\t{len}\t{alloc}\t{}\t{:.1}\t{}\t{}",
        path.extension().unwrap().to_string_lossy(),
        ids.len(),
        len / ids.len() as u64,
        write.as_secs_f64() * 1e6 / ids.len() as f64,
        pct(&mut hits),
        pct(&mut misses)
    ))
}

/// Apparent length and allocated bytes of the store file and SQLite's
/// `-wal` and `-shm` beside it.
fn file_size(path: &Path) -> (u64, u64) {
    let mut total = (0, 0);
    for suffix in ["", "-wal", "-shm"] {
        let mut p = path.as_os_str().to_owned();
        p.push(suffix);
        if let Ok(m) = std::fs::metadata(&p) {
            total.0 += m.len();
            total.1 += m.blocks() * 512;
        }
    }
    total
}

fn main() -> R<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(out) = args.first().map(PathBuf::from) else {
        return Err("usage: fact_cache OUTDIR [--julia-module] [--dump DIR] --history REPO ... [--head DIR ...]".into());
    };
    std::fs::create_dir_all(&out)?;
    let mut ids = Ids::default();
    // Options come between OUTDIR and the first folder.
    let mut rest = 1;
    loop {
        match args.get(rest).map(String::as_str) {
            Some("--julia-module") => ids.julia_module = true,
            Some("--dump") => {
                let dir = args.get(rest + 1).ok_or("--dump needs a directory")?;
                std::fs::create_dir_all(dir)?;
                ids.dump = Some(PathBuf::from(dir));
                rest += 1;
            }
            _ => break,
        }
        rest += 1;
    }
    let mut cache = Cache::default();
    let mut events = std::fs::File::create(out.join("commits.tsv"))?;
    writeln!(events, "folder\tcommit\trev\tlive\tdefs_changed\tlocal\tcache_same\tcache_other\truns")?;
    let mut summary = vec!["event\tfolder\tdefs\tlocal\tcache_same\tcache_other\truns".to_string()];
    let mut folder = 0u32;
    let mut it = args[rest..].iter();
    while let Some(flag) = it.next() {
        let dir = std::fs::canonicalize(it.next().ok_or("missing path")?)?;
        if !dir.is_dir() {
            return Err(format!("{} is not a directory", dir.display()).into());
        }
        let name = dir.file_name().ok_or("a folder needs a name")?.to_string_lossy().to_string();
        let (event, c) = match flag.as_str() {
            "--history" => ("history", history(&dir, &name, folder, &mut ids, &mut cache, &mut events)?),
            "--head" => ("head", head(&dir, folder, &mut ids, &mut cache)?),
            _ => return Err(format!("unknown flag {flag}").into()),
        };
        summary.push(format!("{event}\t{name}\t{}\t{}\t{}\t{}\t{}", c.defs, c.local, c.cache_same_folder, c.cache_other_folder, c.runs()));
        folder += 1;
        if folder >= 31 {
            return Err("at most 31 folders".into());
        }
    }
    summary.push(String::new());
    summary.push(format!(
        "distinct ids (cache entries per fact kind): {}\n\
         distinct (language, text) pairs: {}; files refused whole by the frontend: {}; definitions refused: {}\n\
         frontend time {:.2} s for {} definitions: {:.1} µs per definition",
        cache.seen.len(),
        ids.by_key.keys().filter(|k| !k.starts_with("julia-module:")).count(),
        ids.refused_files,
        ids.refused_defs,
        ids.frontend.as_secs_f64(),
        ids.defs,
        ids.frontend.as_secs_f64() * 1e6 / ids.defs.max(1) as f64
    ));

    for (lang, (texts, defs, refused)) in &ids.per_lang {
        summary.push(format!("{lang}: {texts} distinct texts, {defs} definitions with an id, {refused} refused"));
    }
    if ids.julia_module {
        summary.push(format!(
            "julia module mode: {} distinct Julia file sets, {} definitions with an id, {} refused, {} sets refused whole, \
             resolver time {:.2} s",
            ids.module_sets.len(),
            ids.module_defs,
            ids.module_refused_defs,
            ids.module_refused_sets,
            ids.module_frontend.as_secs_f64()
        ));
    }

    // The cache store at that size: SQLite and redb, three value sizes.
    let mut all: Vec<Digest> = cache.seen.keys().copied().collect();
    all.sort();
    if !all.is_empty() {
        summary.push(String::new());
        summary.push(
            "backend\tvalue_bytes\tfacts\tfile_bytes\tallocated_bytes\tfile_bytes_per_fact\twrite_us_per_fact\t\
             hit_p50_us\thit_p90_us\thit_p99_us\tmiss_p50_us\tmiss_p90_us\tmiss_p99_us"
                .into(),
        );
        for value_len in [8usize, 72, 1024] {
            let p = out.join(format!("cache-{value_len}.sqlite"));
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{suffix}", p.display()));
            }
            summary.push(lookup(Store::new(sophia_store::sqlite::Sqlite::open(&p)?), &p, &all, value_len)?);
            let p = out.join(format!("cache-{value_len}.redb"));
            let _ = std::fs::remove_file(&p);
            summary.push(lookup(Store::new(sophia_store::redb_store::Redb::open(&p)?), &p, &all, value_len)?);
        }
    }
    let text = summary.join("\n");
    std::fs::write(out.join("summary.tsv"), &text)?;
    println!("{text}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree `path -> key` for these texts, as `commit_tree` builds it.
    fn tree(ids: &mut Ids, files: &[(&str, &str)]) -> HashMap<String, String> {
        let mut t: HashMap<String, String> = files.iter().map(|(p, text)| (p.to_string(), ids.key("julia", text))).collect();
        ids.module_keys(&mut t).unwrap();
        t
    }

    fn ids_of(ids: &Ids, t: &HashMap<String, String>, path: &str) -> Vec<Digest> {
        ids.by_key[&t[path]].clone()
    }

    /// B-094: module-wide, a change to `f` in a.jl changes the id of `g` in
    /// the unchanged b.jl, and deleting a.jl changes it again; per file,
    /// b.jl's ids never move. Deleting b.jl leaves no live ids.
    #[test]
    fn module_mode_changes_ids_of_unchanged_files() {
        let (a1, a2, b) = ("f() = 1\n", "f() = 2\n", "g() = f()\n");
        let mut per = Ids::default();
        let mut m = Ids { julia_module: true, ..Ids::default() };
        let t1 = tree(&mut m, &[("a.jl", a1), ("b.jl", b)]);
        let t2 = tree(&mut m, &[("a.jl", a2), ("b.jl", b)]);
        let t3 = tree(&mut m, &[("b.jl", b)]);
        let (g1, g2, g3) = (ids_of(&m, &t1, "b.jl"), ids_of(&m, &t2, "b.jl"), ids_of(&m, &t3, "b.jl"));
        assert_eq!(g1.len(), 1);
        assert_ne!(g1, g2, "g's id must follow f's body module-wide");
        assert_ne!(g2, g3, "g's id must change when f's file goes");
        let p1 = tree(&mut per, &[("a.jl", a1), ("b.jl", b)]);
        let p2 = tree(&mut per, &[("a.jl", a2), ("b.jl", b)]);
        assert_eq!(ids_of(&per, &p1, "b.jl"), ids_of(&per, &p2, "b.jl"), "per file, b.jl's ids do not move");

        // The folder model on these keys: b.jl counts as changed when only a.jl changed.
        let mut f = Folder::default();
        let mut cache = Cache::default();
        f.apply(0, &t1, &m, &mut cache);
        let c = f.apply(0, &t2, &m, &mut cache);
        assert_eq!(c.defs, 2, "both files re-identified");
        assert_eq!(c.runs(), 2, "f and g both have new ids");
        let t4 = tree(&mut m, &[]);
        f.apply(0, &t4, &m, &mut cache);
        assert!(f.live.is_empty());
    }

    /// A path with a hidden segment is no source file, as in Moonkale.
    #[test]
    fn hidden_paths_are_skipped() {
        assert_eq!(lang_of("src/a.jl"), Some("julia"));
        assert_eq!(lang_of(".hidden.jl"), None);
        assert_eq!(lang_of("src/.cache/a.jl"), None);
    }
}
