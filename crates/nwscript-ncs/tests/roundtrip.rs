//! Decode and re-encode every NCS file in the corpus.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nwscript_diag::FileId;
use nwscript_ncs::{decode, encode};

fn corpus() -> Option<PathBuf> {
    let dir = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".cache/nwscript/corpus/k1"),
    };
    dir.join("ncs").is_dir().then_some(dir)
}

fn ncs_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ncs")))
        .collect();
    files.sort();
    files
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = rd
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn roundtrip_corpus() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let mut groups: BTreeMap<&str, Vec<PathBuf>> = BTreeMap::new();
    groups.insert("ncs", ncs_files(&root.join("ncs")));
    for group in ["modules", "rims"] {
        let files = groups.entry(group).or_default();
        for d in subdirs(&root.join(group)) {
            files.extend(ncs_files(&d));
        }
    }

    let mut failures = Vec::new();
    let mut forms: BTreeMap<String, usize> = BTreeMap::new();
    for (group, files) in &groups {
        let mut same = 0;
        for path in files {
            let bytes = std::fs::read(path).unwrap();
            match decode(FileId(0), &bytes) {
                Ok(program) => {
                    for i in program.instructions() {
                        let key = format!(
                            "{:02X} {:02X} {}",
                            i.opcode as u8,
                            i.ty.0,
                            i.opcode.mnemonic(i.ty)
                        );
                        *forms.entry(key).or_default() += 1;
                    }
                    if encode(&program) == bytes {
                        same += 1;
                    } else {
                        failures.push(format!("{}: re-encoded bytes differ", path.display()));
                    }
                }
                Err(d) => failures.push(format!("{}: {}", path.display(), d.message)),
            }
        }
        println!("{group}: {same}/{} byte-identical", files.len());
    }
    println!("instruction forms (opcode type mnemonic: count):");
    for (form, n) in &forms {
        println!("  {form}: {n}");
    }
    assert!(
        !groups["ncs"].is_empty(),
        "no NCS files under {}/ncs",
        root.display()
    );
    assert!(
        failures.is_empty(),
        "{} files failed:\n{}",
        failures.len(),
        failures[..failures.len().min(50)].join("\n")
    );
}

#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn roundtrip_corpus_through_assembly() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let mut files = ncs_files(&root.join("ncs"));
    for group in ["modules", "rims"] {
        for d in subdirs(&root.join(group)) {
            files.extend(ncs_files(&d));
        }
    }
    let mut failures = Vec::new();
    for path in &files {
        let bytes = std::fs::read(path).unwrap();
        let program = decode(FileId(0), &bytes).unwrap();
        for raw in [false, true] {
            let text = nwscript_ncs::asm::print(
                &program,
                &nwscript_ncs::asm::PrintOptions {
                    raw,
                    routine_name: None,
                },
            );
            match nwscript_ncs::asm::parse(FileId(0), text.as_bytes()) {
                Ok(back) if encode(&back) == bytes => {}
                Ok(_) => failures.push(format!("{} (raw: {raw}): different bytes", path.display())),
                Err(e) => failures.push(format!(
                    "{} (raw: {raw}): {:?}",
                    path.display(),
                    e[0].message
                )),
            }
        }
    }
    println!(
        "assembly: {}/{} byte-identical in both forms",
        files.len() - failures.len(),
        files.len()
    );
    assert!(failures.is_empty(), "{failures:#?}");
}
