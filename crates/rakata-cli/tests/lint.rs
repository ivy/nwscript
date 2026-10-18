//! `rakata lint` over an install built here, and over the user's installs
//! when `$RAKATA_GAME_K1` and `$RAKATA_BUILD` name them. No game bytes are
//! committed.

use assert_cmd::Command;
use rakata_archive::Key;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// The run's exit code and its JSON document.
fn json(args: &[&str]) -> (Option<i32>, Json) {
    let out = rakata()
        .args(["--format", "json", "lint"])
        .args(args)
        .output()
        .unwrap();
    let doc = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{args:?}: {e}: {}", String::from_utf8_lossy(&out.stderr)));
    (out.status.code(), doc)
}

fn codes(doc: &Json) -> Vec<&str> {
    doc["result"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap())
        .collect()
}

/// An install whose Override holds a texture as both `.tpc` and `.tga`,
/// and a creature under two spellings.
fn install(root: &Path) {
    std::fs::create_dir_all(root.join("Override")).unwrap();
    std::fs::write(root.join("chitin.key"), Key::new().to_bytes().unwrap()).unwrap();
    for f in ["a.tpc", "a.tga", "Foo.utc", "foo.utc"] {
        std::fs::write(root.join("Override").join(f), b"").unwrap();
    }
}

#[test]
fn a_finding_not_known_fails_the_run_and_a_known_list_accepts_it() {
    let dir = tempfile::tempdir().unwrap();
    let game = dir.path().join("game");
    install(&game);
    let known = dir.path().join("known.toml");
    let (code, doc) = json(&["--game", s(&game), "--write-known", s(&known)]);
    assert_eq!(code, Some(1), "{doc}");
    assert_eq!(codes(&doc), ["RE1901", "RW1903", "RW1903"]);
    assert_eq!(doc["result"]["findings"][0]["path"], "Override/a.tpc");
    assert_eq!(doc["command"], "lint");
    assert!(
        doc["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "RE1901")
    );

    let (code, doc) = json(&["--game", s(&game), "--known", s(&known)]);
    assert_eq!(code, Some(0), "{doc}");
    assert!(codes(&doc).is_empty());
    assert_eq!(doc["result"]["known"]["RW1903"], 2);

    // Only the listed paths: a checkpoint's changed.txt.
    let changed = dir.path().join("changed.txt");
    std::fs::write(&changed, "+ Override/foo.utc\n- Override/a.tpc\n").unwrap();
    let (_, doc) = json(&["--game", s(&game), "--paths", s(&changed)]);
    assert_eq!(codes(&doc), ["RW1903", "RW1903"]);

    let text = rakata()
        .args(["lint", "--game", s(&game), "--known", s(&known)])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&text.stdout).starts_with("lint: 0 findings and 3 known"),
        "{}",
        String::from_utf8_lossy(&text.stdout)
    );

    std::fs::write(&known, "[RW1901\n").unwrap();
    let (code, doc) = json(&["--game", s(&game), "--known", s(&known)]);
    assert_eq!(code, Some(1));
    assert_eq!(doc["diagnostics"][0]["code"], "RE1902");
}

fn tree(var: &str) -> Option<PathBuf> {
    match std::env::var(var) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${var} is unset; set it to a KotOR 1 install");
            None
        }
    }
}

fn vanilla_known() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/tree-known.toml")
}

#[test]
fn the_vanilla_install_has_only_the_known_findings() {
    let Some(game) = tree("RAKATA_GAME_K1") else {
        return;
    };
    let (code, doc) = json(&["--game", s(&game), "--known", s(&vanilla_known())]);
    assert_eq!(code, Some(0), "{}", doc["result"]["findings"]);
    assert_eq!(doc["result"]["unread"], 0);
    eprintln!(
        "RAKATA_GAME_K1: {} resources, {} known, {} s",
        doc["result"]["checked"], doc["result"]["known"], doc["result"]["seconds"]
    );
}

#[test]
fn the_build_holds_no_case_variants_or_tpc_shadows() {
    let Some(game) = tree("RAKATA_BUILD") else {
        return;
    };
    let (_, doc) = json(&["--game", s(&game), "--known", s(&vanilla_known())]);
    let found = codes(&doc);
    assert!(
        !found.iter().any(|c| *c == "RW1903" || *c == "RE1901"),
        "{found:?}"
    );
    assert_eq!(doc["result"]["unread"], 0);
    eprintln!(
        "RAKATA_BUILD: {} resources, {} findings, {} s",
        doc["result"]["checked"],
        found.len(),
        doc["result"]["seconds"]
    );
}
