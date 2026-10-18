//! Over the user's own modded build: the copy of `feat.2da` the game loads,
//! found through `rakata-game`, has its `label` cells intact. This is the
//! table a size-trusting reader once blanked (modshop's
//! `games/kotor/patches/README.md`, item 3). BioWare's own table carries
//! three placeholder rows with an empty label (rows 0, 23 and 59 in KotOR
//! 1, measured), so the check is that no other row's label is empty and,
//! when `$RAKATA_GAME_K1` is set too, that the loaded vanilla table has the
//! same blank-label rows. Reads `$RAKATA_BUILD` (the folder holding
//! `chitin.key`) and skips, naming the variable, when it is unset. No game
//! bytes are committed.

use std::path::PathBuf;

use rakata_2da::TwoDa;
use rakata_game::{Install, Profile};
use rakata_res::ResId;

const BUILD_VAR: &str = "RAKATA_BUILD";
const GAME_VAR: &str = "RAKATA_GAME_K1";

/// The labels of the rows of KotOR 1's `feat.2da` whose `label` cell is
/// empty: BioWare's placeholders.
const PLACEHOLDERS: [&str; 3] = ["0", "23", "59"];

fn build() -> Option<PathBuf> {
    match std::env::var(BUILD_VAR) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${BUILD_VAR} is unset; set it to a modded KotOR 1 build");
            None
        }
    }
}

/// The loaded `feat.2da` of the install at `root`, and where it came from.
fn loaded_feat(root: &PathBuf) -> (String, TwoDa) {
    let install = Install::open(root, Profile::K1).unwrap();
    let id: ResId = "feat.2da".parse().unwrap();
    let (location, bytes) = install
        .read(id, None)
        .unwrap()
        .expect("the install holds feat.2da");
    (location.short(), TwoDa::parse(&bytes).unwrap())
}

/// The labels of the rows whose `label` cell is empty.
fn blank_label_rows(t: &TwoDa) -> Vec<&str> {
    t.rows
        .iter()
        .filter(|r| t.get(r.label.as_str(), "label").unwrap().is_empty())
        .map(|r| r.label.as_str())
        .collect()
}

#[test]
fn the_loaded_feat_table_keeps_its_labels() {
    let Some(root) = build() else { return };
    let (from, t) = loaded_feat(&root);
    assert!(
        t.column_index("label").is_some(),
        "feat.2da has a `label` column"
    );
    let blank = blank_label_rows(&t);
    let labelled = t.rows.len() - blank.len();
    println!(
        "feat.2da from {from}: {} rows, {labelled} with a label, {} placeholder rows {blank:?}",
        t.rows.len(),
        blank.len()
    );
    assert!(
        labelled > 0,
        "every label is empty: the cells were not read"
    );
    for label in &blank {
        assert!(
            PLACEHOLDERS.contains(label),
            "row `{label}` has an empty label and is not one of BioWare's placeholder rows {PLACEHOLDERS:?}"
        );
    }
    if let Ok(vanilla) = std::env::var(GAME_VAR) {
        let (_, v) = loaded_feat(&PathBuf::from(vanilla));
        assert_eq!(
            blank,
            blank_label_rows(&v),
            "the build's blank-label rows are vanilla's placeholders"
        );
    } else {
        eprintln!("${GAME_VAR} is unset; the placeholder rows were not compared with vanilla's");
    }
}
