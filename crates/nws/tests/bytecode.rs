//! `nws disasm` and `nws asm`, on hand-built bytecode and over the corpus.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn nws() -> Command {
    Command::cargo_bin("nws").unwrap()
}

/// `void main() { }` as BioWare's compiler lays it out: JSR main, RETN, main: RETN.
fn tiny_ncs() -> Vec<u8> {
    let mut b = b"NCS V1.0".to_vec();
    b.push(0x42);
    b.extend_from_slice(&23u32.to_be_bytes());
    b.extend_from_slice(&[0x1e, 0x00, 0, 0, 0, 8]); // JSR +8
    b.extend_from_slice(&[0x20, 0x00]); // RETN
    b.extend_from_slice(&[0x20, 0x00]); // RETN
    b
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nws-bytecode-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.join(name)
}

#[test]
fn disasm_then_asm_is_identical() {
    let ncs = tmp("tiny.ncs");
    std::fs::write(&ncs, tiny_ncs()).unwrap();
    let text = nws().arg("disasm").arg(&ncs).output().unwrap();
    assert_eq!(text.status.code(), Some(0));
    let s = String::from_utf8(text.stdout.clone()).unwrap();
    assert!(
        s.contains("JSR          sub_15") && s.contains("sub_15:"),
        "{s}"
    );
    let out = tmp("back.ncs");
    nws()
        .args(["asm", "-", "-o"])
        .arg(&out)
        .write_stdin(text.stdout)
        .assert()
        .code(0);
    assert_eq!(std::fs::read(out).unwrap(), tiny_ncs());
}

#[test]
fn asm_errors_exit_1_with_codes() {
    let src = tmp("bad.asm");
    std::fs::write(&src, "    JMP nowhere\n    FROB\n").unwrap();
    nws()
        .arg("asm")
        .arg(&src)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E0112").and(predicate::str::contains("E0110")));
}

#[test]
fn disasm_of_garbage_is_a_diagnostic() {
    let f = tmp("garbage.ncs");
    std::fs::write(&f, b"not an ncs file").unwrap();
    let out = nws()
        .args(["--format", "json", "disasm"])
        .arg(&f)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["diagnostics"][0]["code"], "E0101");
}

#[test]
fn missing_input_is_a_usage_error() {
    nws()
        .args(["disasm", "/nonexistent/x.ncs"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot read"));
}

fn corpus() -> Option<PathBuf> {
    let dir = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".cache/nwscript/corpus/k1"),
    };
    dir.join("ncs").is_dir().then_some(dir)
}

fn all_ncs(root: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.join("ncs"), root.join("modules"), root.join("rims")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "ncs") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The acceptance check for the assembly form: through the real binary,
/// `nws disasm X | nws asm - -o Y` gives Y identical to X for every file.
#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn corpus_pipeline() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let files = all_ncs(&root);
    let bin = assert_cmd::cargo::cargo_bin("nws");
    let failures = std::sync::Mutex::new(Vec::new());
    let next = std::sync::Mutex::new(files.iter());
    std::thread::scope(|s| {
        for _ in 0..std::thread::available_parallelism().map_or(4, |n| n.get()) {
            s.spawn(|| {
                while let Some(f) = next.lock().unwrap().next() {
                    let text = std::process::Command::new(&bin)
                        .arg("disasm")
                        .arg(f)
                        .output()
                        .unwrap();
                    let mut asm = std::process::Command::new(&bin)
                        .args(["asm", "-", "-o", "-"])
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                        .unwrap();
                    use std::io::Write;
                    asm.stdin.take().unwrap().write_all(&text.stdout).unwrap();
                    let out = asm.wait_with_output().unwrap();
                    if !out.status.success() || out.stdout != std::fs::read(f).unwrap() {
                        failures.lock().unwrap().push(f.display().to_string());
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().unwrap();
    println!(
        "pipeline: {}/{} byte-identical",
        files.len() - failures.len(),
        files.len()
    );
    assert!(
        failures.is_empty(),
        "{:?}",
        &failures[..failures.len().min(10)]
    );
}
