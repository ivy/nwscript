use assert_cmd::Command;
use predicates::prelude::*;

fn dir() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nws-fn-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("nwscript.nss"), "int TRUE = 1;\r\n\r\n// Returns a global number.\r\nint GetGlobalNumber(string sIdentifier);\r\n\r\n// Set it.\r\nvoid SetGlobalNumber(string sIdentifier, int nValue = TRUE);\r\n").unwrap();
    d
}

#[test]
fn prints_comment_and_prototype() {
    Command::cargo_bin("nws")
        .unwrap()
        .arg("-I")
        .arg(dir())
        .args(["fn", "getglobalnumber"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "// Returns a global number.\nint GetGlobalNumber(string sIdentifier);    // routine 0",
        ));
}

#[test]
fn grep_lists_matches_and_defaults() {
    Command::cargo_bin("nws")
        .unwrap()
        .arg("-I")
        .arg(dir())
        .args(["fn", "--grep", "global"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "void SetGlobalNumber(string sIdentifier, int nValue=1);    // routine 1",
        ));
}

#[test]
fn unknown_names_get_suggestions_and_exit_1() {
    Command::cargo_bin("nws")
        .unwrap()
        .arg("-I")
        .arg(dir())
        .args(["fn", "GetGlobalNumbr"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("did you mean GetGlobalNumber"));
}

#[test]
fn json_has_the_routine_number() {
    let out = Command::cargo_bin("nws")
        .unwrap()
        .arg("-I")
        .arg(dir())
        .args(["--format", "json", "fn", "SetGlobalNumber"])
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["result"]["matches"][0]["routine"], 1);
    assert_eq!(doc["result"]["matches"][0]["doc"], "Set it.");
}
