//! How much of a folder store no current file reaches (SPEC-009, the
//! measurement before garbage collection). Read only.
//!
//! cargo run --release -p sophia-store --example folder_garbage -- STORE.sqlite [IDS.tsv]
//!
//! With IDS.tsv, writes `state<TAB>hex id` for every record not live.

use std::collections::{BTreeMap, HashSet};

use sophia_hash::Digest;
use sophia_store::folder::FileManifest;
use sophia_store::sqlite::Sqlite;
use sophia_store::*;

#[derive(Default)]
struct Tally {
    records: u64,
    bytes: u64,
}

impl Tally {
    fn add(&mut self, r: &sophia_hash::Record) {
        self.records += 1;
        self.bytes += (r.bytes.len() + r.display.len() + 32 * (r.refs.len() + r.nominal_refs.len())) as u64;
    }
}

fn main() -> Result<(), StoreError> {
    let path = std::env::args().nth(1).expect("usage: folder_garbage STORE.sqlite");
    let store = Store::new(Sqlite::open(&path)?);
    store.read(|tx| {
        // Roots: current file manifests, names and current memberships.
        let mut live: HashSet<Digest> = HashSet::new();
        let mut stack: Vec<Digest> = Vec::new();
        let files = tx.meta_prefix("file/")?;
        for (_, id) in &files {
            let id = Digest::from_slice(id).ok_or_else(|| StoreError::Codec("file key".into()))?;
            stack.push(id);
            let r = tx.record(&id)?.ok_or_else(|| StoreError::NotFound(id.to_hex()))?;
            let m = FileManifest::decode(&r.bytes)?;
            stack.extend(m.source);
            stack.extend(m.names.iter().map(|(_, d)| *d));
            stack.extend(m.generics.iter().flat_map(|(_, _, ms)| ms.iter().copied()));
            stack.extend(m.spans.iter().map(|(d, _, _)| *d));
        }
        let names = tx.names_under("", usize::MAX)?;
        for n in &names {
            match n.target {
                Target::Content(d) => stack.push(d),
                Target::Nominal(g) => stack.extend(tx.relation(&g)?.into_iter().map(|e| e.member)),
            }
        }
        while let Some(d) = stack.pop() {
            if !live.insert(d) {
                continue;
            }
            if let Some(r) = tx.record(&d)? {
                stack.extend(r.refs.iter().copied());
            }
        }
        // Lineage history of live records: kept if history is a root.
        let mut history: HashSet<Digest> = HashSet::new();
        for d in &live {
            for l in tx.lineages_of(d)? {
                for e in tx.lineage(&l)? {
                    if !live.contains(&e.id) {
                        history.insert(e.id);
                    }
                }
            }
        }
        // Every record, by category.
        let all = tx.records_of_kind(None, usize::MAX)?;
        let mut by: BTreeMap<(&str, String), Tally> = BTreeMap::new();
        let mut facts = (0u64, 0u64);
        let mut ids = String::new();
        for d in &all {
            let r = tx.record(d)?.ok_or_else(|| StoreError::NotFound(d.to_hex()))?;
            let state = if live.contains(d) {
                "live"
            } else if history.contains(d) {
                "lineage-only"
            } else {
                "unreached"
            };
            let class = match (r.lang.as_str(), r.kind.as_str()) {
                ("sophia-ingest", k) => format!("manifest ({k})"),
                ("source", _) => "source text".to_string(),
                _ => "definition".to_string(),
            };
            by.entry((state, class)).or_default().add(&r);
            if state != "live" {
                ids.push_str(&format!("{state}\t{}\n", d.to_hex()));
            }
            let n = tx.facts_of(d)?.len() as u64;
            if live.contains(d) { facts.0 += n } else { facts.1 += n }
        }
        let log = tx.log_since(0, usize::MAX)?;
        let mut ops: BTreeMap<String, u64> = BTreeMap::new();
        for e in &log {
            *ops.entry(format!("{:?}", e.op)).or_default() += 1;
        }
        println!("store: {path}");
        println!("files: {}, names: {}, records: {}, log entries: {}", files.len(), names.len(), all.len(), log.len());
        println!("{:<13} {:<28} {:>9} {:>14}", "state", "class", "records", "payload bytes");
        let (mut tr, mut tb) = (BTreeMap::<&str, u64>::new(), BTreeMap::<&str, u64>::new());
        for ((state, class), t) in &by {
            println!("{state:<13} {class:<28} {:>9} {:>14}", t.records, t.bytes);
            *tr.entry(state).or_default() += t.records;
            *tb.entry(state).or_default() += t.bytes;
        }
        for (state, r) in &tr {
            println!("{state:<13} {:<28} {r:>9} {:>14}", "total", tb[state]);
        }
        println!("facts about live records: {}, about the rest: {}", facts.0, facts.1);
        println!("log by op: {ops:?}");
        if let Some(out) = std::env::args().nth(2) {
            std::fs::write(out, ids).map_err(|e| StoreError::Codec(e.to_string()))?;
        }
        Ok(())
    })
}
