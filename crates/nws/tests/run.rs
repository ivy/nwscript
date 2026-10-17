//! `nws run` on hand-written bytecode and a two-routine nwscript.nss.

use assert_cmd::Command;
use predicates::prelude::*;

fn setup() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nws-run-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("nwscript.nss"),
        "int Random(int nMax);\r\nvoid PrintString(string s);\r\n",
    )
    .unwrap();
    // int StartingConditional() { PrintString("hi"); return Random(3) == 0; }
    let asm = "    RSADDI\n    JSR sc\n    RETN\nsc:\n    CONSTS \"hi\"\n    ACTION 1, 1\n    CONSTI 3\n    ACTION 0, 1\n    CONSTI 0\n    EQUALII\n    CPDOWNSP -8, 4\n    MOVSP -4\n    RETN\n";
    std::fs::write(d.join("t.asm"), asm).unwrap();
    Command::cargo_bin("nws")
        .unwrap()
        .arg("asm")
        .arg(d.join("t.asm"))
        .assert()
        .code(0);
    d
}

#[test]
fn run_traces_and_returns() {
    let d = setup();
    Command::cargo_bin("nws")
        .unwrap()
        .args(["--game", "k1", "run", "--trace"])
        .arg(d.join("t.ncs"))
        .assert()
        .code(0)
        .stdout(
            predicate::str::contains("PrintString(\"hi\") (unmodelled)")
                .and(predicate::str::contains("result: 1")),
        );
}

#[test]
fn run_with_the_world_model_and_json() {
    let d = setup();
    let out = Command::cargo_bin("nws")
        .unwrap()
        .args([
            "--game", "k1", "--format", "json", "run", "--engine", "world", "--seed", "3",
        ])
        .arg(d.join("t.ncs"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["result"]["trace"][0]["name"], "PrintString");
    assert_eq!(doc["result"]["trace"][1]["modelled"], true);
    assert!(
        doc["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["role"] == "nwscript")
    );
}

#[test]
fn run_needs_a_game_and_a_table() {
    let d = setup();
    Command::cargo_bin("nws")
        .unwrap()
        .arg("run")
        .arg(d.join("t.ncs"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game"));
    let lone = std::env::temp_dir().join(format!("nws-run-lone-{}", std::process::id()));
    std::fs::create_dir_all(&lone).unwrap();
    std::fs::copy(d.join("t.ncs"), lone.join("t.ncs")).unwrap();
    Command::cargo_bin("nws")
        .unwrap()
        .args(["--game", "k1", "run"])
        .arg(lone.join("t.ncs"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("nwscript.nss"));
}
