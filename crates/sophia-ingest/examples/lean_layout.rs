//! M-1 and M-2 of Storage Measurements (B-033): how small a SQLite store
//! gets when it keeps only what cannot be rebuilt, and what rebuilding
//! costs. Works on copies; the input store is opened read-only. See
//! `lean_layout.md`.
//!
//! ```sh
//! cargo run --release -p sophia-ingest --example lean_layout -- STORE.sqlite OUTDIR
//! ```

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use moonkale_content_id::julia::{self, ModuleFile};
use moonkale_content_id::{Budget, Syntax, Tree, definition, is_definition, parse};
use rusqlite::{Connection, OpenFlags, params};
use sophia_hash::{Digest, Record};
use sophia_ingest::source::{JULIA_MODULE_FRONTEND, MANIFEST_LANG, METAMATH_FRONTEND, Manifest, SOURCE_LANG};
use sophia_ingest::{metamath, module};

type R<T> = Result<T, Box<dyn std::error::Error>>;

/// The layouts, each a transformation of the one before.
const STEPS: &[(&str, &str)] = &[
    ("L0", "as stored (vacuumed copy)"),
    ("L1", "reference blobs dropped from records (edge tables kept)"),
    ("L2", "canonical bytes and display dropped for records a manifest rebuilds"),
    ("L3", "integer keys for edges; strings by dictionary; component only for SCCs"),
    ("L4", "ingest log collapsed to one entry; sources compressed (zstd -19)"),
    ("L5", "floor: ingest-derived names, lineage, annotations, method tables, facts dropped"),
];

fn main() -> R<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // `--verify LEAN.sqlite L0.sqlite`: only the verification, e.g. of a
    // deliberately damaged copy (it must fail).
    if let [flag, lean, l0, ..] = args.as_slice()
        && flag == "--verify"
    {
        let l0 = Connection::open_with_flags(l0, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        return verify(&Connection::open_with_flags(lean, OpenFlags::SQLITE_OPEN_READ_ONLY)?, &l0, lean);
    }
    let [store, out, ..] = args.as_slice() else {
        return Err("usage: lean_layout STORE.sqlite OUTDIR".into());
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out)?;
    // The outputs are deleted and rewritten: the input must not be among them.
    let input = std::fs::canonicalize(store)?;
    if input.parent() == Some(std::fs::canonicalize(&out)?.as_path()) {
        return Err("STORE must not be in OUTDIR".into());
    }

    // ---- M-1: the ladder of layouts ----
    let mut sizes = Vec::new();
    let mut prev: Option<PathBuf> = None;
    for (name, what) in STEPS {
        let path = out.join(format!("{name}.sqlite"));
        let _ = std::fs::remove_file(&path);
        match &prev {
            None => {
                // `immutable=1`: no lock and no `-shm` file is written beside the input.
                let uri = format!("file:{}?immutable=1", input.display());
                let c = Connection::open_with_flags(&uri, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI)?;
                c.execute("VACUUM INTO ?1", [path.to_str().ok_or("path")?])?;
            }
            Some(p) => {
                std::fs::copy(p, &path)?;
            }
        }
        let c = Connection::open(&path)?;
        c.execute_batch("PRAGMA journal_mode = DELETE;")?;
        match *name {
            "L0" => {}
            "L1" => l1(&c)?,
            "L2" => l2(&c)?,
            "L3" => l3(&c)?,
            "L4" => l4(&c)?,
            "L5" => l5(&c)?,
            _ => unreachable!(),
        }
        c.execute_batch("VACUUM;")?;
        let bytes = std::fs::metadata(&path)?.len();
        println!("\n== {name}: {what}\n   file {:.2} MiB", mib(bytes));
        for (t, kib) in dbstat(&c)?.iter().take(12) {
            println!("   {t:<32} {:>10.1} KiB", *kib as f64 / 1024.0);
        }
        sizes.push((name, bytes));
        prev = Some(path);
    }
    let base = sizes[0].1 as f64;
    println!("\n== M-1 summary (factor against L0)");
    for (name, b) in &sizes {
        println!("   {name}  {:>9.2} MiB  {:>6.2}x", mib(*b), base / *b as f64);
    }

    // ---- The alternative to L2: keep canonical bytes, compressed ----
    compressed_bytes(&out.join("L0.sqlite"))?;

    // ---- Verify L4 and L5 from their own contents ----
    let l0 = Connection::open_with_flags(out.join("L0.sqlite"), OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    for name in ["L4", "L5"] {
        let c = Connection::open(out.join(format!("{name}.sqlite")))?;
        verify(&c, &l0, name)?;
    }

    // ---- M-2: rebuild costs, from L5's sources ----
    let c = Connection::open(out.join("L5.sqlite"))?;
    let (m, texts) = sources(&c)?;
    match m.frontend.as_str() {
        JULIA_MODULE_FRONTEND => julia_costs(&m, &texts)?,
        METAMATH_FRONTEND => metamath_costs(&texts[0])?,
        f => return Err(format!("unknown frontend {f}").into()),
    }
    Ok(())
}

fn mib(b: u64) -> f64 {
    b as f64 / 1048576.0
}

fn dbstat(c: &Connection) -> R<Vec<(String, i64)>> {
    let mut s = c.prepare("SELECT name, sum(pgsize) FROM dbstat GROUP BY name ORDER BY 2 DESC")?;
    let v = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
    Ok(v)
}

/// L1: the `refs` and `nominal_refs` blobs repeat `t_ref` and
/// `t_nominal_ref` (SP-011 option 2). `verify` checks that the edge
/// tables alone give every rebuilt record's references.
fn l1(c: &Connection) -> R<()> {
    c.execute_batch("UPDATE t_definition SET refs = x'', nominal_refs = x'';")?;
    Ok(())
}

/// L2: drop what the manifest's frontend rebuilds. Every store measured
/// here holds exactly one ingest, so every record that is neither a source
/// nor a manifest is covered; `verify` checks that.
fn l2(c: &Connection) -> R<()> {
    c.execute("UPDATE t_definition SET bytes = x'', display = x'' WHERE lang NOT IN (?1, ?2)", params![SOURCE_LANG, MANIFEST_LANG])?;
    Ok(())
}

/// L3: one integer key per id (the record table's rowid); edges as
/// integer pairs with a reverse index (SP-011 options 1 and 4); language,
/// normaliser, kind and status by dictionary; the component digest kept
/// only for multi-member components (for a singleton it is derivable from
/// the bytes and says nothing a query uses).
fn l3(c: &Connection) -> R<()> {
    c.execute_batch(
        r#"
        CREATE TABLE t_dict(k INTEGER PRIMARY KEY, s TEXT NOT NULL UNIQUE);
        INSERT OR IGNORE INTO t_dict(s) SELECT lang FROM t_definition;
        INSERT OR IGNORE INTO t_dict(s) SELECT normaliser FROM t_definition;
        INSERT OR IGNORE INTO t_dict(s) SELECT kind FROM t_definition;
        INSERT OR IGNORE INTO t_dict(s) SELECT status FROM t_definition;
        CREATE TABLE t_def(
          k INTEGER PRIMARY KEY, id BLOB NOT NULL UNIQUE, lang INTEGER NOT NULL, normaliser INTEGER NOT NULL,
          kind INTEGER NOT NULL, status INTEGER NOT NULL, component BLOB, idx INTEGER NOT NULL, size INTEGER NOT NULL,
          bytes BLOB NOT NULL, display BLOB NOT NULL);
        INSERT INTO t_def(k, id, lang, normaliser, kind, status, component, idx, size, bytes, display)
          SELECT d.rowid, d.id, (SELECT k FROM t_dict WHERE s = d.lang), (SELECT k FROM t_dict WHERE s = d.normaliser),
                 (SELECT k FROM t_dict WHERE s = d.kind), (SELECT k FROM t_dict WHERE s = d.status),
                 CASE WHEN d.size > 1 THEN d.component END, d.idx, d.size, d.bytes, d.display
          FROM t_definition d;
        -- Edge targets not stored as records get a key too (none in the corpora measured).
        INSERT INTO t_def(id, lang, normaliser, kind, status, idx, size, bytes, display)
          SELECT DISTINCT r.dst, 0, 0, 0, 0, 0, 0, x'', x'' FROM t_ref r WHERE r.dst NOT IN (SELECT id FROM t_definition);
        CREATE TABLE t_edge(src INTEGER NOT NULL, dst INTEGER NOT NULL, PRIMARY KEY(src, dst)) WITHOUT ROWID;
        INSERT INTO t_edge SELECT s.k, d.k FROM t_ref r JOIN t_def s ON s.id = r.src JOIN t_def d ON d.id = r.dst;
        CREATE INDEX t_edge_dst ON t_edge(dst, src);
        CREATE TABLE t_nominal_edge(src INTEGER NOT NULL, dst BLOB NOT NULL, PRIMARY KEY(src, dst)) WITHOUT ROWID;
        INSERT INTO t_nominal_edge SELECT s.k, r.dst FROM t_nominal_ref r JOIN t_def s ON s.id = r.src;
        CREATE INDEX t_def_component ON t_def(component, idx) WHERE component IS NOT NULL;
        CREATE INDEX t_def_kind ON t_def(kind);
        DROP VIEW IF EXISTS definitions;
        DROP VIEW IF EXISTS refs;
        DROP TABLE t_ref;
        DROP TABLE t_nominal_ref;
        DROP TABLE t_definition;
        "#,
    )?;
    Ok(())
}

/// L4: the ingest is one deterministic transformation (SPEC-006 §6a), so
/// its log is one entry naming the manifest; source texts compressed with
/// the zstd command at level 19, one blob per file.
fn l4(c: &Connection) -> R<()> {
    let manifest: Vec<u8> =
        c.query_row("SELECT d.id FROM t_def d JOIN t_dict l ON l.k = d.lang WHERE l.s = ?1", [MANIFEST_LANG], |r| r.get(0))?;
    c.execute("DELETE FROM t_log", [])?;
    c.execute(
        "INSERT INTO t_log(seq, op, subject, old, new, actor, at) VALUES (1, 'ingest', 'manifest', NULL, ?1, 'lean_layout', 0)",
        [manifest],
    )?;
    let rows: Vec<(i64, Vec<u8>)> = {
        let mut s = c.prepare("SELECT d.k, d.bytes FROM t_def d JOIN t_dict l ON l.k = d.lang WHERE l.s = ?1")?;
        s.query_map([SOURCE_LANG], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?
    };
    for (k, b) in rows {
        c.execute("UPDATE t_def SET bytes = ?1 WHERE k = ?2", params![zstd(&b, &["-19", "-q", "-c"])?, k])?;
    }
    Ok(())
}

/// L5: everything else the ingest derives from the source. Queries by
/// name need `t_name`, so this is a lower bound, not a proposal. Nominal
/// entities stay: records' `nominal_refs` bind to them (store state).
fn l5(c: &Connection) -> R<()> {
    c.execute_batch(
        "DELETE FROM t_name WHERE target NOT IN
           (SELECT d.id FROM t_def d JOIN t_dict l ON l.k = d.lang WHERE l.s IN ('source', 'sophia-ingest'));
         DELETE FROM t_lineage; DELETE FROM t_annotation; DELETE FROM t_annotation_subject;
         DELETE FROM t_relation; DELETE FROM t_fact;",
    )?;
    Ok(())
}

/// Records per compressed block in `compressed_bytes`.
const BLOCK: usize = 256;

/// What L2 drops (canonical bytes and display of rebuildable records), if
/// it were kept instead, compressed with zstd -19 in blocks of `BLOCK`
/// records in insertion order (a block is the unit one read decompresses).
/// Also the whole column as one stream, the lower bound.
fn compressed_bytes(l0: &Path) -> R<()> {
    let c = Connection::open_with_flags(l0, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut s = c.prepare("SELECT bytes, display FROM t_definition WHERE lang NOT IN (?1, ?2) ORDER BY rowid")?;
    let rows: Vec<(Vec<u8>, Vec<u8>)> =
        s.query_map(params![SOURCE_LANG, MANIFEST_LANG], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
    let (mut raw, mut blocks, mut all) = (0usize, 0usize, Vec::new());
    for chunk in rows.chunks(BLOCK) {
        let mut b = Vec::new();
        for (x, d) in chunk {
            b.extend_from_slice(x);
            b.extend_from_slice(d);
        }
        raw += b.len();
        blocks += zstd(&b, &["-19", "-q", "-c"])?.len();
        all.extend_from_slice(&b);
    }
    let whole = zstd(&all, &["-19", "-q", "-c", "--long=27"])?.len();
    println!(
        "\n== canonical bytes + display of {} rebuildable records: {:.2} MiB raw; zstd -19 in blocks of {BLOCK}: {:.2} MiB ({:.1}x); as one stream: {:.2} MiB ({:.1}x)",
        rows.len(),
        mib(raw as u64),
        mib(blocks as u64),
        raw as f64 / blocks as f64,
        mib(whole as u64),
        raw as f64 / whole as f64
    );
    Ok(())
}

fn zstd(input: &[u8], args: &[&str]) -> R<Vec<u8>> {
    let mut p = Command::new("zstd").args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    let mut stdin = p.stdin.take().ok_or("stdin")?;
    let data = input.to_vec();
    let w = std::thread::spawn(move || stdin.write_all(&data));
    let o = p.wait_with_output()?;
    w.join().map_err(|_| "writer")??;
    if !o.status.success() {
        return Err("zstd failed".into());
    }
    Ok(o.stdout)
}

/// The manifest and its source texts, from a lean store (L4 or L5).
fn sources(c: &Connection) -> R<(Manifest, Vec<String>)> {
    let n: i64 = c.query_row("SELECT count(*) FROM t_def d JOIN t_dict l ON l.k = d.lang WHERE l.s = ?1", [MANIFEST_LANG], |r| r.get(0))?;
    if n != 1 {
        return Err(format!("the store holds {n} manifests; this program needs exactly one").into());
    }
    let mb: Vec<u8> =
        c.query_row("SELECT d.bytes FROM t_def d JOIN t_dict l ON l.k = d.lang WHERE l.s = ?1", [MANIFEST_LANG], |r| r.get(0))?;
    let m = Manifest::decode(&mb)?;
    let mut texts = Vec::new();
    for f in &m.files {
        let z: Vec<u8> = c.query_row("SELECT bytes FROM t_def WHERE id = ?1", [f.source.as_bytes().as_slice()], |r| r.get(0))?;
        let t = zstd(&z, &["-d", "-q", "-c"])?;
        // The source record's id is the hash of its uncompressed text.
        let rec = sophia_ingest::source::source_record(&f.path, std::str::from_utf8(&t)?);
        if rec.id != f.source {
            return Err(format!("source {} does not match its id after decompression", f.path).into());
        }
        texts.push(String::from_utf8(t)?);
    }
    if texts.is_empty() {
        return Err("the manifest names no source".into());
    }
    Ok((m, texts))
}

fn rebuild(m: &Manifest, texts: &[String]) -> R<Vec<Record>> {
    Ok(match m.frontend.as_str() {
        JULIA_MODULE_FRONTEND => {
            module::ingest(&discovered(m, texts))?.files.into_iter().flat_map(|(_, v)| v.into_iter().map(|(_, r)| r)).collect()
        }
        METAMATH_FRONTEND => metamath::parse(&texts[0])?.into_iter().flat_map(|a| std::iter::once(a.record).chain(a.proof)).collect(),
        f => return Err(format!("unknown frontend {f}").into()),
    })
}

fn discovered(m: &Manifest, texts: &[String]) -> module::Discovered {
    module::Discovered {
        files: m
            .files
            .iter()
            .zip(texts)
            .map(|(f, t)| module::SourceFile { path: f.path.clone(), text: t.clone(), base_module: f.base_module.clone() })
            .collect(),
        dynamic: m.dynamic.iter().map(|(i, p)| (*i as usize, p.clone())).collect(),
    }
}

/// Rebuilds every record from the lean store's own sources and checks that
/// the lean store still holds, for each: its id (and the rebuilt bytes hash
/// to it), language, normaliser, kind, status, component when it has
/// several members, and its references as edges; and that no stored
/// definition is left that the rebuild does not produce.
fn verify(c: &Connection, l0: &Connection, name: &str) -> R<()> {
    let t = Instant::now();
    let (m, texts) = sources(c)?;
    let rebuilt = rebuild(&m, &texts)?;
    let rebuild_time = t.elapsed();
    let mut by_id: BTreeMap<Digest, Record> = BTreeMap::new();
    for r in rebuilt {
        by_id.entry(r.id).or_insert(r);
    }
    // The dropped payloads: the rebuilt bytes and display equal what L0 held.
    let mut payload_differs = 0usize;
    {
        let mut s = l0.prepare("SELECT id, bytes, display FROM t_definition WHERE lang NOT IN (?1, ?2)")?;
        let rows = s.query_map(params![SOURCE_LANG, MANIFEST_LANG], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?, r.get::<_, Vec<u8>>(2)?))
        })?;
        for row in rows {
            let (id, bytes, display) = row?;
            let id = Digest(id.try_into().map_err(|_| "id")?);
            match by_id.get(&id) {
                Some(r) if r.bytes == bytes && r.display == display => {}
                _ => payload_differs += 1,
            }
        }
    }
    // The authority records are checked like the rest: the manifest (its id
    // from its decoded content, its edges to the sources) and each source.
    let authority = texts.len() + 1;
    for (f, t) in m.files.iter().zip(&texts) {
        let r = sophia_ingest::source::source_record(&f.path, t);
        by_id.insert(r.id, r);
    }
    let mr = m.record();
    by_id.insert(mr.id, mr);
    // Rebuilt bytes give the ids (Record::verify, per component).
    let mut comps: HashMap<Digest, Vec<&Record>> = HashMap::new();
    for r in by_id.values() {
        comps.entry(r.component).or_default().push(r);
    }
    // Records under Moonkale's normaliser carry Moonkale's ids, which
    // `Record::verify` (Sophia's own framing) does not recompute; for those
    // the rebuild's ids come from the crate itself.
    let (mut bad_hash, mut hash_checked) = (0, 0);
    for v in comps.values_mut() {
        v.sort_by_key(|r| r.index);
        if v[0].normaliser.starts_with("moonkale-") {
            continue;
        }
        hash_checked += v.len();
        if !Record::verify(v) {
            bad_hash += v.len();
        }
    }
    let dict: HashMap<i64, String> = {
        let mut s = c.prepare("SELECT k, s FROM t_dict")?;
        s.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?
    };
    let key: HashMap<i64, Digest> = {
        let mut s = c.prepare("SELECT k, id FROM t_def")?;
        s.query_map([], |r| Ok((r.get(0)?, r.get::<_, Vec<u8>>(1)?)))?
            .map(|x| x.map(|(k, id): (i64, Vec<u8>)| (k, Digest(id.try_into().expect("32-byte id")))))
            .collect::<Result<_, _>>()?
    };
    let mut edges: HashMap<Digest, BTreeSet<Digest>> = HashMap::new();
    {
        let mut s = c.prepare("SELECT src, dst FROM t_edge")?;
        for e in s.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))? {
            let (a, b) = e?;
            edges.entry(key[&a]).or_default().insert(key[&b]);
        }
    }
    // `nominal_refs` bind to nominal entities, which are store state, not
    // rebuilt by the frontend (as in `source::first_difference`): kept, not compared.
    let mut stored = 0usize;
    let mut differ: BTreeMap<&str, usize> = BTreeMap::new();
    let mut extra = 0usize;
    // Every row but the stubs L3 adds for edge targets that are not records.
    let stubs: i64 = c.query_row("SELECT count(*) FROM t_def WHERE size = 0", [], |r| r.get(0))?;
    let mut s = c.prepare("SELECT id, lang, normaliser, kind, status, component, idx, size FROM t_def WHERE size > 0")?;
    let rows = s.query_map([], |r| {
        Ok((
            r.get::<_, Vec<u8>>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, Option<Vec<u8>>>(5)?,
            r.get::<_, i64>(6)?,
            r.get::<_, i64>(7)?,
        ))
    })?;
    let mut seen = BTreeSet::new();
    for row in rows {
        let (id, lang, norm, kind, status, comp, idx, size) = row?;
        let id = Digest(id.try_into().map_err(|_| "id")?);
        stored += 1;
        seen.insert(id);
        let Some(r) = by_id.get(&id) else {
            extra += 1;
            continue;
        };
        let mut d = |f| *differ.entry(f).or_default() += 1;
        if dict[&lang] != r.lang {
            d("lang");
        }
        if dict[&norm] != r.normaliser {
            d("normaliser");
        }
        if dict[&kind] != r.kind {
            d("kind");
        }
        if dict[&status] != r.status.as_str() {
            d("status");
        }
        if idx != r.index as i64 || size != r.component_size as i64 {
            d("index/size");
        }
        if r.component_size > 1 && comp.as_deref() != Some(r.component.as_bytes().as_slice()) {
            d("component");
        }
        let want: BTreeSet<Digest> = r.refs.iter().copied().collect();
        if edges.get(&id).cloned().unwrap_or_default() != want {
            d("refs");
        }
    }
    let missing = by_id.keys().filter(|k| !seen.contains(k)).count();
    // Nominal references are not rebuilt (store state), so they are compared
    // with L0's table instead: the same pairs.
    let nominal_lean: BTreeSet<(Digest, Vec<u8>)> = {
        let mut s = c.prepare("SELECT src, dst FROM t_nominal_edge")?;
        s.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?)))?
            .map(|x| x.map(|(a, b)| (key[&a], b)))
            .collect::<Result<_, _>>()?
    };
    let nominal_l0: BTreeSet<(Digest, Vec<u8>)> = {
        let mut s = l0.prepare("SELECT src, dst FROM t_nominal_ref")?;
        s.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?)))?
            .map(|x| x.map(|(a, b)| (Digest(a.try_into().expect("32-byte id")), b)))
            .collect::<Result<_, _>>()?
    };
    let nominal_ok = nominal_lean == nominal_l0;
    println!(
        "\n== verify {name}: {} sources, {} records rebuilt in {:.2?} (decompression included), plus {authority} sources and manifest; \
         {} stored records ({stubs} edge-target stubs not counted); \
         {} rebuilt but not stored; {} stored but not rebuilt; {} of {} hash-checked records whose rebuilt bytes do not give the id; \
         {payload_differs} of L0's dropped payloads (bytes, display) not reproduced; nominal edges equal to L0's: {nominal_ok}; differing fields: {:?}",
        texts.len(),
        by_id.len() - authority,
        rebuild_time,
        stored,
        missing,
        extra,
        bad_hash,
        hash_checked,
        differ
    );
    if missing + extra + bad_hash + payload_differs > 0 || !differ.is_empty() || !nominal_ok {
        return Err(format!("{name} does not verify").into());
    }
    Ok(())
}

fn median(v: &mut [Duration]) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn quantile(v: &[Duration], q: f64) -> Duration {
    v[((v.len() - 1) as f64 * q).round() as usize]
}

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

const RUNS: usize = 5;

/// M-2 for a Julia module: the phases of a rebuild, each the median of
/// `RUNS` runs, and the per-definition costs.
fn julia_costs(m: &Manifest, texts: &[String]) -> R<()> {
    let d = discovered(m, texts);
    let n_defs = module::ingest(&d)?.files.iter().map(|(_, v)| v.len()).sum::<usize>();
    if n_defs == 0 {
        return Err("no identified definitions".into());
    }
    let (mut whole, mut parse_all, mut resolve) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..RUNS {
        let t = Instant::now();
        let r = module::ingest(&d);
        whole.push(t.elapsed());
        r?;
        let t = Instant::now();
        let trees: Vec<Tree> = texts.iter().map(|x| parse("julia", x, &mut Budget::new())).collect::<Result<_, _>>()?;
        parse_all.push(t.elapsed());
        let files: Vec<ModuleFile> =
            d.files.iter().zip(&trees).map(|(f, t)| ModuleFile { text: &f.text, tree: t, module: f.base_module.clone() }).collect();
        let dynamic: Vec<Vec<String>> = d.dynamic.iter().map(|(_, m)| m.clone()).collect();
        let t = Instant::now();
        let r = julia::module(&files, &dynamic);
        resolve.push(t.elapsed());
        r?;
    }
    // Per file: parse time. Per definition: its WP-66 encoding alone, given
    // the references the module resolution found.
    let trees: Vec<Tree> = texts.iter().map(|x| parse("julia", x, &mut Budget::new())).collect::<Result<_, _>>()?;
    let mut file_parse = Vec::new();
    for x in texts {
        let mut v: Vec<Duration> = (0..RUNS)
            .map(|_| {
                let t = Instant::now();
                let r = parse("julia", x, &mut Budget::new());
                let e = t.elapsed();
                r.map(|_| e)
            })
            .collect::<Result<_, _>>()?;
        file_parse.push(median(&mut v));
    }
    let files: Vec<ModuleFile> =
        d.files.iter().zip(&trees).map(|(f, t)| ModuleFile { text: &f.text, tree: t, module: f.base_module.clone() }).collect();
    let dynamic: Vec<Vec<String>> = d.dynamic.iter().map(|(_, m)| m.clone()).collect();
    let jm = julia::module(&files, &dynamic)?;
    let mut encode = Vec::new();
    let mut with_sidecar = Vec::new();
    for x in jm.definitions.iter().filter(|x| x.identity.is_ok()) {
        let f = x.key.0 as usize;
        let node = node_at(&trees[f], x.key.1, x.key.2).ok_or("definition node not found")?;
        let refs: HashMap<(usize, usize), _> = x.references.iter().copied().collect();
        let mut v: Vec<Duration> = (0..RUNS)
            .map(|_| {
                let mut b = Budget::new();
                let t = Instant::now();
                let r = definition("julia", node, &texts[f], &refs, &mut b);
                let e = t.elapsed();
                r.map(|_| e)
            })
            .collect::<Result<_, _>>()?;
        let e = median(&mut v);
        encode.push(e);
        // A rebuild that had each definition's resolved references stored
        // beside it (a sidecar) would parse one file and encode one definition.
        with_sidecar.push(file_parse[f] + e);
    }
    encode.sort();
    with_sidecar.sort();
    let mut fp = file_parse.clone();
    fp.sort();
    let w = median(&mut whole);
    println!(
        "\n== M-2 Julia: {} files, {} bytes, {} definitions (runs: {RUNS}, medians)",
        texts.len(),
        texts.iter().map(String::len).sum::<usize>(),
        n_defs
    );
    println!("   whole rebuild (module::ingest)          {:>10.2?}   = {:.1?} per definition", w, w / n_defs as u32);
    println!("   of which: parse every file               {:>10.2?}", median(&mut parse_all));
    println!("   of which: module resolution and ids      {:>10.2?}", median(&mut resolve));
    println!("   parse one file: median {:.2?}, p90 {:.2?}, max {:.2?}", quantile(&fp, 0.5), quantile(&fp, 0.9), fp[fp.len() - 1]);
    println!(
        "   encode one definition (WP-66 parts): median {:.1?}, p99 {:.1?}, max {:.1?}, sum {:.2?}",
        quantile(&encode, 0.5),
        quantile(&encode, 0.99),
        encode[encode.len() - 1],
        encode.iter().sum::<Duration>()
    );
    println!(
        "   one definition with a reference sidecar (parse its file + encode): median {:.2?}, p99 {:.2?}, max {:.2?}",
        quantile(&with_sidecar, 0.5),
        quantile(&with_sidecar, 0.99),
        with_sidecar[with_sidecar.len() - 1]
    );
    Ok(())
}

/// M-2 for Metamath: the frontend reads the whole file in one pass (the
/// frames in scope at each statement come from everything before it).
fn metamath_costs(text: &str) -> R<()> {
    let mut v = Vec::new();
    let mut n = 0;
    for _ in 0..3 {
        let t = Instant::now();
        let a = metamath::parse(text);
        v.push(t.elapsed());
        let a = a?;
        n = a.iter().map(|a| 1 + a.proof.is_some() as usize).sum();
    }
    if n == 0 {
        return Err("no assertions".into());
    }
    let w = median(&mut v);
    println!("\n== M-2 Metamath: {} bytes, {} statements and proofs (runs: 3, median)", text.len(), n);
    println!("   whole rebuild (metamath::parse)  {:>10.2?}   = {:.1?} per statement or proof", w, w / n as u32);
    Ok(())
}
