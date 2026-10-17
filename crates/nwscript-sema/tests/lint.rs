//! Each lint on a small script, the comment directive, and the modes'
//! levels.

use nwscript_diag::{Diagnostic, Level, Levels, Severity, SourceMap, codes};
use nwscript_sema::lint::{apply_directives, lint};
use nwscript_sema::{EngineTable, ParsedIncludes, analyze};
use nwscript_syntax::parse;

const NWSCRIPT: &str = "int TRUE = 1;\r\nint FALSE = 0;\r\nvoid PrintString(string s);\r\nvoid PrintInteger(int n);\r\nvoid PrintPair(int a, int b);\r\nint GetHitDice(object o);\r\n";

fn lints(src: &str) -> (Vec<Diagnostic>, SourceMap) {
    let mut map = SourceMap::new();
    let tf = map.add("nwscript.nss", NWSCRIPT.as_bytes().to_vec());
    let (table, _) = EngineTable::from_nwscript(tf, NWSCRIPT.as_bytes());
    let f = map.add("t.nss", src.as_bytes().to_vec());
    let a = analyze(
        "t.nss",
        parse(f, src.as_bytes()),
        &table,
        &mut ParsedIncludes::new(),
    );
    assert!(
        a.diagnostics.iter().all(|d| !d.is_error()),
        "{:?}",
        a.diagnostics
    );
    (lint(&a.program), map)
}

fn ids(ds: &[Diagnostic]) -> Vec<&'static str> {
    ds.iter().map(|d| d.code.id).collect()
}

#[test]
fn each_lint() {
    assert_eq!(
        ids(&lints("void main() { int n = GetHitDice(OBJECT_SELF); }").0),
        ["W0001"]
    );
    assert_eq!(ids(&lints("void main() { int n; n = 1; }").0), ["W0001"]);
    assert!(
        lints("void main() { int n = 1; PrintInteger(n); }")
            .0
            .is_empty()
    );
    assert_eq!(
        ids(&lints("void Unused() { }\nvoid main() { }").0),
        ["W0003"]
    );
    // A library has no entry point; its functions are for whoever includes it.
    assert!(lints("void Helper() { }").0.is_empty());
    assert_eq!(
        ids(&lints("void F(int n) { int n = 2; PrintInteger(n); }\nvoid main() { F(1); }").0),
        ["W0004"]
    );
    assert_eq!(
        ids(&lints("int g = 1;\nvoid main() { int g = 2; PrintInteger(g); }").0),
        ["W0004"]
    );
    assert_eq!(
        ids(&lints("void main() { return; PrintString(\"x\"); }").0),
        ["W0005"]
    );
    assert!(lints("void main() { int n = 1; switch (n) { case 1: break; case 2: PrintString(\"x\"); break; } }").0.is_empty());
    assert_eq!(
        ids(&lints("void main() { int n = 1; PrintPair(n++, n); }").0),
        ["W0006"]
    );
    assert_eq!(
        ids(&lints("void main() { int n = 1; n == 2; PrintInteger(n); }").0),
        ["W0007"]
    );
}

#[test]
fn mixed_arithmetic() {
    let (ds, _) = lints("void main() { int n = 3; if (n * 2.0 > 1.0) PrintString(\"x\"); }");
    assert_eq!(ids(&ds), ["W0008"]);
}

#[test]
fn directives_allow_on_the_line() {
    let src = "void main() {\n  int a = 1; // nwscript: allow(W0001)\n  // nwscript: allow(w0001)\n  int b = 2;\n  int c = 3;\n}\n";
    let (ds, map) = lints(src);
    assert_eq!(ids(&ds), ["W0001", "W0001", "W0001"]);
    let left = apply_directives(ds, &map);
    assert_eq!(left.len(), 1);
    assert!(left[0].message.contains("`c`"));
}

#[test]
fn modes_set_severity() {
    let (ds, _) = lints("void main() { int n = GetHitDice(OBJECT_SELF); }");
    let d = ds[0].clone();
    let mut strict = Levels::default();
    strict.strict = true;
    assert_eq!(strict.apply(d.clone()).unwrap().severity, Severity::Error);
    let compat = Levels::default();
    assert_eq!(compat.apply(d.clone()).unwrap().severity, Severity::Warning);
    let mut allowed = Levels::default();
    allowed.strict = true;
    allowed.set("W0001", Level::Allow);
    assert!(allowed.apply(d.clone()).is_none());
    let mut warn = Levels::default();
    warn.strict = true;
    warn.set("W0001", Level::Warn);
    assert_eq!(warn.apply(d).unwrap().severity, Severity::Warning);
    // --compat bioware accepts what BioWare's compiler did, as warnings.
    let e = Diagnostic::new(&codes::E0309, "argument of the wrong type");
    let mut bioware = Levels::default();
    bioware.accepted = codes::BIOWARE_ACCEPTED.iter().map(|c| c.id).collect();
    assert_eq!(
        bioware.apply(e.clone()).unwrap().severity,
        Severity::Warning
    );
    assert_eq!(compat.apply(e).unwrap().severity, Severity::Error);
}
