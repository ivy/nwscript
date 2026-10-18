//! Over the user's own KotOR 1 install: every member of `data/2da.bif` reads,
//! writes back byte for byte, and the count is printed. Reads
//! `$RAKATA_GAME_K1` (the folder holding `chitin.key`) and skips, naming the
//! variable, when it is unset. No game bytes are committed.

use std::fs;
use std::path::{Path, PathBuf};

use rakata_2da::TwoDa;
use rakata_archive::Bif;
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
        let t = TwoDa::parse(m.data()).unwrap_or_else(|e| panic!("member {i}: {e}"));
        let written = t.to_bytes().unwrap_or_else(|e| panic!("member {i}: {e}"));
        assert!(
            written == m.data(),
            "member {i} did not write back identical"
        );
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
