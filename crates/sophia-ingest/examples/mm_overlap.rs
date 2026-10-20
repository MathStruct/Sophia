//! How much do set.mm and iset.mm share? Statement ids by label, statement
//! ids anywhere, and identical proofs. Prints the first divergences.
use std::collections::HashMap;

fn main() {
    let dir = std::env::args().nth(1).expect("dir with set.mm and iset.mm");
    let set = sophia_ingest::metamath::parse(&std::fs::read_to_string(format!("{dir}/set.mm")).unwrap()).unwrap();
    let iset = sophia_ingest::metamath::parse(&std::fs::read_to_string(format!("{dir}/iset.mm")).unwrap()).unwrap();
    let by_label: HashMap<&str, &sophia_ingest::metamath::Assertion> = set.iter().map(|a| (a.label.as_str(), a)).collect();
    let set_ids: std::collections::HashSet<_> = set.iter().map(|a| a.record.id).collect();
    let (mut same_label, mut same_id, mut same_stmt_diff_id, mut kinds) = (0, 0, 0, HashMap::new());
    let mut shown = 0;
    for a in &iset {
        let Some(b) = by_label.get(a.label.as_str()) else { continue };
        same_label += 1;
        if a.record.id == b.record.id {
            same_id += 1;
            continue;
        }
        let stmt_same = false;
        if stmt_same {
            same_stmt_diff_id += 1;
        }
        *kinds.entry((a.record.kind.clone(), stmt_same)).or_insert(0) += 1;
        if shown < 12 {
            shown += 1;
            println!("{:<12} {:<10} statement {}", a.label, a.record.kind, if stmt_same { "same, proof/refs differ" } else { "differs" });
        }
    }
    println!("labels in both: {same_label}; same statement id: {same_id}; different statement: {}", same_label - same_id);
    let _ = same_stmt_diff_id;
    let set_proofs: std::collections::HashSet<_> = set.iter().filter_map(|a| a.proof.as_ref().map(|p| p.id)).collect();
    let shared_proofs = iset.iter().filter_map(|a| a.proof.as_ref()).filter(|p| set_proofs.contains(&p.id)).count();
    println!("iset proofs identical to a set proof: {shared_proofs} of {}", iset.iter().filter(|a| a.proof.is_some()).count());
    println!("iset ids found anywhere in set: {}", iset.iter().filter(|a| set_ids.contains(&a.record.id)).count());
    let mut k: Vec<_> = kinds.into_iter().collect();
    k.sort();
    for ((kind, same), n) in k {
        println!("  {kind:<10} statement {}: {n}", if same { "same" } else { "differs" });
    }
}
