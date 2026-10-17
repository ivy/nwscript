//! `nws check` end to end, on scripts written to a temporary directory.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn nws() -> Command {
    Command::cargo_bin("nws").unwrap()
}

/// A directory with a small nwscript.nss, an include and the given scripts.
fn dir(test: &str, scripts: &[(&str, &str)]) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nws-check-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("nwscript.nss"),
        "int TRUE = 1;\r\nvoid PrintInteger(int nInteger);\r\n",
    )
    .unwrap();
    std::fs::write(d.join("K_INC_ONE.nss"), "int One() { return TRUE; }\r\n").unwrap();
    for (name, src) in scripts {
        std::fs::write(d.join(name), src).unwrap();
    }
    d
}

#[test]
fn a_clean_script_exits_0() {
    let d = dir(
        "clean",
        &[(
            "ok.nss",
            "#include \"k_inc_one\"\r\nvoid main() { PrintInteger(One()); }\r\n",
        )],
    );
    nws()
        .args(["--game", "k1", "check"])
        .arg(d.join("ok.nss"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("checked 1 file: no errors"));
}

#[test]
fn errors_exit_1_with_the_code() {
    let d = dir(
        "errors",
        &[
            ("ok.nss", "void main() { }\r\n"),
            ("bad.nss", "void main() { PrintInteger(\"x\"); }\r\n"),
        ],
    );
    nws()
        .args(["--game", "k1", "--color", "never", "check"])
        .arg(d.join("ok.nss"))
        .arg(d.join("bad.nss"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E0309"))
        .stdout(predicate::str::contains("checked 2 files: 1 with errors"));
}

#[test]
fn json_names_every_input_and_each_files_outcome() {
    let d = dir(
        "json",
        &[(
            "ok.nss",
            "#include \"k_inc_one\"\r\nint StartingConditional() { return One(); }\r\n",
        )],
    );
    let out = nws()
        .args(["--game", "k1", "--format", "json", "check"])
        .arg(d.join("ok.nss"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["command"], "check");
    let roles: Vec<&str> = doc["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, ["source", "nwscript", "include"]);
    assert_eq!(doc["result"]["files"][0]["entry"], "StartingConditional");
    assert_eq!(doc["result"]["files"][0]["ok"], true);
}

#[test]
fn without_nwscript_it_is_e0002() {
    let d = std::env::temp_dir().join(format!("nws-check-none-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("a.nss"), "void main() { }\r\n").unwrap();
    nws()
        .args(["--game", "k1", "check"])
        .arg(d.join("a.nss"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E0002"));
}

#[test]
fn check_needs_a_game() {
    nws()
        .args(["check", "x.nss"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game"));
}

#[test]
fn compile_writes_bytecode_and_assembly_and_goes_on_past_errors() {
    let d = dir(
        "compile",
        &[
            (
                "ok.nss",
                "#include \"k_inc_one\"\r\nvoid main() { PrintInteger(One()); }\r\n",
            ),
            ("bad.nss", "void main() { PrintInteger(\"x\"); }\r\n"),
        ],
    );
    let out = d.join("out");
    let res = nws()
        .args(["--game", "k1", "--format", "json", "compile"])
        .arg(d.join("bad.nss"))
        .arg(d.join("ok.nss"))
        .arg("--out-dir")
        .arg(&out)
        .args(["--emit", "ncs,asm"])
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(1));
    let doc: serde_json::Value = serde_json::from_slice(&res.stdout).unwrap();
    assert_eq!(doc["result"]["files"][0]["ok"], false);
    assert_eq!(doc["result"]["files"][1]["ok"], true);
    assert_eq!(doc["outputs"].as_array().unwrap().len(), 2);
    let ncs = std::fs::read(out.join("ok.ncs")).unwrap();
    assert!(ncs.starts_with(b"NCS V1.0"));
    let asm = std::fs::read_to_string(out.join("ok.asm")).unwrap();
    assert!(asm.contains("JSR"), "{asm}");
    assert!(!out.join("bad.ncs").exists());
}

#[test]
fn compile_a_library_is_e0401() {
    let d = dir("library", &[("lib.nss", "int F() { return 1; }\r\n")]);
    nws()
        .args(["--game", "k1", "compile"])
        .arg(d.join("lib.nss"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("error[E0401]"));
}
