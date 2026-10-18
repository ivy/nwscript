//! The dialogue lints over the user's own installs: every vanilla dialogue
//! finds exactly the published list of the game's own faults
//! (`conformance/dialog-known.toml`), and the Juhani mod's five dialogues in
//! the modded build find nothing.
//!
//! Reads `$RAKATA_GAME_K1` (a vanilla KotOR 1 install) and `$RAKATA_BUILD`
//! (a modded build), and skips, naming the variable, when one is unset. No
//! game bytes are committed.

use std::path::{Path, PathBuf};
use std::time::Instant;

use rakata::dialog::lint::{Entry, Known, Linter};
use rakata::game::{ModuleName, Profile};

fn tree(var: &str) -> Option<PathBuf> {
    match std::env::var(var) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${var} is unset; set it to a KotOR 1 install");
            None
        }
    }
}

#[test]
fn the_vanilla_dialogues_find_exactly_the_known_list() {
    let Some(root) = tree("RAKATA_GAME_K1") else {
        return;
    };
    let known = Known::read(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/dialog-known.toml"),
    )
    .unwrap();
    let start = Instant::now();
    let mut linter = Linter::open(&root, Profile::K1).unwrap();
    let targets = linter.targets(None).unwrap();
    let mut found = Known::default();
    for t in &targets {
        let l = linter.lint_target(t).unwrap();
        for f in &l.findings {
            found.findings.insert(Entry::of(
                l.loaded.dialog.name(),
                l.loaded.module.as_deref(),
                f,
            ));
        }
    }
    eprintln!(
        "{} dialogues linted in {:.2} s",
        targets.len(),
        start.elapsed().as_secs_f64()
    );
    let new: Vec<_> = found.findings.difference(&known.findings).collect();
    let gone: Vec<_> = known.findings.difference(&found.findings).collect();
    assert!(
        new.is_empty() && gone.is_empty(),
        "not in the known list: {new:#?}\nknown, not found: {gone:#?}"
    );
}

#[test]
fn the_juhani_mods_dialogues_find_nothing() {
    let Some(root) = tree("RAKATA_BUILD") else {
        return;
    };
    let mut linter = Linter::open(&root, Profile::K1).unwrap();
    for name in [
        "dan14_cutjuh.dlg",
        "dan14_juhani.dlg",
        "k_hcar_dialog.dlg",
        "k_hmis_dialog.dlg",
        "oaj_banter.dlg",
    ] {
        let l = linter
            .lint_named(name, Some(ModuleName::new("danm14ac")))
            .unwrap();
        let messages: Vec<String> = l
            .findings
            .iter()
            .map(|f| format!("{}: {}", f.code, f.message(name)))
            .collect();
        assert!(messages.is_empty(), "{messages:#?}");
    }
}
