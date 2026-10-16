//! `nws conformance` over a synthetic corpus and a shell "compiler", so the
//! classes, causes, determinism and baseline check are tested without game data.

use assert_cmd::Command;
use sha2::{Digest, Sha256};
use std::path::Path;

fn hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// Scripts named by the class the fake compiler produces for them: it copies
/// the source to the output, so the expected bytecode decides the class.
fn corpus(dir: &Path) {
    let scripts: &[(&str, &[u8], &[u8])] = &[
        ("same", b"abc", b"abc"),
        ("swap", b"abc", b"abd"),
        ("long", b"abcdef", b"abc"),
        ("fail_me", b"abc", b"abc"),
        ("slow", b"abc", b"abc"),
    ];
    std::fs::create_dir_all(dir.join("nss")).unwrap();
    std::fs::create_dir_all(dir.join("ncs")).unwrap();
    std::fs::create_dir_all(dir.join("oracle")).unwrap();
    let mut manifest = String::from(
        "game = \"k1\"\nsource = \"test\"\n[counts]\nnss = 5\nncs = 5\npairs = 5\nmodule_ncs = 0\n",
    );
    for (name, nss, ncs) in scripts {
        std::fs::write(dir.join(format!("nss/{name}.nss")), nss).unwrap();
        std::fs::write(dir.join(format!("ncs/{name}.ncs")), ncs).unwrap();
        manifest.push_str(&format!(
            "[scripts.{name}]\nnss = \"{}\"\nncs = \"{}\"\n",
            hex(nss),
            hex(ncs)
        ));
    }
    std::fs::write(dir.join("k1.toml"), manifest).unwrap();
    std::fs::write(
        dir.join("compilers.toml"),
        r#"[compiler.fake]
description = "copies its input, fails on fail_me, hangs on slow"
command = ["sh", "-c", "case {name} in fail_me) echo 'x.nss(1): Error: nope' >&2; exit 1;; slow) sleep 5;; esac; cp {in} {out}"]
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("known.toml"),
        "[cause.broken]\nsummary = \"broken on purpose\"\nscripts = [\"fail_me\"]\n",
    )
    .unwrap();
}

fn run(dir: &Path, extra: &[&str]) -> std::process::Output {
    let mut c = Command::cargo_bin("nws").unwrap();
    c.current_dir(dir).args([
        "--format",
        "json",
        "conformance",
        "--corpus",
        ".",
        "--manifest",
        "k1.toml",
        "--compilers",
        "compilers.toml",
        "--known",
        "known.toml",
        "--compiler",
        "fake",
        "--timeout",
        "1",
    ]);
    c.args(extra).output().unwrap()
}

#[test]
fn classifies_attributes_and_is_deterministic() {
    let dir = std::env::temp_dir().join(format!("nws-conf-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    corpus(&dir);
    let out = run(&dir, &["--report", "a.json"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let c = &doc["result"]["compilers"][0];
    assert_eq!(c["counts"]["identical"], 1);
    assert_eq!(c["counts"]["samelen"], 1);
    assert_eq!(c["counts"]["difflen"], 1);
    assert_eq!(c["counts"]["failed"], 2);
    assert_eq!(c["scripts"]["long"]["delta"], 3);
    assert_eq!(c["scripts"]["fail_me"]["detail"], "x.nss(1): Error: nope");
    assert_eq!(c["scripts"]["fail_me"]["cause"], "broken");
    assert_eq!(c["scripts"]["slow"]["detail"], "timed out after 1s");

    run(&dir, &["--report", "b.json"]);
    assert_eq!(
        std::fs::read(dir.join("a.json")).unwrap(),
        std::fs::read(dir.join("b.json")).unwrap()
    );

    // A baseline in which `swap` was identical makes this run a regression.
    let mut base: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("a.json")).unwrap()).unwrap();
    base["compilers"][0]["scripts"]["swap"]["class"] = "identical".into();
    std::fs::write(dir.join("base.json"), base.to_string()).unwrap();
    assert_eq!(
        run(&dir, &["--baseline", "base.json"]).status.code(),
        Some(3)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
