//! Over the user's own installs: every vanilla DLG read through the model
//! and written back byte for byte, and a voice key resolved in the modded
//! build's copy of Juhani's cutscene.
//!
//! Reads `$RAKATA_GAME_K1` (a vanilla KotOR 1 install, the folder holding
//! `chitin.key`) and `$RAKATA_BUILD` (a modded build in the same layout),
//! and skips, naming the variable, when one is unset. No game bytes are
//! committed.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rakata_archive::{Bif, Erf, Key, Rim, Source};
use rakata_dialog::{Dialog, Kind, NodeKey};
use rakata_game::{Install, Profile};
use rakata_res::{ResId, ResRef, ResType};

fn tree(var: &str) -> Option<PathBuf> {
    match std::env::var(var) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${var} is unset; set it to a KotOR 1 install");
            None
        }
    }
}

fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

fn files_with(dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
        })
        .collect();
    files.sort();
    files
}

#[derive(Default)]
struct Census {
    read: usize,
    identical: usize,
    nodes: usize,
    different: Vec<String>,
}

impl Census {
    fn member(&mut self, at: &str, ty: ResType, data: &[u8]) {
        if ty != ResType::Dlg {
            return;
        }
        self.read += 1;
        let d = match Dialog::parse(at, data) {
            Ok(d) => d,
            Err(e) => {
                self.different.push(format!("{at}: {e}"));
                return;
            }
        };
        self.nodes += d.len(Kind::Entry) + d.len(Kind::Reply);
        match d.to_bytes() {
            Ok(out) if out == data => self.identical += 1,
            Ok(out) => {
                self.different
                    .push(format!("{at}: {} read, {} written", data.len(), out.len()))
            }
            Err(e) => self.different.push(format!("{at}: {e}")),
        }
    }
}

#[test]
fn every_vanilla_dlg_writes_back_identical_through_the_model() {
    let Some(root) = tree("RAKATA_GAME_K1") else {
        return;
    };
    let start = Instant::now();
    let mut census = Census::default();
    for dir in ["modules", "rims"] {
        let Some(d) = child_ci(&root, dir) else {
            continue;
        };
        for p in files_with(&d, &["rim", "mod", "erf"]) {
            let src = Source::read(&p).unwrap();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            if src.bytes().starts_with(b"RIM ") {
                for m in Rim::parse(&src).unwrap().members() {
                    census.member(&format!("{name}:{}", m.id), m.id.ty, m.data());
                }
            } else {
                for m in Erf::parse(&src).unwrap().members() {
                    census.member(&format!("{name}:{}", m.id), m.id.ty, m.data());
                }
            }
        }
    }
    if let Some(o) = child_ci(&root, "Override") {
        for p in files_with(&o, &["dlg"]) {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            census.member(&name, ResType::Dlg, &fs::read(&p).unwrap());
        }
    }
    let key_path = child_ci(&root, "chitin.key").expect("an install has chitin.key");
    let key = Key::parse(&fs::read(&key_path).unwrap()).unwrap();
    for b in key.bifs() {
        let mut p = root.clone();
        for part in b.components() {
            p = match child_ci(&p, &part) {
                Some(n) => n,
                None => break,
            };
        }
        let Ok(src) = Source::map(&p) else { continue };
        let bif = Bif::parse(&src).unwrap();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        for (i, m) in bif.members().iter().enumerate() {
            census.member(&format!("{name}[{i}]"), m.ty, m.data());
        }
    }
    println!(
        "{} DLG files, {} nodes, {} written back identical, in {:.2} s",
        census.read,
        census.nodes,
        census.identical,
        start.elapsed().as_secs_f64()
    );
    for d in census.different.iter().take(40) {
        println!("  different: {d}");
    }
    assert!(
        census.read > 1000,
        "an install has more than a thousand DLGs"
    );
    assert_eq!(
        census.identical,
        census.read,
        "{} different",
        census.different.len()
    );
}

#[test]
fn a_voice_key_names_one_node_of_juhanis_cutscene_in_the_build() {
    let Some(root) = tree("RAKATA_BUILD") else {
        return;
    };
    let install = Install::open(&root, Profile::K1).unwrap();
    let id = ResId::parse("dan14_cutjuh.dlg").unwrap();
    let module = rakata_game::ModuleName::new("danm14ac");
    let (_, bytes) = install
        .read(id, Some(&module))
        .unwrap()
        .expect("the build holds dan14_cutjuh.dlg");
    let d = Dialog::parse("dan14_cutjuh.dlg", &bytes).unwrap();
    let key = NodeKey::Voice(ResRef::new("nm14acjuha07024_").unwrap());
    let at = key.resolve(&d).unwrap();
    let node = d.node(at).unwrap();
    println!("{key} is {at}: {:?}", node.speaker());
    assert_eq!(node.voice(), Some(ResRef::new("NM14ACJUHA07024_").unwrap()));

    let missing = NodeKey::Voice(ResRef::new("oaj_missing").unwrap());
    let e = missing.resolve(&d).unwrap_err();
    assert_eq!(e.code(), "RE1102");
    assert!(e.to_string().contains("dan14_cutjuh.dlg"), "{e}");
}
