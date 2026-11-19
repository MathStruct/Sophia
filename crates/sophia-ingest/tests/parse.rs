//! Parser agreement regression tests; Julia is needed only to regenerate truth.
use sophia_ingest::conformance::{ParseLedger, ParseOutcome, parse_ledger};
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/julia-semantics/parse")
}
const TRUTH: &str = include_str!("julia-semantics/parse/truth.tsv");

fn row<'a>(ledger: &'a ParseLedger, name: &str) -> &'a sophia_ingest::conformance::ParseRow {
    ledger.rows.iter().find(|r| r.name == name).unwrap_or_else(|| panic!("no row for {name}"))
}

#[test]
fn fixture_matches_julia_syntax() {
    let ledger = parse_ledger(&fixture(), TRUTH).unwrap();
    assert_eq!(ledger.malformed, 0);
    assert!(ledger.julia_errors.is_empty());
    assert!(ledger.undiscovered.is_empty());
    for name in ["kw_semicolon", "kw_comma", "kw_shorthand"] {
        let r = row(&ledger, name);
        assert_eq!(r.outcome, ParseOutcome::Wrong, "{r:?}");
        assert!(r.detail.contains("callkw"), "{r:?}");
    }
    let typed = row(&ledger, "x");
    assert_eq!(typed.outcome, ParseOutcome::Wrong);
    assert!(typed.detail.contains("form: Julia=typed_global Sophia=short"));
    assert_eq!(row(&ledger, "refused").outcome, ParseOutcome::LeavesOpen);
    use ParseOutcome::{Approximates as A, Knows as K, LeavesOpen as O, Wrong as W};
    let expected = [
        (1, "Pars", O),
        (3, "long_where", K),
        (6, "short_where", K),
        (7, "kw_semicolon", W),
        (8, "kw_comma", W),
        (9, "kw_shorthand", W),
        (10, "x", W),
        (11, "a", O),
        (11, "b", W),
        (12, "+", K),
        (13, "⊕", W),
        (14, "+", K),
        (15, "Callable", A),
        (16, "-", K),
        (17, "-", K),
        (20, "juxtapose", K),
        (21, "adjoint_case", K),
        (22, "end_index", K),
        (23, "dotted", K),
        (26, "var\"odd name\"", O),
        (27, "interpolated", K),
        (29, "strings", K),
        (30, "do_case", K),
        (33, "lambda_case", K),
        (34, "Options", A),
        (35, "n", W),
        (37, "Mutable", A),
        (40, "Bits", K),
        (41, "Abstract", K),
        (42, "Bare", K),
        (43, "bare", K),
        (45, "Inner", K),
        (46, "nested", K),
        (48, "inlined", K),
        (52, "documented", K),
        (55, "m", O),
        (58, "refused", O),
    ];
    let actual: Vec<_> = ledger.rows.iter().filter(|r| r.file == "Pars.jl").map(|r| (r.line, r.name.as_str(), r.outcome)).collect();
    assert_eq!(actual, expected);
    assert_eq!(ledger.tally().into_iter().collect::<Vec<_>>(), [(K, 28), (A, 3), (O, 6), (W, 10)]);
    let more: Vec<_> = ledger.rows.iter().filter(|r| r.file == "More.jl").map(|r| (r.line, r.name.as_str(), r.outcome)).collect();
    assert_eq!(
        more,
        [
            (1, "outer", O),
            (2, "inner", K),
            (5, "documented_macro", K),
            (6, "typed_const", K),
            (6, "typed_const", W),
            (8, "conditional", K),
            (10, "check", W),
            (11, "==", K),
            (12, "positional_default", K),
            (12, "k", W)
        ]
    );
    // UTF-8 positions are bytes, and Moonkale's exclusive end equals the
    // oracle's inclusive last byte plus one for the long method.
    let text = std::fs::read_to_string(fixture().join("src/Pars.jl")).unwrap();
    let code = sophia_ingest::code::file("julia", &text).unwrap();
    let def = code.defs.iter().find(|d| d.name == "long_where").unwrap();
    let truth: Vec<_> = TRUTH.lines().find(|l| l.split('\t').nth(6) == Some("long_where")).unwrap().split('\t').collect();
    assert_eq!(def.start, truth[3].parse::<usize>().unwrap());
    assert_eq!(def.end, truth[4].parse::<usize>().unwrap() + 1);
    assert_eq!(def.name_span.unwrap().0, truth[2].parse::<usize>().unwrap());
    assert_eq!(row(&ledger, "juxtapose").name_byte, text.find("juxtapose").unwrap());
}

fn change(name: &str, column: usize, value: &str) -> String {
    TRUTH
        .lines()
        .map(|line| {
            let mut fields: Vec<_> = line.split('\t').map(str::to_string).collect();
            if fields[0] == "def" && fields[6] == name {
                fields[column] = value.into();
            }
            fields.join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn falsified_truth_exercises_comparison() {
    for (column, value, field) in
        [(5, "function", "form:"), (6, "renamed", "name:"), (7, "8", "npos:"), (8, "invented", "kwnames:"), (9, "7", "callkw:")]
    {
        let ledger = parse_ledger(&fixture(), &change("juxtapose", column, value)).unwrap();
        let r = row(&ledger, if column == 6 { "renamed" } else { "juxtapose" });
        assert_eq!(r.outcome, ParseOutcome::Wrong);
        assert!(r.detail.contains(field), "{r:?}");
    }
    let ledger = parse_ledger(&fixture(), TRUTH).unwrap();
    assert_eq!(row(&ledger, "juxtapose").outcome, ParseOutcome::Knows);
    assert_eq!(row(&ledger, "Callable").outcome, ParseOutcome::Approximates);
    assert_eq!(row(&ledger, "refused").outcome, ParseOutcome::LeavesOpen);
    let removed = TRUTH.lines().filter(|l| l.split('\t').nth(6) != Some("juxtapose")).collect::<Vec<_>>().join("\n");
    let ledger = parse_ledger(&fixture(), &removed).unwrap();
    assert!(row(&ledger, "juxtapose").detail.starts_with("extra:"));
    let mut added = TRUTH.to_string();
    added.push_str("def\tPars.jl\t0\t0\t0\tconst\tmissing\t-\t-\t0\n");
    let ledger = parse_ledger(&fixture(), &added).unwrap();
    assert!(row(&ledger, "missing").detail.starts_with("missed:"));
}

#[test]
fn malformed_truth_is_counted_and_julia_errors_are_separate() {
    let bad = [
        "nonsense",
        "def\tPars.jl",
        "error\tPars.jl\tx\t0",
        "def\tPars.jl\t0\t0\t0\tunknown\tf\t-\t-\t0",
        "def\tPars.jl\t0\t0\t0\tfunction\tf\t-\t-\t0",
        "def\tPars.jl\t0\t0\t0\tconst\tf\t1\t-\t0",
        "def\tPars.jl\t0\t1\t0\tconst\tf\t-\t-\t0",
        "def\tPars.jl\t0\t0\t999999\tconst\tf\t-\t-\t0",
        "def\tPars.jl\t0\t0\t0\tshort\tf\t1\tz,a\t0",
        "def\tPars.jl\t0\t0\t0\tshort\tf\t1\t-\tnan",
    ];
    let text = format!("{}\n{}\nerror\tPars.jl\t0\t1\n", TRUTH, bad.join("\n"));
    let ledger = parse_ledger(&fixture(), &text).unwrap();
    // TRUTH already ends with newline: the extra blank is also malformed.
    assert_eq!(ledger.malformed, bad.len() + 1);
    assert_eq!(ledger.julia_errors, [("Pars.jl".into(), 0, 1)]);
    let duplicate = format!("{}{}\n", TRUTH, TRUTH.lines().next().unwrap());
    assert_eq!(parse_ledger(&fixture(), &duplicate).unwrap().malformed, 1);
}

#[test]
fn syntax_recovery_refusal_and_undiscovered_files_are_explicit() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join("src")).unwrap();
    std::fs::write(tmp.path().join("Project.toml"), "name = \"Bad\"\n").unwrap();
    let source = "f(x) = (x + )\n";
    std::fs::write(tmp.path().join("src/Bad.jl"), source).unwrap();
    let truth = format!("def\tBad.jl\t0\t0\t{}\tshort\tf\t1\t-\t0\n", source.len() - 2);
    let ledger = parse_ledger(tmp.path(), &truth).unwrap();
    let r = row(&ledger, "f");
    assert_eq!(r.outcome, ParseOutcome::LeavesOpen, "{r:?}");
    assert!(r.detail.contains("syntax error"), "{r:?}");
    assert!(ledger.julia_errors.is_empty());
    let with_error = format!("{truth}error\tBad.jl\t12\t12\n");
    let ledger = parse_ledger(tmp.path(), &with_error).unwrap();
    assert_eq!(ledger.julia_errors.len(), 1);
    assert_eq!(row(&ledger, "f").outcome, ParseOutcome::LeavesOpen);
    let outside = format!("{truth}def\torphan.jl\t0\t0\t1\tconst\ta\t-\t-\t0\n");
    let ledger = parse_ledger(tmp.path(), &outside).unwrap();
    assert_eq!(ledger.undiscovered, ["orphan.jl"]);
    assert!(ledger.rows.iter().all(|r| r.file != "orphan.jl"));
}
