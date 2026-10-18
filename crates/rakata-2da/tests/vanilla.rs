//! Over the user's own KotOR 1 install: every member of `data/2da.bif` and
//! every table inside `rims/global.rim` and `rims/miniglobal.rim` reads and
//! writes back byte for byte, the RIMs' NUL-ended names and labels
//! included, and the counts are printed. Reads `$RAKATA_GAME_K1` (the folder
//! holding `chitin.key`) and skips, naming the variable, when it is unset.
//! No game bytes are committed.

use std::fs;
use std::path::{Path, PathBuf};

use rakata_2da::{Terminator, TwoDa};
use rakata_archive::{Bif, Rim};
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

/// A child of `dir` named `name` in any case, as the game finds it.
fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

/// Read and write one table, asserting the bytes come back.
fn round_trip(name: &str, bytes: &[u8]) -> TwoDa {
    let t = TwoDa::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
    let written = t.to_bytes().unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(written == bytes, "{name} did not write back identical");
    t
}

#[test]
fn every_vanilla_table_writes_back_byte_for_byte() {
    let Some(root) = game() else { return };
    let data = child_ci(&root, "data").expect("data/");
    let path = child_ci(&data, "2da.bif").expect("data/2da.bif");
    let bytes = fs::read(&path).unwrap();
    let bif = Bif::parse(&bytes).unwrap();
    let mut tables = 0usize;
    let mut non_index_labels = 0usize;
    for (i, m) in bif.members().iter().enumerate() {
        if m.ty != ResType::TwoDa {
            continue;
        }
        let t = round_trip(&format!("2da.bif member {i}"), m.data());
        assert_eq!(t.terminator, Terminator::Tab, "2da.bif member {i}");
        if t.rows
            .iter()
            .enumerate()
            .any(|(n, r)| r.label != n.to_string())
        {
            non_index_labels += 1;
        }
        tables += 1;
    }
    println!(
        "{tables} tables in data/2da.bif, every one identical after a round trip; {non_index_labels} with row labels that are not the row index"
    );
    assert!(tables > 0);
}

#[test]
fn the_tables_inside_the_global_rims_write_back_byte_for_byte() {
    let Some(root) = game() else { return };
    let rims = child_ci(&root, "rims").expect("rims/");
    for rim in ["global.rim", "miniglobal.rim"] {
        let path = child_ci(&rims, rim).unwrap_or_else(|| panic!("rims/{rim}"));
        let bytes = fs::read(&path).unwrap();
        let archive = Rim::parse(&bytes).unwrap();
        let (mut tab, mut nul) = (0usize, 0usize);
        for m in archive.members() {
            if m.id.ty != ResType::TwoDa {
                continue;
            }
            let t = round_trip(&format!("{rim}!{}", m.id), m.data());
            match t.terminator {
                Terminator::Tab => tab += 1,
                Terminator::Nul => nul += 1,
            }
        }
        println!(
            "{} tables in rims/{rim}, every one identical after a round trip: {tab} tab-ended, {nul} NUL-ended",
            tab + nul
        );
        assert!(nul > 0, "rims/{rim} holds NUL-ended tables in KotOR 1");
    }
}
