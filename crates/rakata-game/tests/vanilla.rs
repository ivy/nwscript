//! The texture layer over the user's own vanilla KotOR 1 install: a texture
//! font resolves to `swpc_tex_gui.erf`, an Endar Spire room texture to the
//! selected quality pack, and a texture placed in `Override/` to Override.
//! Reads `$RAKATA_GAME_K1` (the folder holding `chitin.key`) and skips,
//! naming the variable, when it is unset. The Override case runs over a
//! mirror of the install whose `Override/` is the test's own; nothing is
//! written to the install, and no game bytes are committed.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use rakata_game::{Install, Layer, Location, ModuleName, Profile, TexturePack, Verdict, codes};
use rakata_res::{ContentHash, ResId};

const GAME_VAR: &str = "RAKATA_GAME_K1";

fn game() -> Option<PathBuf> {
    match std::env::var(GAME_VAR) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${GAME_VAR} is unset; set it to a vanilla KotOR 1 install");
            None
        }
    }
}

fn id(text: &str) -> ResId {
    ResId::parse(text).unwrap()
}

fn file(l: &Location) -> String {
    l.path().file_name().unwrap().to_string_lossy().into_owned()
}

#[test]
fn a_texture_font_resolves_to_the_gui_pack() {
    let Some(root) = game() else { return };
    let install = Install::open(&root, Profile::K1).unwrap();
    let r = install.resolve(id("fnt_d16x16.tpc"), None).unwrap();
    assert_eq!(r.candidates.len(), 1);
    let w = r.winner().unwrap();
    assert_eq!(w.layer, Layer::TexturePack);
    assert_eq!(file(&w.location), "swpc_tex_gui.erf");
    // The pack's one TXI member.
    let r = install.resolve(id("lbl_menudarr.txi"), None).unwrap();
    assert_eq!(file(&r.winner().unwrap().location), "swpc_tex_gui.erf");
    // The font's TXI is not a resource of its own: it is the text at the
    // end of the TPC, so no layer holds `fnt_d16x16.txi`.
    let r = install.resolve(id("fnt_d16x16.txi"), None).unwrap();
    assert_eq!(r.not_found().unwrap().code(), codes::NOT_FOUND);
}

#[test]
fn an_endar_spire_room_texture_resolves_to_the_selected_quality_pack() {
    let Some(root) = game() else { return };
    // `lhr_flr01` is a texture of `m01aa_01a`, the first room of
    // `end_m01aa`.
    let name = id("lhr_flr01.tpc");
    let mut hashes = Vec::new();
    for pack in TexturePack::ALL {
        let install = Install::open(&root, Profile::K1)
            .unwrap()
            .with_texture_pack(pack);
        assert_eq!(install.missing_texture_pack(), None);
        let r = install
            .resolve(name, Some(&ModuleName::new("end_m01aa")))
            .unwrap();
        assert_eq!(r.candidates.len(), 1, "{pack}: {:?}", r.candidates);
        let w = r.winner().unwrap();
        assert_eq!(w.layer, Layer::TexturePack);
        assert_eq!(file(&w.location), pack.file_name());
        hashes.push(w.hash);
    }
    assert_ne!(hashes[0], hashes[1], "tpa and tpb hold other bytes");
    assert_ne!(hashes[1], hashes[2], "tpb and tpc hold other bytes");
}

#[test]
fn patch_erf_outranks_the_quality_pack_for_the_textures_both_hold() {
    let Some(root) = game() else { return };
    let install = Install::open(&root, Profile::K1).unwrap();
    let r = install.resolve(id("pfha01.tpc"), None).unwrap();
    let layers: Vec<Layer> = r.candidates.iter().map(|c| c.layer).collect();
    assert_eq!(layers, [Layer::Patch, Layer::TexturePack]);
    assert_eq!(file(&r.candidates[1].location), "swpc_tex_tpa.erf");
}

/// A mirror of `game`: every entry linked except `Override/`, a folder of
/// the mirror's own.
fn mirror(game: &Path, root: &Path) {
    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root.join("Override")).unwrap();
    for e in fs::read_dir(game).unwrap() {
        let e = e.unwrap();
        let n = e.file_name();
        if !n.to_string_lossy().eq_ignore_ascii_case("override") {
            symlink(e.path(), root.join(&n)).unwrap();
        }
    }
}

#[test]
fn a_texture_in_override_outranks_the_packs() {
    let Some(game) = game() else { return };
    let root = std::env::temp_dir().join(format!("rakata-game-vanilla-{}", std::process::id()));
    mirror(&game, &root);
    fs::write(root.join("Override/LHR_flr01.tpc"), b"ours").unwrap();
    let install = Install::open(&root, Profile::K1).unwrap();
    let r = install.resolve(id("lhr_flr01.tpc"), None).unwrap();
    assert_eq!(r.candidates.len(), 2);
    let w = r.winner().unwrap();
    assert_eq!(w.layer, Layer::Override);
    assert_eq!(w.hash, ContentHash::of(b"ours"));
    assert_eq!(
        r.candidates[1].verdict,
        Verdict::Shadowed {
            by: 0,
            reason: "Override outranks the texture packs".into()
        }
    );
    assert_eq!(file(&r.candidates[1].location), "swpc_tex_tpa.erf");
    fs::remove_dir_all(root).unwrap();
}
