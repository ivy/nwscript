//! The shim's command line, flag by flag, on a tiny script.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nwnnsscomp-shim-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("inc")).unwrap();
    std::fs::write(
        d.join("inc/nwscript.nss"),
        "void PrintString(string s);\r\n",
    )
    .unwrap();
    std::fs::write(
        d.join("inc/k_inc_x.nss"),
        "string Hello() { return \"hi\"; }\r\n",
    )
    .unwrap();
    std::fs::write(
        d.join("a.nss"),
        "#include \"k_inc_x\"\r\nvoid main() { PrintString(Hello()); }\r\n",
    )
    .unwrap();
    std::fs::write(
        d.join("bad.nss"),
        "void main()\r\n{\r\n    Nope();\r\n}\r\n",
    )
    .unwrap();
    d
}

fn shim(d: &PathBuf) -> Command {
    let mut c = Command::cargo_bin("nwnnsscomp").unwrap();
    c.current_dir(d);
    c
}

#[test]
fn compile_with_flags_writes_beside_the_source() {
    let d = dir("c");
    shim(&d)
        .args(["-c", "-g", "-q", "-v1.00", "-i", "inc", "a.nss"])
        .assert()
        .code(0);
    assert!(
        std::fs::read(d.join("a.ncs"))
            .unwrap()
            .starts_with(b"NCS V1.0")
    );
}

#[test]
fn holopatcher_form_with_output() {
    let d = dir("o");
    std::fs::create_dir_all(d.join("out")).unwrap();
    shim(&d)
        .args(["-c", "a.nss", "-o", "out/x.ncs", "-i", "inc"])
        .assert()
        .code(0);
    assert!(d.join("out/x.ncs").is_file());
}

#[test]
fn errors_in_the_installer_shape() {
    let d = dir("e");
    shim(&d)
        .args(["-c", "-i", "inc", "bad.nss"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "bad.nss(3): Error: no function named `Nope`",
        ));
    assert!(!d.join("bad.ncs").exists());
}

#[test]
fn v130_is_an_error() {
    let d = dir("v");
    shim(&d)
        .args(["-c", "-v1.30", "-i", "inc", "a.nss"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("only -v1.00"));
}

#[test]
fn ignored_flags_warn() {
    let d = dir("w");
    shim(&d)
        .args([
            "-c", "-e", "-x", "-t1", "-p", "/nowhere", "-i", "inc", "a.nss",
        ])
        .assert()
        .code(0)
        .stderr(
            predicate::str::contains("-e is ignored")
                .and(predicate::str::contains("-t1 is ignored"))
                .and(predicate::str::contains("-p is ignored")),
        );
}

#[test]
fn combined_flags_and_semicolon_includes() {
    let d = dir("k");
    std::fs::create_dir_all(d.join("empty")).unwrap();
    shim(&d)
        .args(["-cgq", "-iempty;inc", "a.nss"])
        .assert()
        .code(0);
    assert!(d.join("a.ncs").is_file());
}

#[test]
fn disassemble() {
    let d = dir("d");
    shim(&d).args(["-c", "-i", "inc", "a.nss"]).assert().code(0);
    shim(&d).args(["-d", "a.ncs"]).assert().code(0);
    assert!(
        std::fs::read_to_string(d.join("a.pcode"))
            .unwrap()
            .contains("ACTION")
    );
}

#[test]
fn compat_accepts_what_nwnnsscomp_did() {
    let d = dir("a");
    std::fs::write(
        d.join("dup.nss"),
        "void main() { int n = 1; switch (n) { case 1: case 1: break; } }\r\n",
    )
    .unwrap();
    shim(&d)
        .args(["-c", "-i", "inc", "dup.nss"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Warning: two `case` labels"));
}

#[test]
fn unknown_flags_and_missing_input_are_usage_errors() {
    let d = dir("u");
    shim(&d).args(["-z", "a.nss"]).assert().code(2);
    shim(&d).args(["-c"]).assert().code(2);
}
