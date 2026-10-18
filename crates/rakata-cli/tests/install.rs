//! `rakata install` over the reader's fixtures (`--dry-run`) and over a
//! hand-made install (the executor's report, as text and as JSON).

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../rakata-patch/tests/fixtures")
        .join(name)
}

#[test]
fn dry_run_prints_the_plan_in_the_patchers_order() {
    let out = rakata()
        .args(["install", "--game", "games/swkotor", "--dry-run"])
        .arg(fixture("full"))
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    let heads: Vec<&str> = text.lines().filter(|l| l.starts_with('[')).collect();
    assert_eq!(
        heads,
        [
            "[TLKList]",
            "[InstallList]",
            "[2DAList]",
            "[GFFList]",
            "[HACKList]",
            "[CompileList]",
            "[SSFList]"
        ],
        "{text}"
    );
    assert!(text.contains("plan for"), "{text}");
}

#[test]
fn dry_run_json_wraps_the_plan_in_the_envelope() {
    let out = rakata()
        .args([
            "--format",
            "json",
            "install",
            "--game",
            "games/swkotor",
            "--dry-run",
        ])
        .arg(fixture("missing-section-dir"))
        .output()
        .unwrap();
    // The folder does not exist: one envelope, one RE0001, exit 1.
    assert_eq!(out.status.code(), Some(1));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["diagnostics"][0]["code"], "RE0001");

    let out = rakata()
        .args([
            "--format",
            "json",
            "install",
            "--game",
            "games/swkotor",
            "--option",
            "1",
            "--dry-run",
        ])
        .arg(fixture("options"))
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["command"], "install");
    assert_eq!(doc["game"], "games/swkotor");
    assert_eq!(doc["result"]["dry_run"], true);
    assert_eq!(doc["result"]["option"]["ini_name"], "changes.ini");
    assert_eq!(doc["result"]["plan"]["steps"][0]["phase"], "install");
    assert_eq!(doc["result"]["plan"]["steps"][0]["op"], "install_file");
    assert_eq!(doc["inputs"].as_array().unwrap().len(), 2);
}

#[test]
fn a_missing_section_is_a_warning_and_the_plan_goes_on() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy(
        fixture("missing-section.ini"),
        dir.path().join("changes.ini"),
    )
    .unwrap();
    let out = rakata()
        .args(["install", "--game", "games/swkotor", "--dry-run"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("RW0301"), "{err}");
    assert!(err.contains("[repeating_blaster]"), "{err}");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("ChangeRow0 change row #12"), "{text}");
    assert!(!text.contains("ChangeRow12"), "{text}");
}

#[test]
fn install_needs_the_game_and_an_option_when_the_installer_offers_them() {
    rakata()
        .args(["install", "--dry-run"])
        .arg(fixture("full"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game"));
    rakata()
        .args(["install", "--game", "g", "--dry-run"])
        .arg(fixture("options"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--option").and(predicate::str::contains("Option B")));
    // Without --dry-run the install is opened, and a folder that is not one
    // stops it before any operation.
    let dir = tempfile::tempdir().unwrap();
    rakata()
        .args(["install", "--game"])
        .arg(dir.path())
        .arg(fixture("full"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE0401"));
}

/// An install with nothing in it but an empty KEY and the folders.
fn empty_game(dir: &std::path::Path) -> PathBuf {
    let root = dir.join("game");
    std::fs::create_dir_all(root.join("Override")).unwrap();
    std::fs::write(
        root.join("chitin.key"),
        rakata_archive::Key::new().to_bytes().unwrap(),
    )
    .unwrap();
    root
}

#[test]
fn an_install_reports_every_operation_with_the_hash_found_as_json_and_to_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let game = empty_game(dir.path());
    let data = dir.path().join("tslpatchdata");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::copy(
        fixture("exec/required-lone/changes.ini"),
        data.join("changes.ini"),
    )
    .unwrap();
    std::fs::write(data.join("k_copy.ncs"), b"copied bytecode").unwrap();
    let report = dir.path().join("report.json");
    let out = rakata()
        .args(["--format", "json", "install", "--game"])
        .arg(&game)
        .arg(&data)
        .arg("--report")
        .arg(&report)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let rec = &doc["result"]["report"]["records"][0];
    assert_eq!(rec["target"], "Override/k_copy.ncs");
    assert_eq!(rec["outcome"], "written");
    let sha = rakata_res::ContentHash::of(b"copied bytecode").to_hex();
    assert_eq!(rec["found"], sha.as_str());
    assert_eq!(doc["result"]["ok"], true);
    let file: serde_json::Value = serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    assert_eq!(file["report"]["records"][0]["found"], sha.as_str());

    // Again as text: the summary line and the hash found, abbreviated.
    let text = rakata()
        .args(["install", "--game"])
        .arg(&game)
        .arg(&data)
        .output()
        .unwrap();
    assert_eq!(text.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&text.stdout);
    assert!(stdout.contains("1 written and read back"), "{stdout}");
    assert!(stdout.contains(&sha[..16]), "{stdout}");
}
