use assert_cmd::Command;
use predicates::prelude::*;

fn setup() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nws-diff-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("nwscript.nss"), "void PrintInteger(int n);\r\n").unwrap();
    let progs = [
        (
            "a",
            "    JSR main\n    RETN\nmain:\n    CONSTI 1\n    CONSTI 2\n    ADDII\n    ACTION 0, 1\n    RETN\n",
        ),
        // The same behaviour, encoded differently: the sum folded.
        (
            "b",
            "    JSR main\n    RETN\nmain:\n    CONSTI 3\n    ACTION 0, 1\n    RETN\n",
        ),
        // Different behaviour.
        (
            "c",
            "    JSR main\n    RETN\nmain:\n    CONSTI 4\n    ACTION 0, 1\n    RETN\n",
        ),
    ];
    for (n, asm) in progs {
        std::fs::write(d.join(format!("{n}.asm")), asm).unwrap();
        Command::cargo_bin("nws")
            .unwrap()
            .arg("asm")
            .arg(d.join(format!("{n}.asm")))
            .assert()
            .code(0);
    }
    d
}

#[test]
fn different_bytes_same_behaviour() {
    let d = setup();
    Command::cargo_bin("nws")
        .unwrap()
        .arg("diff")
        .arg(d.join("a.ncs"))
        .arg(d.join("b.ncs"))
        .assert()
        .code(3)
        .stdout(
            predicate::str::contains("bytes: different")
                .and(predicate::str::contains(
                    "instructions: different at instruction 2",
                ))
                .and(predicate::str::contains("trace: same")),
        );
    Command::cargo_bin("nws")
        .unwrap()
        .args(["diff", "--level", "trace"])
        .arg(d.join("a.ncs"))
        .arg(d.join("b.ncs"))
        .assert()
        .code(0);
}

#[test]
fn different_behaviour_names_the_call() {
    let d = setup();
    Command::cargo_bin("nws")
        .unwrap()
        .args(["diff", "--level", "trace"])
        .arg(d.join("b.ncs"))
        .arg(d.join("c.ncs"))
        .assert()
        .code(3)
        .stdout(predicate::str::contains(
            "call 1: PrintInteger(3) (unmodelled) against PrintInteger(4) (unmodelled)",
        ));
}

#[test]
fn identical_files_and_json() {
    let d = setup();
    let out = Command::cargo_bin("nws")
        .unwrap()
        .args(["--format", "json", "diff"])
        .arg(d.join("a.ncs"))
        .arg(d.join("a.ncs"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["result"]["bytes"]["status"], "same");
    assert_eq!(doc["result"]["trace"]["status"], "same");
}

#[test]
fn trace_without_a_table_is_skipped() {
    let d = setup();
    let lone = std::env::temp_dir().join(format!("nws-diff-lone-{}", std::process::id()));
    std::fs::create_dir_all(&lone).unwrap();
    std::fs::copy(d.join("a.ncs"), lone.join("a.ncs")).unwrap();
    Command::cargo_bin("nws")
        .unwrap()
        .arg("diff")
        .arg(lone.join("a.ncs"))
        .arg(lone.join("a.ncs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("trace: skipped"));
}
