//! `Interface::propose_text` (SPEC-010 §5) and the signature aliases of
//! `export` (§4), against the specification: each test compares a proposal
//! with what an edit through `write_text` / `create_text` does, and checks
//! that a proposal writes nothing (`Store::last_seq` before and after).
//! Written by Mistral as review probes for B-003 (2026-11-17), two probe
//! mistakes corrected by Claude; on SQLite only (`acceptance.rs` runs the
//! core cases on all four backends). See `propose.md`.

use sophia_interface::{EditError, Interface, Key, Severity};
use sophia_store::{StoreError, Target, Visibility};

fn sample() -> (tempfile::TempDir, Interface<sophia_store::sqlite::Sqlite>) {
    let dir = tempfile::tempdir().unwrap();
    let i = Interface::open(sophia_store::sqlite::Sqlite::open(dir.path().join("s")).unwrap(), "daniel").unwrap();
    i.load_sample().unwrap();
    (dir, i)
}

fn span_text<'a>(e: &'a sophia_interface::ExportedFile, name: &str) -> (String, &'a str) {
    let (n, a, b) = e.spans.iter().find(|(n, _, _)| n == name).unwrap().clone();
    (n, &e.text[a as usize..b as usize])
}

/// Acceptance 3 + "writes nothing": export, propose the export back, and
/// check the store's log position and names are untouched.
#[test]
fn roundtrip_proposal_changes_nothing_and_writes_nothing() {
    let (_d, i) = sample();
    let even = i.store().resolve("sample.even").unwrap().unwrap().target;
    i.store().alias("sample.also_even", even, Visibility::Public).unwrap();
    let e = i.export("sample").unwrap();
    // The §4 change: signature names are aliases, not printed.
    assert!(e.aliases.iter().any(|(a, b)| a == "sample.ancestor_sig" && b == "sample.ancestor"), "{e:?}");
    assert!(e.aliases.iter().any(|(a, b)| a == "sample.parent_sig" && b == "sample.ancestor"), "{e:?}");
    assert!(e.aliases.iter().any(|(a, _)| a == "sample.even"), "{e:?}");
    assert!(!e.spans.iter().any(|(n, _, _)| n == "sample.ancestor_sig"));
    let seq = i.store().last_seq().unwrap();
    let before: Vec<(String, Target)> =
        ["sample.add", "sample.double", "sample.even", "sample.odd", "sample.ancestor", "sample.ancestor_sig"]
            .iter()
            .map(|p| (p.to_string(), i.store().resolve(p).unwrap().unwrap().target))
            .collect();
    let p = i.propose_text("sample", &e.text, e.token).unwrap();
    assert!(p.changes_nothing(), "{p:?}");
    assert_eq!(p.proposal.files.len(), 1);
    assert_eq!(p.proposal.files[0].path, "sample");
    assert!(!p.proposal.files[0].text_changed);
    assert!(p.proposal.conflicts.is_empty() && p.proposal.error.is_none() && p.proposal.report.is_some());
    assert_eq!(i.store().last_seq().unwrap(), seq, "a proposal must not write");
    for (path, target) in before {
        assert_eq!(i.store().resolve(&path).unwrap().unwrap().target, target, "{path}");
    }
}

/// Empty text: every printed name is reported removed, nothing is written.
#[test]
fn empty_text_reports_every_name_removed() {
    let (_d, i) = sample();
    let e = i.export("sample").unwrap();
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", "", e.token).unwrap();
    let f = &p.proposal.files[0];
    let mut removed: Vec<&str> = f.names_removed.iter().map(|(n, _)| n.as_str()).collect();
    removed.sort();
    let mut printed: Vec<&str> = e.spans.iter().map(|(n, _, _)| n.as_str()).collect();
    printed.sort();
    assert_eq!(removed, printed, "every printed name is removed; aliases are not: {f:?}");
    assert!(f.names_added.is_empty() && f.names_retargeted.is_empty() && f.members_added.is_empty());
    assert!(p.proposal.error.is_none() && p.proposal.conflicts.is_empty());
    assert!(f.text_changed);
    assert!(!p.changes_nothing());
    assert_eq!(i.store().last_seq().unwrap(), seq);
}

/// One name defined twice: a parse error, reported like `write_text`'s
/// `EditError::Diagnostics`, with no file in the proposal.
#[test]
fn duplicate_name_is_a_parse_error_like_an_edit() {
    let (_d, i) = sample();
    let e = i.export("sample").unwrap();
    let twice = "function double(x::Int64)::Int64\n    x + x\nend\n\nfunction double(x::Int64)::Int64\n    x\nend\n";
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", twice, e.token).unwrap();
    assert!(p.proposal.files.is_empty(), "{p:?}");
    assert!(p.diagnostics.iter().any(|d| d.severity == Severity::Error && d.message.contains("twice")), "{p:?}");
    assert!(!p.changes_nothing());
    assert_eq!(i.store().last_seq().unwrap(), seq);
    let (_, v) = i.fetch_text(&Key::name("sample.double")).unwrap();
    assert!(matches!(i.write_text(&Key::name("sample.double"), v, twice), Err(EditError::Diagnostics(_))));
}

/// A name the file does not hold (a query in the namespace): refused, and
/// the refusal matches what an edit through `create_text` refuses.
#[test]
fn name_claimed_outside_the_file_is_refused_like_an_edit() {
    let (_d, i) = sample();
    let q = "query users\n    select p where name(p, a)\nend";
    i.create_text(&Key::ns("sample"), "users", q).unwrap();
    let e = i.export("sample").unwrap();
    assert!(!e.spans.iter().any(|(n, _, _)| n == "sample.users"));
    let text = format!("{}\nfunction users(x::Nat)::Nat\n    x\nend\n", e.text);
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", &text, e.token).unwrap();
    match &p.proposal.error {
        Some(StoreError::NameClaimed { name, .. }) => {
            assert_eq!(name, "sample.users");
        }
        other => panic!("expected NameClaimed, got {other:?} in {p:?}"),
    }
    assert!(!p.changes_nothing());
    assert_eq!(i.store().last_seq().unwrap(), seq);
    let r = i.create_text(&Key::ns("sample"), "users2", &text[e.text.len()..]);
    assert!(matches!(r, Err(EditError::Refused(_))), "an edit refuses: {r:?}");
}

/// A name that exists in another namespace is defined locally, not refused.
#[test]
fn name_from_another_namespace_is_added_locally() {
    let (_d, i) = sample();
    i.create_text(&Key::ns(""), "lib/g", "function g(x::Nat)::Nat\n    x\nend\n").unwrap();
    let e = i.export("sample").unwrap();
    let text = format!("{}\nfunction g(x::Nat)::Nat\n    x\nend\n", e.text);
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", &text, e.token).unwrap();
    assert!(p.proposal.error.is_none(), "{p:?}");
    assert_eq!(p.proposal.files[0].names_added.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), vec!["sample.g"]);
    assert_eq!(i.store().last_seq().unwrap(), seq);
}

/// A namespace that does not exist: export is an empty file, and a
/// proposal into it reports additions (or nothing), never an error.
#[test]
fn nonexistent_namespace_exports_empty_and_proposes() {
    let (_d, i) = sample();
    let seq = i.store().last_seq().unwrap();
    let e = i.export("ghost").unwrap();
    assert!(e.text.is_empty() && e.spans.is_empty(), "{e:?}");
    let p = i.propose_text("ghost", "", e.token).unwrap();
    assert!(p.changes_nothing(), "{p:?}");
    let p = i.propose_text("ghost", "function f(x::Nat)::Nat\n    x\nend\n", e.token).unwrap();
    assert!(p.proposal.error.is_none() && p.proposal.conflicts.is_empty(), "{p:?}");
    assert_eq!(p.proposal.files[0].names_added.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), vec!["ghost.f"]);
    assert_eq!(i.store().last_seq().unwrap(), seq, "proposing into a ghost namespace writes nothing");
}

/// A stale token is a conflict; the diff is still computed.
#[test]
fn stale_token_conflicts_but_still_diffs() {
    let (_d, i) = sample();
    let e = i.export("sample").unwrap();
    let (n, a, b) = e.spans.iter().find(|(n, _, _)| n == "sample.double").unwrap().clone();
    let (_, v) = i.fetch_text(&Key::name(&n)).unwrap();
    i.write_text(&Key::name(&n), v, &e.text[a as usize..b as usize].replace("x + x", "2 * x")).unwrap();
    let now = i.export("sample").unwrap();
    assert_ne!(now.token, e.token);
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", &e.text, e.token).unwrap();
    assert_eq!(p.proposal.conflicts, vec![("sample".to_string(), e.token, now.token)], "{p:?}");
    assert!(!p.changes_nothing());
    assert!(!p.proposal.files.is_empty());
    assert_eq!(i.store().last_seq().unwrap(), seq);
    let p = i.propose_text("sample", &now.text, now.token).unwrap();
    assert!(p.proposal.conflicts.is_empty() && p.changes_nothing(), "{p:?}");
}

/// Reordering definitions only: no name changes.
#[test]
fn reorder_only_changes_no_name() {
    let (_d, i) = sample();
    let e = i.export("sample").unwrap();
    let mut blocks: Vec<&str> = e.spans.iter().map(|(_, a, b)| &e.text[*a as usize..*b as usize]).collect();
    let mut seen = std::collections::HashSet::new();
    blocks.retain(|b| seen.insert(b.as_ptr())); // a recursive group's names share one span
    blocks.reverse();
    let reordered = blocks.join("\n");
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", &reordered, e.token).unwrap();
    assert!(p.proposal.error.is_none() && p.proposal.conflicts.is_empty(), "{p:?}");
    assert!(p.changes_nothing(), "reordering changes no name: {p:?}");
    assert!(p.proposal.files[0].text_changed);
    assert_eq!(i.store().last_seq().unwrap(), seq);
}

/// A recursive group and its alias: proposing the export back changes
/// nothing; editing the group retargets the printed names (also_even, odd)
/// but not the alias (even), with the ids an edit stores.
#[test]
fn recursive_group_and_its_alias() {
    let (_d, i) = sample();
    let even = i.store().resolve("sample.even").unwrap().unwrap().target;
    i.store().alias("sample.also_even", even, Visibility::Public).unwrap();
    let e = i.export("sample").unwrap();
    let p = i.propose_text("sample", &e.text, e.token).unwrap();
    assert!(p.changes_nothing(), "{p:?}");

    let (group_name, block) = span_text(&e, "sample.also_even");
    assert!(block.contains("function odd"), "the group prints both members: {block}");
    let edited_block = block.replace("zero => true", "zero => true == true");
    let mut edited = e.text.clone();
    let (_, a, b) = e.spans.iter().find(|(n, _, _)| n == &group_name).unwrap().clone();
    edited.replace_range(a as usize..b as usize, &edited_block);
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", &edited, e.token).unwrap();
    assert!(p.proposal.error.is_none() && p.proposal.conflicts.is_empty(), "{p:?}");
    let f = &p.proposal.files[0];
    let mut retargeted: Vec<&str> = f.names_retargeted.iter().map(|(n, _, _)| n.as_str()).collect();
    retargeted.sort();
    assert_eq!(retargeted, vec!["sample.also_even", "sample.odd"], "the alias sample.even is not retargeted: {f:?}");
    assert!(!f.names_removed.iter().any(|(n, _)| n == "sample.even"), "the alias is not removed: {f:?}");
    assert_eq!(i.store().last_seq().unwrap(), seq);

    // The ids the proposal reports are the ids an edit stores.
    let (_, v) = i.fetch_text(&Key::name("sample.also_even")).unwrap();
    let w = i.write_text(&Key::name("sample.also_even"), v, &edited_block).unwrap();
    for (name, _, new) in &f.names_retargeted {
        let stored = w.targets.iter().find(|(p, _)| p == name).map(|(_, id)| *id);
        assert_eq!(Some(*new), stored, "{name}");
    }
}

/// Relations and rules: the export proposes back with no membership
/// changes; a new rule over an existing relation gets the id an edit
/// stores; an edited rule moves its membership.
#[test]
fn relations_and_rules_memberships() {
    let (_d, i) = sample();
    let e = i.export("sample").unwrap();
    let ancestor = i.store().resolve("sample.ancestor").unwrap().unwrap().target.digest();
    let before = i.store().methods(&ancestor).unwrap();
    let p = i.propose_text("sample", &e.text, e.token).unwrap();
    assert!(p.changes_nothing(), "{p:?}");
    assert!(p.proposal.files[0].members_added.is_empty() && p.proposal.files[0].members_removed.is_empty());

    // A new rule over the existing relation.
    let rule = "rule ancestor_extra(X, Y) = ancestor(X, Y)\n";
    let text = format!("{}\n{rule}", e.text);
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", &text, e.token).unwrap();
    let f = &p.proposal.files[0];
    assert!(p.proposal.error.is_none(), "{p:?}");
    assert_eq!(f.names_added.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), vec!["sample.ancestor_extra"]);
    assert_eq!(f.members_added.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), vec!["sample.ancestor"]);
    let proposed_rule = f.names_added[0].1;
    assert!(f.members_added.iter().any(|(_, m)| *m == proposed_rule), "the rule joins its head's relation: {f:?}");
    assert_eq!(i.store().last_seq().unwrap(), seq);
    let (_, w) = i.create_text(&Key::ns("sample"), "ancestor_extra", rule).unwrap();
    assert_eq!(w.targets[0].1, proposed_rule, "the proposal reports the id the edit stores");
    assert!(i.store().methods(&ancestor).unwrap().contains(&proposed_rule));

    // An edited rule: retargeted name, membership removed and added.
    let (_, block) = span_text(&e, "sample.ancestor_base");
    let edited_block = block.replace("parent(X, Y)", "parent(Y, X)");
    let mut edited = e.text.clone();
    let (_, a, b) = e.spans.iter().find(|(n, _, _)| n == "sample.ancestor_base").unwrap().clone();
    edited.replace_range(a as usize..b as usize, &edited_block);
    let old_rule = i.store().resolve("sample.ancestor_base").unwrap().unwrap().target.digest();
    let p = i.propose_text("sample", &edited, i.export("sample").unwrap().token).unwrap();
    let f = &p.proposal.files[0];
    assert_eq!(f.names_retargeted.iter().map(|(n, _, _)| n.as_str()).collect::<Vec<_>>(), vec!["sample.ancestor_base"], "{f:?}");
    assert!(f.members_added.iter().any(|(n, m)| n == "sample.ancestor" && *m == f.names_retargeted[0].2), "{f:?}");
    assert!(f.members_removed.iter().any(|(n, m)| n == "sample.ancestor" && *m == old_rule), "{f:?}");
    let seq = i.store().last_seq().unwrap();
    assert_eq!(i.store().last_seq().unwrap(), seq);
    assert_eq!(before.len(), 3);
}

/// A new relation: the nominal's id (and the ids of records referring to
/// it) in the proposal are stand-ins; only the signature's id is real.
#[test]
fn new_relation_reports_provisional_ids() {
    let (_d, i) = sample();
    let e = i.export("sample").unwrap();
    let snippet = "relation likes :: Nat --> Nat --> Prop\n\nrule likes_self(X) = likes(X, X)\n";
    let text = format!("{}\n{snippet}", e.text);
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", &text, e.token).unwrap();
    assert!(p.proposal.error.is_none() && p.proposal.conflicts.is_empty(), "{p:?}");
    let f = &p.proposal.files[0];
    let proposed_nominal = f.names_added.iter().find(|(n, _)| n == "sample.likes").map(|(_, id)| *id);
    let proposed_sig = f.members_added.iter().find(|(n, _)| n == "sample.likes").map(|(_, id)| *id);
    let proposed_rule = f.names_added.iter().find(|(n, _)| n == "sample.likes_self").map(|(_, id)| *id);
    assert!(proposed_nominal.is_some() && proposed_sig.is_some() && proposed_rule.is_some(), "{f:?}");
    assert_eq!(i.store().last_seq().unwrap(), seq);

    let (_, w) = i.create_text(&Key::ns("sample"), "likes", snippet).unwrap();
    let real_nominal = w.targets.iter().find(|(p, _)| p == "sample.likes").unwrap().1;
    let real_rule = w.targets.iter().find(|(p, _)| p == "sample.likes_self").unwrap().1;
    let methods = i.store().methods(&real_nominal).unwrap();
    assert_ne!(proposed_nominal, Some(real_nominal), "new_nominal draws a fresh id; the proposal's id is a stand-in");
    assert_ne!(proposed_rule, Some(real_rule), "records referring to the new relation embed its id");
    assert!(
        f.members_added.iter().any(|(n, id)| n == "sample.likes" && methods.contains(id) && *id != real_rule),
        "the signature's id matches: {f:?}"
    );
}

/// A name a file lays out: refused with `LaidOut`, like `write_text`.
#[test]
fn laid_out_name_is_refused_like_an_edit() {
    use sophia_hash::merkle::{component_digest, member_digest};
    let (_d, i) = sample();
    let d = i.store().resolve("sample.double").unwrap().unwrap().target.digest();
    let rec = i.store().record(&d).unwrap().unwrap();
    let text = "function double(x::Int64)::Int64\n    x + x\nend\n";
    let component = component_digest("sophialang", &rec.normaliser, &rec.bytes);
    let _ = member_digest(&component, 0);
    i.store().remove("sample.double", None).unwrap();
    let f = sophia_store::FileIngest {
        path: "src/double.sl".into(),
        text: Some(text.into()),
        normaliser: rec.normaliser.clone(),
        records: vec![rec],
        spans: vec![(0, 0, text.len() as u32 - 1)],
        names: vec![("sample.double".into(), 0)],
        ..Default::default()
    };
    i.store().replace_files_checked(&[(f, 0)]).unwrap();
    let token = i.store().file_token("src/double.sl").unwrap();
    assert_eq!(i.store().laid_out("sample.double").unwrap(), Some(("src/double.sl".to_string(), token)));

    let e = i.export("sample").unwrap();
    assert!(!e.spans.iter().any(|(n, _, _)| n == "sample.double"), "a laid-out name is not exported");
    let proposed = format!("{}\n{text}", e.text);
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", &proposed, e.token).unwrap();
    match &p.proposal.error {
        Some(StoreError::LaidOut { path, token: t }) => assert_eq!((path.as_str(), *t), ("src/double.sl", token)),
        other => panic!("expected LaidOut, got {other:?} in {p:?}"),
    }
    assert!(!p.changes_nothing());
    assert_eq!(i.store().last_seq().unwrap(), seq);
    let (t, v) = i.fetch_text(&Key::name("sample.double")).unwrap();
    match i.write_text(&Key::name("sample.double"), v, &t) {
        Err(EditError::Store(StoreError::LaidOut { path, .. })) => assert_eq!(path, "src/double.sl"),
        other => panic!("write_text: expected LaidOut, got {other:?}"),
    }
}

/// A derived (file-truth) store refuses `propose_text` outright.
#[test]
fn derived_store_refuses_proposals() {
    let (_d, i) = sample();
    let e = i.export("sample").unwrap();
    i.store().set_derived().unwrap();
    let seq = i.store().last_seq().unwrap();
    assert!(matches!(i.propose_text("sample", &e.text, e.token), Err(StoreError::Derived)));
    assert_eq!(i.store().last_seq().unwrap(), seq);
}

/// A stale token and a refusal together: both are reported.
#[test]
fn conflict_and_refusal_are_both_reported() {
    let (_d, i) = sample();
    i.create_text(&Key::ns("sample"), "users", "query users\n    select p where name(p, a)\nend").unwrap();
    let e = i.export("sample").unwrap();
    let (_, v) = i.fetch_text(&Key::name("sample.double")).unwrap();
    let (n, a, b) = e.spans.iter().find(|(n, _, _)| n == "sample.double").unwrap().clone();
    i.write_text(&Key::name(&n), v, &e.text[a as usize..b as usize].replace("x + x", "2 * x")).unwrap();
    let now = i.export("sample").unwrap();
    let text = format!("{}\nfunction users(x::Nat)::Nat\n    x\nend\n", e.text);
    let seq = i.store().last_seq().unwrap();
    let p = i.propose_text("sample", &text, e.token).unwrap();
    assert_eq!(p.proposal.conflicts, vec![("sample".to_string(), e.token, now.token)], "{p:?}");
    assert!(matches!(&p.proposal.error, Some(StoreError::NameClaimed { name, .. }) if name == "sample.users"), "{p:?}");
    assert_eq!(p.proposal.files.len(), 1, "the partial file is reported: {p:?}");
    assert!(p.proposal.report.is_none());
    assert_eq!(i.store().last_seq().unwrap(), seq);
}
