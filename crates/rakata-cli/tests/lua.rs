//! `rakata lua`: a script's arguments and `--game` as its only inputs, its
//! errors as diagnostics with their codes.

use std::fs;
use std::path::Path;

use assert_cmd::Command;

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

fn script(dir: &Path, text: &str) -> String {
    let p = dir.join("main.lua");
    fs::write(&p, text).unwrap();
    p.display().to_string()
}

#[test]
fn the_reference_documents_every_table() {
    let out = rakata().args(["lua", "--reference"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stdout);
    for table in [
        "rakata.Install",
        "rakata.Mod",
        "rakata.twoda",
        "rakata.gff",
        "rakata.lip",
        "rakata.tlk",
        "rakata.dialog",
        "rakata.author",
        "rakata.Error",
        "rakata.fs",
    ] {
        assert!(text.contains(table), "the reference lacks {table}");
    }
}

#[test]
fn arguments_and_game_reach_the_script_wherever_game_stands() {
    let dir = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let game = tempfile::tempdir().unwrap();
    let s = script(
        dir.path(),
        r#"
        local t = rakata.twoda.new({"name", "type"})
        rakata.author.globals.declare(t, "OAJ_DONE", "Boolean")
        rakata.fs.write(rakata.fs.join(rakata.args[1], "globalcat.2da"), t:to_bytes())
        print(#rakata.args, rakata.args[2], rakata.game ~= nil)
    "#,
    );
    let o = rakata()
        .args(["lua", &s, out.path().to_str().unwrap(), "--flag"])
        .args(["--game", game.path().to_str().unwrap()])
        .output();
    // `--flag` is not rakata's: it has to follow `--`.
    assert_eq!(o.unwrap().status.code(), Some(2));
    let o = rakata()
        .args(["lua", &s, out.path().to_str().unwrap()])
        .args(["--game", game.path().to_str().unwrap(), "--", "--flag"])
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&o.stdout), "2\t--flag\ttrue\n");
    let t =
        rakata::twoda::TwoDa::parse(&fs::read(out.path().join("globalcat.2da")).unwrap()).unwrap();
    assert_eq!(t.get(0, "name").unwrap(), "OAJ_DONE");
}

#[test]
fn an_uncaught_authoring_error_exits_1_with_its_code() {
    let dir = tempfile::tempdir().unwrap();
    let s = script(
        dir.path(),
        r#"
        print("before")
        local t = rakata.twoda.new({"name", "type"})
        rakata.author.globals.declare(t, "A", "Boolean")
        rakata.author.globals.declare(t, "A", "Number")
    "#,
    );
    let o = rakata().args(["lua", &s]).output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(stderr.contains("RE1404"), "{stderr}");
    assert_eq!(String::from_utf8_lossy(&o.stdout), "before\n");

    let o = rakata()
        .args(["--format", "json", "lua", &s])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    let doc: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(doc["command"], "lua");
    assert_eq!(doc["diagnostics"][0]["code"], "RE1404");
    assert_eq!(doc["result"]["output"], "before\n");
}

#[test]
fn a_read_outside_the_inputs_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let s = script(dir.path(), r#"print(rakata.fs.read("/etc/hostname"))"#);
    let o = rakata().args(["lua", &s]).output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("RE1801"));
    assert!(o.stdout.is_empty());
}
