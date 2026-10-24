//! SPEC-009 §4, Moonkale's gate 3: the read path in wasm. Opens the fixture
//! store from its bytes, read-only, and browses, prints and queries it.
//! Runs only on `wasm32-unknown-unknown`, under Node:
//!
//!     CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
//!       cargo test -p sophia-interface --no-default-features --features sqlite \
//!       --target wasm32-unknown-unknown --test wasm_read
#![cfg(target_arch = "wasm32")]

use sophia_interface::{Direction, Interface, Key};
use wasm_bindgen_test::wasm_bindgen_test;

const STORE: &[u8] = include_bytes!("fixtures/web.sophia");

fn open() -> Interface<sophia_store::sqlite::Sqlite> {
    Interface::open_read_only(sophia_store::sqlite::Sqlite::from_bytes(STORE).unwrap(), "web")
}

#[wasm_bindgen_test]
fn browse() {
    let i = open();
    let root = i.children(&i.root()).unwrap();
    assert!(root.nodes.iter().any(|n| n.label == "sample"), "{:?}", root.nodes);
    let sample = i.children(&Key::ns("sample")).unwrap();
    assert!(sample.nodes.iter().any(|n| n.label == "add"));
    let g = i.neighbours(&Key::name("sample.add"), 2, Direction::Both, 100).unwrap();
    assert!(g.edges.iter().any(|e| e.kind == "References"));
    assert!(i.all(10_000).unwrap().nodes.len() > 20);
}

#[wasm_bindgen_test]
fn print() {
    let i = open();
    let (t, _) = i.fetch_text(&Key::name("sample.add")).unwrap();
    assert!(t.contains("function add"), "{t}");
    let (p, _) = i.fetch_text(&Key::name("jl.Pkg.P")).unwrap();
    assert!(p.contains("struct P"), "{p}");
    let ts = i.text_source(&Key::name("jl.Pkg.P")).unwrap().unwrap();
    assert_eq!(ts.path, "src/P.jl");
}

#[wasm_bindgen_test]
fn query() {
    let i = open();
    let text = r#"query users
        uses_(a, b) = refs(a, b)
        uses_(a, c) = refs(a, b) && uses_(b, c)
        select a, p where name("sample.add", t) && uses_(a, t) && name(p, a)
    end"#;
    let (table, _) = i.query(text, Default::default()).unwrap();
    assert!(table.rows.iter().any(|r| r[1] == sophia_store::query::Value::Text("sample.add_test".into())), "{table:?}");
}

#[wasm_bindgen_test]
fn writes_are_refused() {
    let i = open();
    assert!(i.store().is_derived().unwrap());
    let (t, v) = i.fetch_text(&Key::name("sample.add")).unwrap();
    assert!(i.write_text(&Key::name("sample.add"), v, &t).is_err());
}
