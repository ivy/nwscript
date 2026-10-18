//! A hand-made install with texture packs: three quality packs and the GUI
//! pack under `TexturePacks/`, a `patch.erf`, a BIF and an `Override/`. It
//! holds the texture layer to its place in the order: Override, then
//! `patch.erf`, then the selected quality pack, then `swpc_tex_gui.erf`,
//! then the BIFs; and the unselected quality packs to not being searched.

use std::fs;
use std::path::{Path, PathBuf};

use rakata_archive::{Bif, BifMember, BifRef, Erf, ErfKind, Key, KeyEntry};
use rakata_game::{Install, Layer, Location, Profile, TexturePack, Verdict, codes};
use rakata_res::{ContentHash, ResId, ResType};

fn id(text: &str) -> ResId {
    ResId::parse(text).unwrap()
}

fn erf(path: &Path, members: &[(&str, &str)]) {
    let mut erf = Erf::new(ErfKind::Erf);
    for (name, bytes) in members {
        erf.add(id(name), bytes.as_bytes().to_vec()).unwrap();
    }
    fs::write(path, erf.to_bytes().unwrap()).unwrap();
}

/// The install, written fresh under the temp directory; `packs` names the
/// quality packs to write.
fn make(name: &str, packs: &[TexturePack]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("rakata-game-tex-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for sub in ["Override", "data", "TexturePacks"] {
        fs::create_dir_all(root.join(sub)).unwrap();
    }
    fs::write(root.join("Override/skin.tpc"), b"skin from Override").unwrap();

    for pack in packs {
        let n = pack.name();
        let wall = format!("wall from {n}");
        let skin = format!("skin from {n}");
        let head = format!("head from {n}");
        let only = format!("only in {n}");
        let only_name = format!("only_{n}.tpc");
        erf(
            &root.join("TexturePacks").join(pack.file_name()),
            &[
                ("wall01.tpc", &wall),
                ("skin.tpc", &skin),
                ("pfha01.tpc", &head),
                ("inbif.tpc", "inbif from a quality pack"),
                (&only_name, &only),
            ],
        );
    }
    erf(
        &root.join("TexturePacks/swpc_tex_gui.erf"),
        &[
            ("fnt_d16x16.tpc", "font from gui"),
            ("lbl_menudarr.txi", "mipmap 0"),
            ("wall01.tpc", "wall from gui"),
        ],
    );
    erf(
        &root.join("patch.erf"),
        &[("pfha01.tpc", "head from patch")],
    );

    let mut bif = Bif::new();
    bif.push(BifMember::owned(
        0,
        ResType::Tpc,
        b"inbif from the BIF".to_vec(),
    ));
    let bif_bytes = bif.to_bytes().unwrap();
    fs::write(root.join("data/textures.bif"), &bif_bytes).unwrap();
    let mut key = Key::new();
    key.add_bif(BifRef {
        size: bif_bytes.len() as u32,
        name: b"data\\textures.bif".to_vec(),
        drives: 1,
    });
    key.add(KeyEntry {
        id: id("inbif.tpc"),
        bif: 0,
        index: 0,
    })
    .unwrap();
    fs::write(root.join("chitin.key"), key.to_bytes().unwrap()).unwrap();
    root
}

fn file(c: &rakata_game::Candidate) -> String {
    c.location
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

#[test]
fn the_default_pack_is_tpa_over_the_gui_pack_and_the_others_are_not_searched() {
    let root = make("order", &TexturePack::ALL);
    let install = Install::open(&root, Profile::K1).unwrap();
    assert_eq!(install.texture_pack(), TexturePack::Tpa);
    let packs: Vec<&str> = install
        .texture_pack_paths()
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap())
        .collect();
    assert_eq!(packs, ["swpc_tex_tpa.erf", "swpc_tex_gui.erf"]);
    assert_eq!(install.missing_texture_pack(), None);

    let r = install.resolve(id("wall01.tpc"), None).unwrap();
    let files: Vec<String> = r.candidates.iter().map(file).collect();
    assert_eq!(files, ["swpc_tex_tpa.erf", "swpc_tex_gui.erf"]);
    let w = r.winner().unwrap();
    assert_eq!(w.layer, Layer::TexturePack);
    assert_eq!(w.hash, ContentHash::of(b"wall from tpa"));
    assert!(matches!(&w.location, Location::Capsule { member, .. } if *member == id("wall01.tpc")));
    assert_eq!(
        r.candidates[1].verdict,
        Verdict::Shadowed {
            by: 0,
            reason: "swpc_tex_tpa.erf outranks swpc_tex_gui.erf".into()
        }
    );

    let (location, bytes) = install.read(id("fnt_d16x16.tpc"), None).unwrap().unwrap();
    assert_eq!(bytes, b"font from gui");
    assert_eq!(location.short(), "swpc_tex_gui.erf:fnt_d16x16.tpc");
    let r = install.resolve(id("lbl_menudarr.txi"), None).unwrap();
    assert_eq!(file(r.winner().unwrap()), "swpc_tex_gui.erf");

    // A name only an unselected pack holds is not in the install's search.
    let r = install.resolve(id("only_tpb.tpc"), None).unwrap();
    assert_eq!(r.not_found().unwrap().code(), codes::NOT_FOUND);
    let ids = install.texture_pack_ids().unwrap();
    assert!(ids.contains(&id("only_tpa.tpc")));
    assert!(ids.contains(&id("fnt_d16x16.tpc")));
    assert!(!ids.contains(&id("only_tpb.tpc")));
    assert_eq!(ids.iter().filter(|i| **i == id("wall01.tpc")).count(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn override_and_patch_erf_outrank_the_packs_which_outrank_the_bifs() {
    let root = make("layers", &TexturePack::ALL);
    let install = Install::open(&root, Profile::K1).unwrap();
    let reasons = |name: &str| -> Vec<(Layer, String)> {
        let r = install.resolve(id(name), None).unwrap();
        r.candidates
            .iter()
            .map(|c| {
                let why = match &c.verdict {
                    Verdict::Wins => "wins".to_owned(),
                    Verdict::Shadowed { reason, .. } => reason.clone(),
                    Verdict::Ambiguous => "ambiguous".to_owned(),
                };
                (c.layer, why)
            })
            .collect()
    };
    assert_eq!(
        reasons("skin.tpc"),
        [
            (Layer::Override, "wins".into()),
            (
                Layer::TexturePack,
                "Override outranks the texture packs".into()
            )
        ]
    );
    assert_eq!(
        reasons("pfha01.tpc"),
        [
            (Layer::Patch, "wins".into()),
            (
                Layer::TexturePack,
                "patch.erf outranks the texture packs".into()
            )
        ]
    );
    assert_eq!(
        reasons("inbif.tpc"),
        [
            (Layer::TexturePack, "wins".into()),
            (Layer::Chitin, "the texture packs outrank the BIFs".into())
        ]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn another_pack_is_selected_by_name() {
    let root = make("select", &TexturePack::ALL);
    let pack: TexturePack = "tpc".parse().unwrap();
    let install = Install::open(&root, Profile::K1)
        .unwrap()
        .with_texture_pack(pack);
    assert_eq!(install.texture_pack(), TexturePack::Tpc);
    let r = install.resolve(id("wall01.tpc"), None).unwrap();
    assert_eq!(r.winner().unwrap().hash, ContentHash::of(b"wall from tpc"));
    assert!(
        install
            .resolve(id("only_tpc.tpc"), None)
            .unwrap()
            .winner()
            .is_some()
    );
    assert!(
        install
            .resolve(id("only_tpa.tpc"), None)
            .unwrap()
            .winner()
            .is_none()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_missing_selected_pack_leaves_the_gui_pack_and_is_reported() {
    let root = make("missing", &[TexturePack::Tpa]);
    let install = Install::open(&root, Profile::K1)
        .unwrap()
        .with_texture_pack(TexturePack::Tpb);
    assert_eq!(
        install.missing_texture_pack(),
        Some(root.join("TexturePacks/swpc_tex_tpb.erf"))
    );
    let r = install.resolve(id("wall01.tpc"), None).unwrap();
    assert_eq!(file(r.winner().unwrap()), "swpc_tex_gui.erf");
    assert_eq!(codes::MISSING_TEXTURE_PACK, "RW0402");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_install_without_texture_packs_has_no_texture_layer() {
    let root = make("none", &[]);
    fs::remove_dir_all(root.join("TexturePacks")).unwrap();
    let install = Install::open(&root, Profile::K1).unwrap();
    assert!(install.texture_pack_paths().is_empty());
    assert_eq!(install.missing_texture_pack(), None);
    assert!(install.texture_pack_ids().unwrap().is_empty());
    let r = install.resolve(id("inbif.tpc"), None).unwrap();
    assert_eq!(r.winner().unwrap().layer, Layer::Chitin);
    fs::remove_dir_all(root).unwrap();
}
