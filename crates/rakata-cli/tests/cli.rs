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
fn a_diagnostic_points_at_rakata_explain_not_nws() {
    let missing = std::env::temp_dir().join(format!("rakata-cli-missing-{}", std::process::id()));
    let text = rakata()
        .args(["--game", missing.to_str().unwrap(), "which", "x.utc"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&text.stderr);
    assert!(stderr.contains("run `rakata explain RE"), "{stderr}");
    assert!(!stderr.contains("nws explain"), "{stderr}");
    let json = rakata()
        .args([
            "--format",
            "json",
            "--game",
            missing.to_str().unwrap(),
            "which",
            "x.utc",
        ])
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let explain = doc["diagnostics"][0]["explain"].as_str().unwrap();
    assert!(explain.starts_with("rakata explain RE"), "{explain}");
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
    let mut commands: Vec<&str> = text
        .lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .filter(|c| *c != "help")
        .collect();
    commands.sort_unstable();
    assert_eq!(
        commands,
        [
            "2da",
            "cat",
            "conformance",
            "corpus",
            "explain",
            "gff",
            "install",
            "lip",
            "tlk",
            "which"
        ],
        "--help lists commands not yet built"
    );
    for cmd in [
        "2da",
        "cat",
        "conformance",
        "corpus",
        "explain",
        "gff",
        "install",
        "lip",
        "tlk",
        "which",
    ] {
        let out = rakata().args([cmd, "--help"]).output().unwrap();
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("Example"),
            "{cmd} --help has no example"
        );
    }
}

/// A tree with a valid empty `chitin.key`, in a fresh temporary directory.
fn install(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rakata-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Override")).unwrap();
    std::fs::write(
        dir.join("chitin.key"),
        rakata_archive::Key::new().to_bytes().unwrap(),
    )
    .unwrap();
    dir
}

#[test]
fn corpus_extract_and_decompile_without_a_game_are_usage_errors_naming_the_flag() {
    for action in ["extract", "decompile"] {
        rakata()
            .args(["corpus", action])
            .assert()
            .code(2)
            .stderr(predicate::str::contains("--game"));
    }
}

#[test]
fn corpus_verify_without_a_manifest_reports_re1003() {
    let dir = install("corpus-verify");
    rakata()
        .args([
            "corpus",
            "verify",
            "--out",
            dir.to_str().unwrap(),
            "--manifest",
            dir.join("absent.toml").to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE1003"));
}

#[test]
fn corpus_decompile_over_an_install_without_nwscript_reports_re1002() {
    let dir = install("corpus-nonwscript");
    rakata()
        .args([
            "--game",
            dir.to_str().unwrap(),
            "corpus",
            "decompile",
            "--out",
            dir.join("corpus").to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE1002"));
}

#[test]
fn conformance_without_a_game_is_a_usage_error_naming_the_flag() {
    rakata()
        .arg("conformance")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game"));
}

#[test]
fn conformance_over_a_directory_that_is_not_an_install_reports_re0501() {
    let dir = std::env::temp_dir().join(format!("rakata-cli-noinstall-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    rakata()
        .args(["--game", dir.to_str().unwrap(), "conformance"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE0501"));
}

#[test]
fn conformance_with_an_unknown_format_is_a_usage_error() {
    let dir = install("only");
    rakata()
        .args([
            "--game",
            dir.to_str().unwrap(),
            "conformance",
            "--only",
            "nope",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no format `nope`"));
}

#[test]
fn conformance_writes_one_envelope_and_the_report_files() {
    let dir = install("json");
    let report = dir.join("report.json");
    let md = dir.join("report.md");
    let out = rakata()
        .args([
            "--game",
            dir.to_str().unwrap(),
            "--format",
            "json",
            "conformance",
            "--only",
            "archive",
            "--report",
            report.to_str().unwrap(),
            "--markdown",
            md.to_str().unwrap(),
            "--known",
            dir.join("absent.toml").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut docs =
        serde_json::Deserializer::from_slice(&out.stdout).into_iter::<serde_json::Value>();
    let doc = docs.next().unwrap().unwrap();
    assert!(
        docs.next().is_none(),
        "more than one JSON document on stdout"
    );
    assert_eq!(doc["command"], "conformance");
    assert_eq!(doc["game"], dir.to_str().unwrap());
    assert_eq!(doc["result"]["formats"][0]["name"], "archive");
    assert_eq!(doc["result"]["formats"][0]["counts"]["identical"], 1);
    assert_eq!(doc["result"]["files"], 1);
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert_eq!(written["files"], 1);
    let text = std::fs::read_to_string(&md).unwrap();
    assert!(text.starts_with("# Round-trip conformance"), "{text}");
    assert!(text.contains("| `archive` | 1 | 1 | 0 | 0 | |"), "{text}");
}

#[test]
fn conformance_text_mode_prints_the_summary() {
    let dir = install("text");
    rakata()
        .args(["--game", dir.to_str().unwrap(), "conformance"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("| Format | Files | Identical |"));
}
