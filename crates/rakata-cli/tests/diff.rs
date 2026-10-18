//! `rakata diff` over fixtures built here, and the acceptance checks over
//! the user's own trees: a checkpoint against its parent when
//! `$RAKATA_CKPT` names one, and a `.mod` of `$RAKATA_BUILD` against a copy
//! with one member changed. Each skips, naming its variable, when it is
//! unset. No game bytes are committed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use rakata::gff::{Gff, Label, Value};
use rakata::res::ResId;
use rakata_archive::{Erf, ErfKind};
use serde_json::Value as Json;
use tempfile::TempDir;

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn utc(tag: &str) -> Vec<u8> {
    let mut g = Gff::new(*b"UTC ");
    g.root
        .push(Label::new("Tag").unwrap(), Value::string(tag))
        .push(Label::new("Plot").unwrap(), Value::Byte(0));
    g.to_bytes().unwrap()
}

fn json(cmd: &mut Command) -> (Option<i32>, Json) {
    let out = cmd.output().unwrap();
    let doc: Json = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.code(), doc)
}

#[test]
fn two_mods_differ_by_member_and_field_and_exit_3() {
    let dir = TempDir::new().unwrap();
    let write = |name: &str, tag: &str| {
        let mut m = Erf::new(ErfKind::Mod);
        m.add(ResId::parse("juhani.utc").unwrap(), utc(tag))
            .unwrap();
        m.add(ResId::parse("k_s.ncs").unwrap(), b"NCS V1.0".to_vec())
            .unwrap();
        let p = dir.path().join(name);
        std::fs::write(&p, m.to_bytes().unwrap()).unwrap();
        p
    };
    let (a, b) = (write("a.mod", "one"), write("b.mod", "two"));
    rakata()
        .args(["diff", s(&a), s(&a)])
        .assert()
        .code(0)
        .stdout(predicates::str::starts_with("no differences"));
    let out = rakata().args(["diff", s(&a), s(&b)]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("!juhani.utc (gff)"), "{text}");
    assert!(text.contains("    Tag: \"one\" -> \"two\""), "{text}");

    let (code, doc) = json(rakata().args(["--format", "json", "diff", s(&a), s(&b)]));
    assert_eq!(code, Some(3));
    let r = &doc["result"];
    assert_eq!(r["same"], false);
    assert!(r["seconds"].is_number());
    let e = &r["entries"][0];
    assert!(e["path"].as_str().unwrap().ends_with("!juhani.utc"));
    assert_eq!(e["change"], "changed");
    assert_eq!(e["sites"][0]["site"], "field");
    assert_eq!(e["sites"][0]["path"], "Tag");
    assert_eq!(e["sites"][0]["after"]["type"], "CExoString");
}

#[test]
fn a_directory_against_a_file_is_re1601() {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("x.utc");
    std::fs::write(&f, utc("x")).unwrap();
    rakata()
        .args(["diff", s(dir.path()), s(&f)])
        .assert()
        .code(1)
        .stderr(predicates::str::contains("RE1601"));
}

fn var(name: &str, what: &str) -> Option<PathBuf> {
    match std::env::var_os(name) {
        Some(v) => Some(PathBuf::from(v)),
        None => {
            eprintln!("skipped: ${name} is not set; it names {what}");
            None
        }
    }
}

/// The paths a checkpoint's `changed.txt` names, with their marks.
fn changed(ckpt: &Path) -> BTreeSet<(char, String)> {
    std::fs::read_to_string(ckpt.join("changed.txt"))
        .unwrap()
        .lines()
        .filter_map(|l| {
            let (mark, path) = l.split_once(' ')?;
            Some((mark.chars().next()?, path.to_owned()))
        })
        .collect()
}

/// The checkpoint `$RAKATA_CKPT` names (a folder holding `tree/`,
/// `changed.txt` and `meta.json`) against its parent, the sibling folder
/// its `meta.json` names: the diff lists exactly the paths `changed.txt`
/// names, with the same marks, and for the Juhani clones entry the mod's
/// own changes by node and row.
#[test]
fn a_checkpoint_against_its_parent_lists_exactly_what_changed() {
    let Some(ckpt) = var(
        "RAKATA_CKPT",
        "a build checkpoint: a folder holding tree/, changed.txt and meta.json",
    ) else {
        return;
    };
    let meta: Json =
        serde_json::from_str(&std::fs::read_to_string(ckpt.join("meta.json")).unwrap()).unwrap();
    let parent = ckpt
        .parent()
        .unwrap()
        .join(meta["parent"].as_str().unwrap());
    let (code, doc) = json(rakata().args([
        "--format",
        "json",
        "diff",
        s(&parent.join("tree")),
        s(&ckpt.join("tree")),
    ]));
    let r = &doc["result"];
    eprintln!(
        "{} against its parent: {} entries, {} pairs, {:.2} s",
        meta["mod"],
        r["entries"].as_array().unwrap().len(),
        r["compared"],
        r["seconds"].as_f64().unwrap()
    );
    let entries = r["entries"].as_array().unwrap();
    let found: BTreeSet<(char, String)> = entries
        .iter()
        .map(|e| {
            let path = e["path"].as_str().unwrap();
            let file = path.split('!').next().unwrap().to_owned();
            let mark = match (e["change"].as_str().unwrap(), path.contains('!')) {
                (_, true) | ("changed", _) => '~',
                ("added", _) => '+',
                ("removed", _) => '-',
                (c, _) => panic!("{c}"),
            };
            (mark, file)
        })
        .collect();
    let want = changed(&ckpt);
    assert_eq!(code, Some(if want.is_empty() { 0 } else { 3 }));
    assert_eq!(found, want, "the diff and changed.txt disagree");
    assert!(
        doc["diagnostics"].as_array().unwrap().is_empty(),
        "{}",
        doc["diagnostics"]
    );

    if meta["mod"] != "mod:juhani-clones" {
        return;
    }
    let entry = |p: &str| {
        entries
            .iter()
            .find(|e| e["path"] == p)
            .unwrap_or_else(|| panic!("no entry {p}"))
    };
    let sites = |p: &str, kind: &str| -> Vec<Json> {
        entry(p)["sites"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["site"] == kind)
            .cloned()
            .collect()
    };
    for dlg in [
        "dan14_cutjuh",
        "dan14_juhani",
        "k_hcar_dialog",
        "k_hmis_dialog",
    ] {
        let nodes = sites(&format!("Override/{dlg}.dlg"), "node");
        assert!(
            nodes.iter().any(|n| n["change"] == "added"),
            "{dlg}.dlg: no node added"
        );
        assert!(
            nodes.iter().all(|n| n["change"] != "removed"),
            "{dlg}.dlg: a node removed"
        );
    }
    let rows = sites("Override/globalcat.2da", "row");
    assert!(!rows.is_empty());
    for row in &rows {
        assert_eq!(row["change"], "added");
        let name = &row["cells"][0]["value"];
        assert!(name.as_str().unwrap().starts_with("OAJ_"), "{row}");
    }
    let app = sites("Override/appearance.2da", "row");
    assert_eq!(app.len(), 1);
    assert_eq!(app[0]["cells"][0]["value"], "Juhani_Clones_Hidden");
    assert_eq!(
        entry("Override/appearance.2da")["sites"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

/// `modules/danm14ac.mod` of `$RAKATA_BUILD` against a copy with Juhani's
/// blueprint retagged: the one member, by field.
#[test]
fn the_build_module_with_one_member_replaced() {
    let Some(build) = var("RAKATA_BUILD", "a modded KotOR 1 build") else {
        return;
    };
    let path = build.join("modules/danm14ac.mod");
    let bytes = std::fs::read(&path).unwrap();
    let mut erf = Erf::parse(&bytes).unwrap();
    let id = ResId::parse("dan14_juhani.utc").unwrap();
    let mut g = Gff::parse(erf.get(id).unwrap()).unwrap();
    g.root
        .set(Label::new("Tag").unwrap(), Value::string("dan14_juhani_x"));
    erf.replace(id, g.to_bytes().unwrap()).unwrap();
    let dir = TempDir::new().unwrap();
    let copy = dir.path().join("danm14ac.mod");
    std::fs::write(&copy, erf.to_bytes().unwrap()).unwrap();
    let out = rakata()
        .args(["diff", s(&path), s(&copy)])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    eprintln!("{text}");
    assert_eq!(out.status.code(), Some(3), "{text}");
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines[0].starts_with("~ ") && lines[0].contains("!dan14_juhani.utc (gff)"),
        "{text}"
    );
    assert_eq!(lines[1], "    Tag: \"dan14_juhani\" -> \"dan14_juhani_x\"");
    assert!(lines[2].starts_with("1 paths differ"), "{text}");
}
