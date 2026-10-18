//! The census over a KotOR 1 install: every GFF in every RIM, MOD and BIF
//! read, written again and compared to its source, counted by file type.
//! `rakata conformance` runs the same round trip through the registry; this
//! test is the per-type count and the timing.
//!
//! Reads the user's own install from `$RAKATA_GAME_K1` (the folder holding
//! `chitin.key`) and skips, naming the variable, when it is unset. No game
//! bytes are committed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rakata_archive::{Bif, Erf, Key, Rim, Source};
use rakata_gff::Gff;
use rakata_res::ResType;

const GAME_VAR: &str = "RAKATA_GAME_K1";

fn game() -> Option<PathBuf> {
    match std::env::var(GAME_VAR) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${GAME_VAR} is unset; set it to a KotOR 1 install");
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
    by_type: BTreeMap<String, (usize, usize)>,
    different: Vec<String>,
    bytes: usize,
}

impl Census {
    fn member(&mut self, where_: &str, ty: ResType, data: &[u8]) {
        if data.len() < 8 || &data[4..8] != b"V3.2" {
            return;
        }
        let gff = match Gff::parse(data) {
            Ok(g) => g,
            Err(e) => {
                self.different.push(format!("{where_} ({ty}): {e}"));
                return;
            }
        };
        let slot = self.by_type.entry(gff.file_type_str()).or_default();
        slot.0 += 1;
        self.bytes += data.len();
        match gff.to_bytes() {
            Ok(out) if out == data => slot.1 += 1,
            Ok(out) => {
                let at = out
                    .iter()
                    .zip(data)
                    .position(|(a, b)| a != b)
                    .unwrap_or(out.len().min(data.len()));
                self.different.push(format!(
                    "{where_} ({ty}): {} read, {} written, first difference at {at}",
                    data.len(),
                    out.len()
                ));
            }
            Err(e) => self.different.push(format!("{where_} ({ty}): {e}")),
        }
    }
}

#[test]
fn every_vanilla_gff_writes_back_identical() {
    let Some(root) = game() else {
        return;
    };
    let start = Instant::now();
    let mut census = Census::default();
    for dir in ["modules", "rims"] {
        let Some(d) = child_ci(&root, dir) else {
            continue;
        };
        for p in files_with(&d, &["rim", "mod"]) {
            let src = Source::read(&p).unwrap();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            if src.bytes().starts_with(b"RIM ") {
                let rim = Rim::parse(&src).unwrap();
                for m in rim.members() {
                    census.member(&format!("{name}!{}", m.id), m.id.ty, m.data());
                }
            } else {
                let erf = Erf::parse(&src).unwrap();
                for m in erf.members() {
                    census.member(&format!("{name}!{}", m.id), m.id.ty, m.data());
                }
            }
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
            census.member(&format!("{name}!#{i}"), m.ty, m.data());
        }
    }
    let elapsed = start.elapsed();
    let total: usize = census.by_type.values().map(|v| v.0).sum();
    let identical: usize = census.by_type.values().map(|v| v.1).sum();
    println!(
        "{total} GFF files ({} MiB), {identical} identical, in {:.2} s",
        census.bytes >> 20,
        elapsed.as_secs_f64()
    );
    for (ty, (n, same)) in &census.by_type {
        println!("  {ty:<4} {n:>6} files, {same:>6} identical");
    }
    for d in census.different.iter().take(40) {
        println!("  different: {d}");
    }
    assert!(total > 10_000, "an install has more than ten thousand GFFs");
    assert_eq!(identical, total, "{} different", census.different.len());
}
