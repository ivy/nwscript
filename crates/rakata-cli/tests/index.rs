//! `rakata index` over an install built here, and the acceptance queries
//! over the user's installs when `$RAKATA_GAME_K1` and `$RAKATA_BUILD`
//! name them. No game bytes are committed.

use assert_cmd::Command;
use rakata::twoda::TwoDa;
use rakata_archive::Key;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn json(args: &[&str]) -> Json {
    let out = rakata()
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// An install: an empty `chitin.key`, `Override/globalcat.2da`, and an
/// executable outside the folders the game loads resources from.
fn install(root: &Path) {
    std::fs::create_dir_all(root.join("Override")).unwrap();
    std::fs::write(root.join("chitin.key"), Key::new().to_bytes().unwrap()).unwrap();
    let mut t = TwoDa::new(["name", "type"]);
    t.add_row("0", &[("name", "K_SWG_HELENA"), ("type", "Number")])
        .unwrap();
    std::fs::write(root.join("Override/globalcat.2da"), t.to_bytes().unwrap()).unwrap();
    std::fs::write(root.join("swkotor.exe"), b"MZ").unwrap();
}

#[test]
fn build_query_digest_and_coverage_over_a_small_install() {
    let dir = tempfile::tempdir().unwrap();
    let game = dir.path().join("game");
    install(&game);
    let db = dir.path().join("idx.sqlite");
    let r = json(&[
        "index",
        "build",
        "--game",
        s(&game),
        "--db",
        s(&db),
        "--tree",
        "t",
    ]);
    assert_eq!(r["result"]["files"], 2, "{r}");
    assert_eq!(r["result"]["outside"], 1);
    assert_eq!(r["result"]["coverage"]["parsed"], 1);
    assert_eq!(r["result"]["coverage"]["container"], 1);

    let q = json(&[
        "index",
        "query",
        "--db",
        s(&db),
        "SELECT kind, value, site FROM refs",
    ]);
    assert_eq!(
        q["result"]["rows"],
        serde_json::json!([["global", "K_SWG_HELENA", "row 0"]])
    );
    let text = rakata()
        .args(["index", "query", "--db", s(&db), "SELECT label FROM trees"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&text.stdout),
        "label\nt\n(1 rows)\n"
    );

    let d1 = json(&["index", "digest", "--db", s(&db), "--tree", "t"]);
    let d = d1["result"]["digest"].as_str().unwrap();
    assert_eq!(d.len(), 64);
    let c = json(&["index", "coverage", "--db", s(&db), "--list"]);
    let rows = &c["result"]["trees"][0]["rows"];
    assert_eq!(rows.as_array().unwrap().len(), 3, "{c}");

    // A tree the database does not hold, and a query SQLite rejects.
    let out = rakata()
        .args(["index", "digest", "--db", s(&db), "--tree", "nope"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("RE1503"));
    let out = rakata()
        .args(["index", "query", "--db", s(&db), "DELETE FROM refs"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("RE1504"));
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

/// Index the tree `var` names and run the acceptance queries.
fn acceptance(var: &str) {
    let Some(game) = tree(var) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("idx.sqlite");
    let r = json(&[
        "index",
        "build",
        "--game",
        s(&game),
        "--db",
        s(&db),
        "--tree",
        "t",
    ]);
    let r = &r["result"];
    eprintln!(
        "{var}: {} s, {} bytes, {} hashes parsed",
        r["seconds"]["total"], r["db_bytes"], r["hashes_parsed"]
    );
    assert_eq!(r["coverage"].get("failed"), None, "{}", r["coverage"]);

    // The k_pman_aioff finding: the Juhani script that turns AI off.
    let q = json(&[
        "index",
        "query",
        "--db",
        s(&db),
        "SELECT DISTINCT l.member FROM calls c JOIN locations l ON l.hash = c.hash WHERE c.routine = 'ExecuteScript' AND json_extract(c.args, '$[0]') = 'k_pman_aioff'",
    ]);
    let members: Vec<&str> = q["result"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[0].as_str().unwrap())
        .collect();
    assert!(members.contains(&"k_pdan_juhani27.ncs"), "{members:?}");

    // Full text: the Taris transformation scripts' calls, and dialogue
    // nodes through the talk table.
    let q = json(&[
        "index",
        "query",
        "--db",
        s(&db),
        "SELECT DISTINCT l.member FROM search s JOIN locations l ON l.hash = s.hash WHERE search MATCH 'rakghoul*' AND s.kind = 'call'",
    ]);
    let scripts: Vec<&str> = q["result"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[0].as_str().unwrap())
        .collect();
    assert!(scripts.contains(&"k_ptar_victrans.ncs"), "{scripts:?}");
    let q = json(&[
        "index",
        "query",
        "--db",
        s(&db),
        "SELECT count(*) FROM search s JOIN dialog_nodes n ON n.strref = CAST(s.site AS INTEGER) WHERE search MATCH 'rakghoul*' AND s.kind = 'string'",
    ]);
    assert!(q["result"]["rows"][0][0].as_i64().unwrap() > 0);

    // Every file of the tree has a coverage row.
    let c = json(&["index", "coverage", "--db", s(&db), "--tree", "t"]);
    let files = c["result"]["trees"][0]["files"].as_u64().unwrap();
    assert_eq!(
        files,
        r["files"].as_u64().unwrap() + r["outside"].as_u64().unwrap()
    );
}

#[test]
fn the_acceptance_queries_over_the_vanilla_install() {
    acceptance("RAKATA_GAME_K1");
}

#[test]
fn the_acceptance_queries_over_the_modded_build() {
    acceptance("RAKATA_BUILD");
}
