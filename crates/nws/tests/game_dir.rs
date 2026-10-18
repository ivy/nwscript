//! `--game-dir`: includes and nwscript.nss from an install, through
//! rakata-game. The tests over a real install read `$RAKATA_GAME_K1`, a
//! vanilla KotOR 1 install, and skip, naming the variable, when it is
//! unset. No game bytes are committed.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

const GAME_VAR: &str = "RAKATA_GAME_K1";

fn nws() -> Command {
    Command::cargo_bin("nws").unwrap()
}

fn vanilla() -> Option<PathBuf> {
    match std::env::var(GAME_VAR) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${GAME_VAR} is unset; set it to a vanilla KotOR 1 install");
            None
        }
    }
}

/// A fresh directory for one test, under the system's temporary directory.
fn scratch(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nws-game-dir-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).expect("one JSON document on stdout")
}

fn inputs(doc: &serde_json::Value) -> Vec<(String, String)> {
    doc["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["role"].as_str().unwrap().to_owned(),
                i["path"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[test]
fn game_dir_needs_the_game_profile() {
    let d = scratch("needs-game");
    nws()
        .args(["--game-dir", d.to_str().unwrap(), "explain", "E0001"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "--game-dir needs the game profile",
        ));
}

#[test]
fn a_folder_without_chitin_key_is_a_usage_error_naming_the_flag() {
    let d = scratch("not-an-install");
    nws()
        .args([
            "--game",
            "k1",
            "--game-dir",
            d.to_str().unwrap(),
            "explain",
            "E0001",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game-dir").and(predicate::str::contains("RE0401")));
}

#[test]
fn module_needs_game_dir() {
    nws()
        .args(["--game", "k1", "--module", "danm14ac", "explain", "E0001"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game-dir"));
}

#[test]
fn explain_knows_the_command_codes() {
    nws()
        .args(["explain", "ne0001"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "NE0001: the install --game-dir names holds no nwscript.nss",
        ));
}

/// An install with a KEY that indexes nothing: a chitin.key and no BIFs.
fn empty_install(dir: &Path) {
    let key = rakata_archive::Key::new().to_bytes().unwrap();
    std::fs::write(dir.join("chitin.key"), key).unwrap();
}

#[test]
fn an_install_without_nwscript_is_ne0001_not_a_panic() {
    let d = scratch("no-nwscript");
    let game = d.join("game");
    std::fs::create_dir_all(&game).unwrap();
    empty_install(&game);
    let src = d.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let script = src.join("a.nss");
    std::fs::write(&script, "void main() {}\n").unwrap();
    let out = nws()
        .args(["--game", "k1", "--format", "json", "--game-dir"])
        .arg(&game)
        .arg("check")
        .arg(&script)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc = json(&out);
    let codes: Vec<&str> = doc["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["NE0001"]);
    let message = doc["diagnostics"][0]["message"].as_str().unwrap();
    assert!(message.contains(game.to_str().unwrap()), "{message}");
}

#[test]
fn a_module_the_install_lacks_is_a_usage_error() {
    let d = scratch("no-module");
    empty_install(&d);
    nws()
        .args([
            "--game",
            "k1",
            "--game-dir",
            d.to_str().unwrap(),
            "--module",
            "nosuch",
        ])
        .args(["explain", "E0001"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--module nosuch"));
}

#[test]
fn nwscript_and_includes_come_from_the_install() {
    let Some(game) = vanilla() else { return };
    let d = scratch("vanilla");
    let script = d.join("a.nss");
    std::fs::write(
        &script,
        "#include \"k_inc_generic\"\nvoid main() { GN_SetDayNightPresence(AMBIENT_PRESENCE_DAY_ONLY); }\n",
    )
    .unwrap();
    let out = nws()
        .args(["--game", "k1", "--format", "json", "--game-dir"])
        .arg(&game)
        .arg("compile")
        .arg(&script)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc = json(&out);
    let inputs = inputs(&doc);
    let from = |role: &str| -> Vec<&str> {
        inputs
            .iter()
            .filter(|(r, _)| r == role)
            .map(|(_, p)| p.as_str())
            .collect()
    };
    assert!(
        from("nwscript")
            .iter()
            .any(|p| p.to_ascii_lowercase().contains("scripts.bif")),
        "nwscript.nss should come from scripts.bif: {inputs:?}"
    );
    assert!(
        from("include")
            .iter()
            .any(|p| p.to_ascii_lowercase().contains("scripts.bif")),
        "k_inc_generic should come from scripts.bif: {inputs:?}"
    );
    assert!(d.join("a.ncs").is_file());
}

#[test]
fn explicit_directories_take_precedence_over_the_install() {
    let Some(game) = vanilla() else { return };
    let d = scratch("precedence");
    let inc = d.join("inc");
    std::fs::create_dir_all(&inc).unwrap();
    std::fs::write(inc.join("k_inc_generic.nss"), "int FROM_DIR = 1;\n").unwrap();
    let script = d.join("a.nss");
    std::fs::write(
        &script,
        "#include \"k_inc_generic\"\nvoid main() { PrintInteger(FROM_DIR); }\n",
    )
    .unwrap();
    let out = nws()
        .args(["--game", "k1", "--format", "json", "--game-dir"])
        .arg(&game)
        .arg("-I")
        .arg(&inc)
        .arg("check")
        .arg(&script)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let inputs = inputs(&json(&out));
    let include = inputs
        .iter()
        .find(|(r, _)| r == "include")
        .map(|(_, p)| p.as_str())
        .expect("one include read");
    assert_eq!(Path::new(include), inc.join("k_inc_generic.nss"));
    assert!(
        inputs
            .iter()
            .any(|(r, p)| r == "nwscript" && p.to_ascii_lowercase().contains(".bif")),
        "nwscript.nss still comes from the install: {inputs:?}"
    );
}
