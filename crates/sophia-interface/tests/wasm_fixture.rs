//! Writes `tests/fixtures/web.sophia`, the store the wasm read-path test
//! (`tests/wasm_read.rs`) opens from its bytes: Sophia's sample program plus
//! one folder-store file. Run natively when the store format changes:
//!
//!     cargo test -p sophia-interface --test wasm_fixture -- --ignored

use sophia_interface::Interface;

#[test]
#[ignore]
fn write_wasm_fixture() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/web.sophia");
    let _ = std::fs::remove_file(path);
    let i = Interface::open(sophia_store::sqlite::Sqlite::open(path).unwrap(), "web").unwrap();
    i.load_sample().unwrap();
    let text = "struct P\n    x::Int\nend\n";
    let component = sophia_hash::merkle::component_digest("julia", "moonkale-wp66@7", b"struct-x");
    let r = sophia_hash::Record {
        id: sophia_hash::merkle::member_digest(&component, 0),
        lang: "julia".into(),
        normaliser: "moonkale-wp66@7".into(),
        kind: "struct_definition".into(),
        component,
        index: 0,
        component_size: 1,
        bytes: b"struct-x".to_vec(),
        display: b"P".to_vec(),
        status: sophia_hash::Status::Complete,
        refs: vec![],
        nominal_refs: vec![],
    };
    i.store()
        .replace_files(&[sophia_store::FileIngest {
            path: "src/P.jl".into(),
            text: Some(text.into()),
            normaliser: "moonkale-wp66@7".into(),
            records: vec![r],
            spans: vec![(0, 0, text.len() as u32 - 1)],
            names: vec![("jl.Pkg.P".into(), 0)],
            ..Default::default()
        }])
        .unwrap();
    i.store().set_derived().unwrap();
    drop(i);
    let c = rusqlite_vacuum(path);
    eprintln!("{path}: {c} bytes");
}

fn rusqlite_vacuum(path: &str) -> u64 {
    let s = sophia_store::sqlite::Sqlite::open(path).unwrap();
    drop(s);
    std::process::Command::new("sqlite3").args([path, "VACUUM;"]).status().ok();
    std::fs::metadata(path).unwrap().len()
}

/// The same fixture natively: `Sqlite::from_bytes` reads a WAL-mode store
/// file from memory, and the store is read-only.
#[test]
fn from_bytes_reads_and_refuses_writes() {
    let bytes = include_bytes!("fixtures/web.sophia");
    let i = Interface::open_read_only(sophia_store::sqlite::Sqlite::from_bytes(bytes).unwrap(), "web");
    assert!(i.store().resolve("sample.add").unwrap().is_some());
    assert!(i.store().is_derived().unwrap());
    assert!(i.store().set_meta("x", b"y").is_err(), "a from_bytes store is read-only");
}
