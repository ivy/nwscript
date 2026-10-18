use assert_cmd::Command;
use predicates::prelude::*;

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

/// The keys of the envelope `nws --format json` writes, in the order its
/// `cli.rs` test checks them plus `tool`, `command` and `result`.
const ENVELOPE_KEYS: [&str; 8] = [
    "tool",
    "version",
    "command",
    "game",
    "inputs",
    "outputs",
    "diagnostics",
    "result",
];

#[test]
fn explain_prints_the_explanation() {
    rakata()
        .args(["explain", "RE0001"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "RE0001: an input file could not be read",
        ));
}

#[test]
fn explain_unknown_code_is_a_usage_error() {
    for code in ["RE9999", "E0001"] {
        rakata()
            .args(["explain", code])
            .assert()
            .code(2)
            .stderr(predicate::str::contains("no diagnostic code"));
    }
}

#[test]
fn json_mode_writes_exactly_one_document_with_the_nws_envelope() {
    let out = rakata()
        .args(["--format", "json", "explain", "re0001"])
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
    assert_eq!(doc["tool"], "rakata");
    assert_eq!(doc["version"], rakata::VERSION);
    assert_eq!(doc["command"], "explain");
    assert_eq!(doc["game"], serde_json::Value::Null);
    assert_eq!(doc["result"]["code"], "RE0001");
    let mut keys: Vec<&str> = doc
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected = ENVELOPE_KEYS.to_vec();
    expected.sort_unstable();
    assert_eq!(keys, expected, "the envelope's keys differ from nws's");
}

#[test]
fn the_game_is_recorded_as_named() {
    let out = rakata()
        .args([
            "--game",
            "games/swkotor",
            "--format",
            "json",
            "explain",
            "RE0001",
        ])
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["game"], "games/swkotor");
}

#[test]
fn unknown_flag_is_a_usage_error() {
    rakata()
        .args(["--frobnicate", "explain", "RE0001"])
        .assert()
        .code(2);
}

#[test]
fn version_is_the_build_identity() {
    let out = rakata().arg("--version").output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(text.trim(), format!("rakata {}", rakata::VERSION));
    assert!(
        rakata::VERSION.starts_with(env!("CARGO_PKG_VERSION")),
        "{}",
        rakata::VERSION
    );
}

#[test]
fn exit_codes_are_the_documented_table() {
    use rakata::Exit;
    assert_eq!(Exit::Ok as u8, 0);
    assert_eq!(Exit::Errors as u8, 1);
    assert_eq!(Exit::Usage as u8, 2);
    assert_eq!(Exit::Different as u8, 3);
    assert_eq!(Exit::Internal as u8, 101);
}

#[test]
fn every_help_has_an_example_and_lists_only_built_commands() {
    let top = rakata().arg("--help").output().unwrap();
    let text = String::from_utf8_lossy(&top.stdout);
    assert!(text.contains("Examples:"));
    let commands: Vec<&str> = text
        .lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .filter(|c| *c != "help")
        .collect();
    assert_eq!(
        commands,
        ["install", "explain"],
        "--help lists commands not yet built"
    );
    for cmd in ["explain", "install"] {
        let out = rakata().args([cmd, "--help"]).output().unwrap();
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("Example"),
            "{cmd}"
        );
    }
}
