//! Syntax, canonical form and identity (SPEC-007 §4, §5).

use sophia_query::syntax::{Profile, parse, print};
use sophia_query::{Error, prepare_text};

const REACH: &str = r#"
query reach(target::Id)
    # direct and transitive dependants
    dep(a, b) = refs(a, b)
    dep(a, c) = refs(a, b) && dep(b, c)
    select a where dep(a, target)
end
"#;

#[test]
fn julia_text_round_trips() {
    let p = prepare_text(REACH).unwrap();
    let text = p.print(Profile::Julia);
    let q = prepare_text(&text).unwrap();
    assert_eq!(q.query, p.query);
    assert_eq!(q.id, p.id);
    assert_eq!(q.print(Profile::Julia), text, "printing is a fixed point");
}

#[test]
fn every_construct_round_trips() {
    let text = r#"
query everything(p::Text, n::Int, d::Id)
    a(x, y) = def(x, k) && k == :Method && refs(x, y) && !uses(y, _)
    b(x) = name(s, x) && startswith(s, p) && s != "a \"quoted\" name"
    b(x) = annotation(_, "doc", x) && fact(_, :Typing, x) && member(x, _) && nominal(x, :Generic)
    c(x, z) = a(x, y) && b(y) && z == n
    select x, count(z) where c(x, z) && x != d && x != 0x0000000000000000000000000000000000000000000000000000000000000001
end
"#;
    let p = prepare_text(text).unwrap();
    let again = prepare_text(&p.print(Profile::Julia)).unwrap();
    assert_eq!(again.query, p.query);
    assert_eq!(again.id, p.id);
    assert!(p.print(Profile::Datalog).contains(":-"));
}

#[test]
fn names_and_formatting_do_not_change_the_id() {
    let renamed = r#"query whatever(x::Id)
        reaches(u, v) = refs(u, v)
        reaches(u, w) = refs(u, v) && reaches(v, w)
        select u where reaches(u, x) end"#;
    assert_eq!(prepare_text(REACH).unwrap().id, prepare_text(renamed).unwrap().id);
}

#[test]
fn rule_order_and_duplicate_rules_do_not_change_the_id() {
    let swapped = r#"
query reach(target::Id)
    dep(a, c) = refs(a, b) && dep(b, c)
    dep(a, b) = refs(a, b)
    dep(a, b) = refs(a, b)
    select a where dep(a, target)
end"#;
    assert_eq!(prepare_text(REACH).unwrap().id, prepare_text(swapped).unwrap().id);
}

#[test]
fn predicate_order_and_unused_predicates_do_not_change_the_id() {
    let a = r#"query q
        p(x) = def(x, :method)
        r(x) = p(x) && refs(x, _)
        select x where r(x) end"#;
    let b = r#"query q
        unused(x) = def(x, :data)
        r(x) = p(x) && refs(x, _)
        p(x) = def(x, :method)
        select x where r(x) end"#;
    assert_eq!(prepare_text(a).unwrap().id, prepare_text(b).unwrap().id);
}

#[test]
fn literal_order_is_part_of_the_identity() {
    // SPEC-007 §4.1: like argument order in code; equivalences are facts.
    let a = prepare_text("select x where def(x, :method) && refs(x, _)").unwrap();
    let b = prepare_text("select x where refs(x, _) && def(x, :method)").unwrap();
    assert_ne!(a.id, b.id);
}

#[test]
fn meaning_changes_the_id() {
    let a = prepare_text("select x where def(x, :method)").unwrap();
    let b = prepare_text("select x where def(x, :data)").unwrap();
    let c = prepare_text("select x, k where def(x, k)").unwrap();
    assert_ne!(a.id, b.id);
    assert_ne!(a.id, c.id);
}

#[test]
fn queries_and_definitions_share_the_identity_rule() {
    let p = prepare_text(REACH).unwrap();
    let r = sophia_query::canon::record(&p.query, &p.names, vec![], vec![]);
    assert_eq!(r.id, p.id);
    assert!(sophia_hash::Record::verify(&[&r]));
    let back = sophia_query::Prepared::from_record(&r).unwrap();
    assert_eq!(back.query, p.query);
    assert_eq!(back.names, p.names);
}

fn err(text: &str) -> String {
    match prepare_text(text) {
        Ok(_) => panic!("accepted: {text}"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn checks_reject_bad_queries() {
    assert!(err("select x where def(x, 3)").contains("Text"), "sort mismatch");
    assert!(err("select x where refs(y, _)").contains("`x`"), "unbound output");
    assert!(err("select x where def(x, _) && !refs(x, y) && y != x").contains("`y`"), "unsafe negation");
    assert!(err("query q\n p(x) = def(x, _) && !p(x)\n select x where p(x) end").contains("negation"));
    assert!(err("select x where nope(x)").contains("nope"));
    assert!(err("select x where refs(x)").contains("arguments"));
    assert!(err("query q\n refs(x, y) = def(x, y)\n select x where def(x, _) end").contains("base relation"));
    assert!(err("select x where def(x, k").contains("expected"));
    assert!(err("query q(t::Thing) select x where def(x, _) end").contains("sort"));
    assert!(err("query q\n p(x) = def(x, _)\n select x where p(x)").contains("end"));
}

#[test]
fn parser_is_bounded() {
    let long = format!("select x where def(x, _){}", " && def(x, _)".repeat(100));
    assert!(matches!(prepare_text(&long), Err(Error::Parse { .. })));
    let huge = "#".repeat(sophia_query::syntax::MAX_TEXT + 1);
    assert!(matches!(parse(&huge), Err(Error::Parse { .. })));
}

#[test]
fn datalog_profile_prints() {
    let p = prepare_text(REACH).unwrap();
    let d = print(&p.query, &p.names, Profile::Datalog);
    assert!(d.contains("dep(A, B) :- refs(A, B)."), "{d}");
    assert!(d.contains("$target"), "{d}");
}
