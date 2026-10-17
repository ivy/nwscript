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

/// The ten example tests in docs/examples pass against the corpus.
#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn docs_examples_pass() {
    let root = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => std::path::PathBuf::from(p),
        None => std::path::PathBuf::from(std::env::var_os("HOME").unwrap())
            .join(".cache/nwscript/corpus/k1"),
    };
    if !root.join("ncs").is_dir() {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    }
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples");
    Command::cargo_bin("nws")
        .unwrap()
        .args(["--game", "k1", "-I"])
        .arg(&root)
        .arg("-I")
        .arg(root.join("ncs"))
        .arg("test")
        .arg(examples)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("test result: ok. 15 passed"));
}

/// Record each example script as a snapshot, replay them all, then change
/// one script and see the first differing call.
#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn snapshots_round_trip_and_catch_a_change() {
    let root = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => std::path::PathBuf::from(p),
        None => std::path::PathBuf::from(std::env::var_os("HOME").unwrap())
            .join(".cache/nwscript/corpus/k1"),
    };
    if !root.join("ncs").is_dir() {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    }
    let d = std::env::temp_dir().join(format!("nws-snap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("world.toml"),
        "seed = 7\nnpc_party = [1]\nparty = [\"player\"]\ncontext = { speaker = \"player\" }\nglobals_number = { DAN_JON_STATE = 1 }\nglobals_boolean = { MAN_PTRAP_DONE = true }\n[[objects]]\nkind = 1\ntag = \"player\"\nis_pc = true\ninventory = [\"ptar_sithpapers\"]\n[[objects]]\nkind = 1\ntag = \"cand\"\nposition = [3.0, 4.0, 0.0]\n",
    )
    .unwrap();
    let scripts = [
        "k_con_hk47npm",
        "k_pman_misc59",
        "k_pdan_drandom07",
        "k_pman_comp35",
        "k_ptar_sithpaper",
        "k_con_canderpm",
        "k_pdan_mand04",
        "k_pman_planet77",
        "k_ptat_canhunt",
        "k_swg_bastila29",
    ];
    for s in scripts {
        std::fs::copy(
            root.join(format!("ncs/{s}.ncs")),
            d.join(format!("{s}.ncs")),
        )
        .unwrap();
        Command::cargo_bin("nws")
            .unwrap()
            .args(["--game", "k1", "-I"])
            .arg(&root)
            .arg("run")
            .arg("--world")
            .arg(d.join("world.toml"))
            .arg("--record")
            .arg(d.join(format!("{s}.snapshot.toml")))
            .arg(d.join(format!("{s}.ncs")))
            .assert()
            .code(0);
    }
    Command::cargo_bin("nws")
        .unwrap()
        .args(["--game", "k1", "-I"])
        .arg(&root)
        .arg("test")
        .arg(&d)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("10 passed"));
    let asm = Command::cargo_bin("nws")
        .unwrap()
        .arg("disasm")
        .arg(d.join("k_pman_comp35.ncs"))
        .output()
        .unwrap()
        .stdout;
    let changed = String::from_utf8(asm)
        .unwrap()
        .replace("MAN_PTRAP_DONE", "MAN_PTRAP_GONE");
    Command::cargo_bin("nws")
        .unwrap()
        .args(["asm", "-", "-o"])
        .arg(d.join("k_pman_comp35.ncs"))
        .write_stdin(changed)
        .assert()
        .code(0);
    Command::cargo_bin("nws")
        .unwrap()
        .args(["--game", "k1", "-I"])
        .arg(&root)
        .arg("test")
        .arg(&d)
        .assert()
        .code(3)
        .stdout(
            predicate::str::contains("call 1 differs").and(predicate::str::contains(
                "GetGlobalBoolean(\"MAN_PTRAP_GONE\")",
            )),
        );
    std::fs::remove_dir_all(&d).unwrap();
}

/// Code generation never sees the mode: every vanilla script accepted in
/// two modes compiles to the same bytes in both.
#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn modes_never_change_the_bytes() {
    let root = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => std::path::PathBuf::from(p),
        None => std::path::PathBuf::from(std::env::var_os("HOME").unwrap())
            .join(".cache/nwscript/corpus/k1"),
    };
    if !root.join("nss").is_dir() {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    }
    let mut sources: Vec<std::path::PathBuf> = std::fs::read_dir(root.join("nss"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().is_some_and(|n| n != "nwscript.nss"))
        .collect();
    sources.sort();
    let base = std::env::temp_dir().join(format!("nws-modes-{}", std::process::id()));
    let modes: [&[&str]; 4] = [
        &[],
        &[
            "--allow", "W0001", "--allow", "W0003", "--allow", "W0004", "--allow", "W0005",
            "--allow", "W0007", "--allow", "W0008",
        ],
        &["--compat", "nwnnsscomp"],
        &["--compat", "bioware"],
    ];
    let mut outs = Vec::new();
    for (i, m) in modes.iter().enumerate() {
        let dir = base.join(i.to_string());
        std::fs::create_dir_all(&dir).unwrap();
        Command::cargo_bin("nws")
            .unwrap()
            .args(["--game", "k1"])
            .args(*m)
            .arg("-I")
            .arg(root.join("nss"))
            .arg("compile")
            .arg("--out-dir")
            .arg(&dir)
            .args(&sources)
            .output()
            .unwrap();
        outs.push(dir);
    }
    let (mut compared, mut pairs) = (0, 0);
    for s in &sources {
        let name = format!("{}.ncs", s.file_stem().unwrap().to_string_lossy());
        let got: Vec<Option<Vec<u8>>> = outs
            .iter()
            .map(|d| std::fs::read(d.join(&name)).ok())
            .collect();
        for a in 0..got.len() {
            for b in a + 1..got.len() {
                if let (Some(x), Some(y)) = (&got[a], &got[b]) {
                    pairs += 1;
                    assert_eq!(x, y, "{name} differs between modes {a} and {b}");
                }
            }
        }
        compared += usize::from(got.iter().filter(|g| g.is_some()).count() >= 2);
    }
    println!(
        "modes: {compared} scripts compiled in two or more modes, {pairs} pairs, all identical"
    );
    assert!(compared > 1700);
    std::fs::remove_dir_all(&base).unwrap();
}
