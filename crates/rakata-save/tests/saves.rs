//! Over the user's own saves: every GFF member of every save round-trips
//! byte for byte through `rakata-gff`, and the readers find what the saves
//! hold.
//!
//! Reads the save directories under `$RAKATA_SAVES` and skips, naming the
//! variable, when it is unset. No save bytes are committed; the assertions
//! name only facts every save of the game carries.

use std::path::PathBuf;

use rakata_gff::Gff;
use rakata_save::{Place, Save};

const SAVES_VAR: &str = "RAKATA_SAVES";

fn saves() -> Vec<PathBuf> {
    let Ok(root) = std::env::var(SAVES_VAR) else {
        eprintln!("skipped: ${SAVES_VAR} is unset; set it to a folder of save-game directories");
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("${SAVES_VAR}={root}: {e}"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    assert!(!dirs.is_empty(), "${SAVES_VAR}={root} holds no directories");
    dirs
}

#[test]
fn every_gff_member_round_trips_byte_identical() {
    for dir in saves() {
        let save = Save::open(&dir).unwrap();
        let members = save.gff_members().unwrap();
        assert!(!members.is_empty());
        let mut differ = Vec::new();
        for m in &members {
            let gff = Gff::parse(&m.bytes).unwrap_or_else(|e| panic!("{}: {e}", m.location));
            let back = gff
                .to_bytes()
                .unwrap_or_else(|e| panic!("{}: {e}", m.location));
            if back != m.bytes {
                let at = back
                    .iter()
                    .zip(&m.bytes)
                    .position(|(a, b)| a != b)
                    .unwrap_or(back.len().min(m.bytes.len()));
                differ.push(format!(
                    "{}: first difference at {at} ({} bytes written, {} read)",
                    m.location,
                    back.len(),
                    m.bytes.len()
                ));
            }
        }
        eprintln!(
            "{}: {} GFF members, {} identical",
            dir.display(),
            members.len(),
            members.len() - differ.len()
        );
        assert!(differ.is_empty(), "{}", differ.join("\n"));
    }
}

#[test]
fn every_save_reads_by_name() {
    for dir in saves() {
        let save = Save::open(&dir).unwrap();
        let globals = save.globals().unwrap();
        assert!(!globals.vars.is_empty(), "{}: no globals", dir.display());
        let modules = save.modules().unwrap();
        assert!(!modules.is_empty());
        let info = save.info();
        let last = info.last_module.expect("savenfo names the last module");
        assert!(
            modules.iter().any(|m| m.eq_ignore_ascii_case(&last)),
            "the last module {last} is among {modules:?}"
        );
        let party = save.party().unwrap();
        for m in &party.members {
            assert!(m.tag.is_some(), "party member {} has an AVAILNPC", m.npc);
        }
        let all = save.creatures(None).unwrap();
        assert!(
            all.iter()
                .any(|c| matches!(c.place, Place::PartyMember { .. }))
        );
        assert!(all.iter().any(|c| matches!(c.place, Place::Module { .. })));
        // A party member is found by its tag in any case.
        let npc = all
            .iter()
            .find(|c| matches!(c.place, Place::PartyMember { .. }))
            .unwrap();
        let shouted = npc.tag.to_ascii_uppercase();
        let found = save.find_creatures(&shouted, Some(&last)).unwrap();
        assert!(found.iter().any(|c| c.place == npc.place));
        save.events(&last).unwrap();
    }
}
