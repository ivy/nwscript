//! `rakata refs` over an install built here, and the transition chain and
//! the Juhani mod's global over the user's modded build when
//! `$RAKATA_BUILD` names it. No game bytes are committed.

use assert_cmd::Command;
use rakata::twoda::TwoDa;
use rakata_archive::Key;
use serde_json::Value as Json;
use std::path::Path;
use std::time::Instant;

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

fn build(game: &Path, db: &Path) {
    json(&[
        "index",
        "build",
        "--game",
        s(game),
        "--db",
        s(db),
        "--tree",
        "t",
    ]);
}

#[test]
fn refs_over_a_small_install_in_text_and_json() {
    let dir = tempfile::tempdir().unwrap();
    let game = dir.path().join("game");
    std::fs::create_dir_all(game.join("Override")).unwrap();
    std::fs::write(game.join("chitin.key"), Key::new().to_bytes().unwrap()).unwrap();
    let mut t = TwoDa::new(["name", "type"]);
    t.add_row("0", &[("name", "K_SWG_HELENA"), ("type", "Number")])
        .unwrap();
    std::fs::write(game.join("Override/globalcat.2da"), t.to_bytes().unwrap()).unwrap();
    let db = dir.path().join("idx.sqlite");
    build(&game, &db);

    let r = json(&["refs", "k_swg_helena", "--db", s(&db)]);
    assert_eq!(r["command"], "refs", "{r}");
    let res = &r["result"];
    assert_eq!(res["direction"], "to");
    assert_eq!(res["tried"], serde_json::json!(["resref", "tag", "global"]));
    assert_eq!(res["matched"], serde_json::json!(["global"]));
    assert_eq!(res["hits"][0]["resource"], "globalcat.2da");
    assert_eq!(res["hits"][0]["site"], "row 0");
    assert_eq!(
        res["hits"][0]["locations"][0]["path"],
        "Override/globalcat.2da"
    );

    let out = rakata()
        .args(["refs", "K_SWG_HELENA", "--kind", "global", "--db", s(&db)])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.starts_with("references to `K_SWG_HELENA`: 1 direct"),
        "{text}"
    );
    assert!(
        text.contains("globalcat.2da  row 0  Override/globalcat.2da"),
        "{text}"
    );

    // An empty answer is qualified by coverage.
    let r = json(&["refs", "k_nothere", "--db", s(&db)]);
    assert_eq!(r["result"]["hits"], serde_json::json!([]));
    assert_eq!(r["result"]["coverage"]["failed"], 0, "{r}");
    assert_eq!(r["diagnostics"][0]["code"], "RW1505", "{r}");

    // From a resource; a symbol that cannot be of its kind.
    let r = json(&["refs", "globalcat.2da", "--from", "--db", s(&db)]);
    assert_eq!(r["result"]["hits"][0]["value"], "K_SWG_HELENA");
    let out = rakata()
        .args(["refs", "abc", "--kind", "strref", "--db", s(&db)])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("RE1506"));
    let out = rakata()
        .args(["refs", "nope.ncs", "--from", "--db", s(&db)])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("RE1507"));
}

/// `rakata refs ARGS` over `db`, timed, with the names of the resources it
/// found.
fn timed(db: &Path, args: &[&str]) -> (Json, Vec<String>) {
    let mut all = vec!["refs", "--db", s(db)];
    all.extend_from_slice(args);
    let start = Instant::now();
    let r = json(&all);
    let wall = start.elapsed().as_secs_f64() * 1000.0;
    let ms = r["result"]["seconds"].as_f64().unwrap() * 1000.0;
    eprintln!(
        "rakata refs {}: {ms:.1} ms in process, {wall:.1} ms wall",
        args.join(" ")
    );
    assert!(ms < 100.0, "rakata refs {args:?} took {ms} ms");
    let names = r["result"]["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["resource"].as_str().unwrap().to_string())
        .collect();
    (r, names)
}

#[test]
fn the_transition_chain_and_oaj_done_over_the_modded_build() {
    let Ok(game) = std::env::var("RAKATA_BUILD") else {
        eprintln!("skipped: $RAKATA_BUILD is unset; set it to a modded KotOR 1 build");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("idx.sqlite");
    build(Path::new(&game), &db);

    let (_, names) = timed(&db, &["42724", "--kind", "strref"]);
    assert!(names.iter().any(|n| n == "k_transabort.dlg"), "{names:?}");
    let (_, names) = timed(&db, &["k_transabort", "--kind", "resref"]);
    assert!(names.iter().any(|n| n == "k_trans_abort.utp"), "{names:?}");
    let (r, _) = timed(&db, &["k_trans_abort", "--kind", "resref"]);
    let script = r["result"]["hits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["resource"] == "k_trg_transfail.ncs")
        .unwrap_or_else(|| panic!("k_trg_transfail.ncs is not among {r}"));
    assert_eq!(script["operand"], "CONSTS");

    let (r, names) = timed(&db, &["OAJ_DONE", "--kind", "global"]);
    for n in ["globalcat.2da", "k_oaj_c_done.ncs", "k_oaj_up.ncs"] {
        assert!(names.iter().any(|x| x == n), "{n} not in {names:?}");
    }
    assert!(
        r["result"]["hits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["resource"].as_str().unwrap().ends_with(".dlg")
                && h["site"].as_str().unwrap().ends_with(".Active")),
        "no dialogue condition reads OAJ_DONE: {r}"
    );
}
