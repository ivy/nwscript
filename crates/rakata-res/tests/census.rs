//! The vanilla type census: every resource type id in a KotOR 1 install's
//! `chitin.key` and module archives maps to an extension.
//!
//! Reads the user's own install from `$RAKATA_GAME_K1` (the folder holding
//! `chitin.key`) and skips, naming the variable, when it is unset. No game
//! bytes are committed. The KEY, ERF and RIM header walkers below read only
//! the type column of each table; their layouts follow BioWare's KEY/BIF and
//! ERF format documents and xoreos's RIM documentation.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rakata_res::ResType;

const GAME_VAR: &str = "RAKATA_GAME_K1";

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// The type ids in a KEY file's key table: one entry of 22 bytes per
/// resource, the type at offset 16.
fn key_types(b: &[u8]) -> Vec<u16> {
    assert_eq!(&b[..8], b"KEY V1  ", "not a KEY V1 file");
    let count = u32_at(b, 12) as usize;
    let table = u32_at(b, 20) as usize;
    (0..count).map(|i| u16_at(b, table + i * 22 + 16)).collect()
}

/// The type ids in a RIM's resource table: one entry of 32 bytes per
/// resource, the type (four bytes wide) at offset 16.
fn rim_types(b: &[u8]) -> Vec<u16> {
    assert_eq!(&b[..8], b"RIM V1.0", "not a RIM V1.0 file");
    let count = u32_at(b, 12) as usize;
    let table = u32_at(b, 16) as usize;
    (0..count)
        .map(|i| u16::try_from(u32_at(b, table + i * 32 + 16)).expect("type id fits u16"))
        .collect()
}

/// The type ids in an ERF's key list: one entry of 24 bytes per resource,
/// the type at offset 20.
fn erf_types(b: &[u8]) -> Vec<u16> {
    assert!(
        matches!(&b[..8], b"ERF V1.0" | b"MOD V1.0"),
        "not an ERF/MOD V1.0 file"
    );
    let count = u32_at(b, 16) as usize;
    let keys = u32_at(b, 24) as usize;
    (0..count).map(|i| u16_at(b, keys + i * 24 + 20)).collect()
}

/// The files directly under `dir` whose extension is `ext`, sorted.
fn files_with(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)))
        .collect();
    files.sort();
    files
}

#[test]
fn every_vanilla_type_id_maps_to_an_extension() {
    let Ok(root) = std::env::var(GAME_VAR) else {
        eprintln!("skipped: ${GAME_VAR} is unset; set it to a KotOR 1 install");
        return;
    };
    let root = PathBuf::from(root);

    let mut counts: BTreeMap<u16, usize> = BTreeMap::new();
    let mut archives = 0usize;
    let mut tally = |ids: Vec<u16>| {
        archives += 1;
        for id in ids {
            *counts.entry(id).or_default() += 1;
        }
    };

    let key = fs::read(root.join("chitin.key")).expect("read chitin.key");
    tally(key_types(&key));
    for path in files_with(&root.join("modules"), "rim")
        .into_iter()
        .chain(files_with(&root.join("rims"), "rim"))
    {
        tally(rim_types(&fs::read(&path).unwrap()));
    }
    for path in files_with(&root.join("modules"), "mod") {
        tally(erf_types(&fs::read(&path).unwrap()));
    }
    assert!(archives > 1, "no module archives under {}", root.display());

    println!("{} archives, {} distinct type ids:", archives, counts.len());
    let mut unknown = Vec::new();
    for (&id, &n) in &counts {
        let ty = ResType::from_id(id);
        match ty.extension() {
            Some(ext) => println!("  {id:>5}  {ext:<4}  {n}"),
            None => {
                println!("  {id:>5}  ????  {n}");
                unknown.push(id);
            }
        }
    }
    assert!(
        unknown.is_empty(),
        "type ids without an extension: {unknown:?}"
    );
}
