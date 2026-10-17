use assert_cmd::Command;
use predicates::prelude::*;

fn nws() -> Command {
    Command::cargo_bin("nws").unwrap()
}

#[test]
fn explain_prints_the_explanation() {
    nws()
        .args(["explain", "E0001"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "E0001: an input file could not be read",
        ));
}

#[test]
fn explain_unknown_code_is_a_usage_error() {
    nws()
        .args(["explain", "E9999"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no diagnostic code"));
}

#[test]
fn json_mode_writes_exactly_one_document() {
    let out = nws()
        .args(["--format", "json", "explain", "e0001"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let mut docs =
        serde_json::Deserializer::from_slice(&out.stdout).into_iter::<serde_json::Value>();
    let doc = docs.next().unwrap().unwrap();
    assert!(
        docs.next().is_none(),
        "more than one JSON document on stdout"
    );
    assert_eq!(doc["tool"], "nws");
    assert_eq!(doc["command"], "explain");
    assert_eq!(doc["result"]["code"], "E0001");
    for key in ["version", "inputs", "outputs", "diagnostics", "game"] {
        assert!(doc.get(key).is_some(), "envelope lacks {key}");
    }
}

#[test]
fn unknown_flag_is_a_usage_error() {
    nws()
        .args(["--frobnicate", "explain", "E0001"])
        .assert()
        .code(2);
}

#[test]
fn unknown_lint_code_is_a_usage_error() {
    nws()
        .args(["--allow", "W9999", "explain", "E0001"])
        .assert()
        .code(2);
}

#[test]
fn reserved_commands_say_not_yet_implemented() {
    for c in ["compile", "check", "diff", "fmt", "lsp"] {
        nws()
            .arg(c)
            .assert()
            .code(2)
            .stderr(predicate::str::contains("not yet implemented"));
    }
}

#[test]
fn internal_errors_exit_101_and_name_the_input() {
    nws()
        .arg("internal-panic")
        .assert()
        .code(101)
        .stderr(predicate::str::contains(
            "while processing `the internal-panic command`",
        ));
}

#[test]
fn every_help_has_an_example() {
    let top = nws().arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&top.stdout).contains("Examples:"));
    for c in [
        "explain",
        "compile",
        "check",
        "disasm",
        "asm",
        "decompile",
        "run",
        "test",
        "diff",
        "fn",
        "conformance",
        "lua",
        "fmt",
        "lsp",
    ] {
        let out = nws().args([c, "--help"]).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains("Example"), "`nws {c} --help` has no example");
    }
}
