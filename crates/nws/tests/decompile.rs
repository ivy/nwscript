//! `nws decompile` on hand-assembled bytecode, verified by the project's
//! compiler and by stand-ins for nwnnsscomp, and on the corpus.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

fn nws() -> Command {
    Command::cargo_bin("nws").unwrap()
}

fn dir(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nws-decompile-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Two engine routines: enough for the program below.
const ENGINE: &str = "int Random(int nMaxInteger);\nvoid PrintInteger(int nInteger);\n";

/// `void main() { int n = Random(3); if (n) PrintInteger(n); }`, as nwnnsscomp lays it out.
const MAIN: &str = "\
    JSR main
    RETN
main:
    RSADDI
    CONSTI 3
    ACTION 0, 1
    CPDOWNSP -8, 4
    MOVSP -4
    CPTOPSP -4, 4
    JZ end
    CPTOPSP -4, 4
    ACTION 1, 1
    JMP end
end:
    MOVSP -4
    RETN
";

/// Assemble `MAIN` into `d/x.ncs` and write the engine table beside it.
fn setup(d: &Path) -> PathBuf {
    std::fs::write(d.join("nwscript.nss"), ENGINE).unwrap();
    std::fs::write(d.join("x.asm"), MAIN).unwrap();
    nws().arg("asm").arg(d.join("x.asm")).assert().success();
    d.join("x.ncs")
}

/// A stand-in compiler: a shell script that writes `ncs` as its output.
fn fake_compiler(d: &Path, ncs: &Path) -> PathBuf {
    let script = d.join("fakecc");
    std::fs::write(
        &script,
        format!("#!/bin/sh\ncp '{}' decompiled.ncs\n", ncs.display()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    script
}

#[test]
fn no_verify_prints_the_source() {
    let d = dir("noverify");
    let ncs = setup(&d);
    nws()
        .arg("decompile")
        .arg("--no-verify")
        .arg(&ncs)
        .env_remove("NWNNSSCOMP")
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "// unchecked: recovered, not recompiled\nvoid main()\n{\n    int nLocal1 = Random(3);\n    if (nLocal1) {\n        PrintInteger(nLocal1);\n    }\n}",
        ));
}

#[test]
fn verification_needs_a_game() {
    let d = dir("nogame");
    let ncs = setup(&d);
    nws()
        .arg("decompile")
        .arg(&ncs)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game k1"));
}

#[test]
fn the_projects_compiler_verifies_by_default() {
    let d = dir("project");
    let ncs = setup(&d);
    nws()
        .args(["--game", "k1", "decompile", "--strict"])
        .arg(&ncs)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "// verified: recompiles to the original bytes\nvoid main()",
        ));
}

#[test]
fn a_compiler_that_reproduces_the_bytes_verifies() {
    let d = dir("verified");
    let ncs = setup(&d);
    let cc = fake_compiler(&d, &ncs);
    let out = nws()
        .args(["--format", "json", "decompile", "--strict", "--compiler"])
        .arg(&cc)
        .arg(&ncs)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["result"]["verified"], 1);
    assert_eq!(v["result"]["functions"][0]["name"], "main");
    assert_eq!(v["result"]["functions"][0]["status"], "verified");
    assert_eq!(v["result"]["functions"][0]["offset"], 13);
    assert!(
        v["result"]["source"]
            .as_str()
            .unwrap()
            .contains("// verified: recompiles to the original bytes")
    );
}

#[test]
fn strict_exits_3_when_a_function_differs() {
    let d = dir("strict");
    let ncs = setup(&d);
    // The stand-in compiler prints a constant: shorter than the copy it
    // replaces, so the `JZ` at 56 that jumps past it differs first.
    std::fs::write(
        d.join("other.asm"),
        MAIN.replace(
            "    CPTOPSP -4, 4\n    ACTION 1, 1",
            "    CONSTI 7\n    ACTION 1, 1",
        ),
    )
    .unwrap();
    nws().arg("asm").arg(d.join("other.asm")).assert().success();
    let cc = fake_compiler(&d, &d.join("other.ncs"));
    let out_nss = d.join("x.nss");
    nws()
        .args(["decompile", "--strict", "-o"])
        .arg(&out_nss)
        .arg("--compiler")
        .arg(&cc)
        .arg(&ncs)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("E0602"));
    let src = std::fs::read_to_string(&out_nss).unwrap();
    assert!(
        src.contains("// unverified: recompiles differently from offset 56\nvoid main()"),
        "{src}"
    );
}

#[test]
fn bytecode_without_an_entry_stub_is_e0601() {
    let d = dir("nostub");
    std::fs::write(d.join("nwscript.nss"), ENGINE).unwrap();
    std::fs::write(d.join("y.asm"), "    CONSTI 1\n    RETN\n").unwrap();
    nws().arg("asm").arg(d.join("y.asm")).assert().success();
    nws()
        .args(["decompile", "--no-verify"])
        .arg(d.join("y.ncs"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E0601"));
}

#[test]
#[ignore = "needs the corpus (NWSCRIPT_CORPUS)"]
fn a_vanilla_script_verifies_with_bioware_declarations() {
    let corpus = std::env::var_os("NWSCRIPT_CORPUS")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/nwscript/corpus/k1"))
        })
        .unwrap();
    if !corpus.join("nwscript.nss").is_file() {
        eprintln!(
            "skipping: no corpus at {} (set NWSCRIPT_CORPUS)",
            corpus.display()
        );
        return;
    }
    // Its ActionStartConversation call has the eleven arguments BioWare's declaration took.
    nws()
        .args(["--game", "k1", "--declarations", "bioware", "--nwscript"])
        .arg(corpus.join("nwscript.nss"))
        .args(["decompile", "--strict"])
        .arg(corpus.join("ncs/k_pend_door01.ncs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("ActionStartConversation("));
}
