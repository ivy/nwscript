//! A hand-made install: two loose files, two module capsules, one BIF. It
//! holds the install to the engine's order: Override over the module, the
//! module's `.mod` over its `.rim`, and the BIFs last; and to reporting two
//! Override files that differ only in case rather than choosing between
//! them.

use std::fs;
use std::path::PathBuf;

use nwscript::Resolver;
use rakata_archive::{Bif, BifMember, BifRef, Erf, ErfKind, Key, KeyEntry, Rim};
use rakata_game::{Install, Layer, Location, ModuleName, Profile, Verdict, codes};
use rakata_res::{ContentHash, ResId, ResType};

fn id(text: &str) -> ResId {
    ResId::parse(text).unwrap()
}

/// The install, written fresh under the temp directory.
fn make(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("rakata-game-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for sub in ["Override", "modules", "data"] {
        fs::create_dir_all(root.join(sub)).unwrap();
    }

    fs::write(root.join("Override/hello.utc"), b"hello from Override").unwrap();
    fs::write(root.join("Override/PLC_Invis.utp"), b"capital").unwrap();
    fs::write(root.join("Override/plc_invis.utp"), b"lower").unwrap();
    fs::write(root.join("Override/k_inc_x.nss"), b"int OVERRIDE;").unwrap();

    let mut dot_mod = Erf::new(ErfKind::Mod);
    dot_mod
        .add(id("k_script.ncs"), b"ncs from .mod".to_vec())
        .unwrap();
    dot_mod
        .add(id("hello.utc"), b"hello from .mod".to_vec())
        .unwrap();
    fs::write(
        root.join("modules/TestMod.mod"),
        dot_mod.to_bytes().unwrap(),
    )
    .unwrap();

    let mut rim = Rim::new();
    rim.add(id("k_script.ncs"), b"ncs from .rim".to_vec())
        .unwrap();
    rim.add(id("area.are"), b"are from .rim".to_vec()).unwrap();
    fs::write(root.join("modules/testmod.rim"), rim.to_bytes().unwrap()).unwrap();

    let mut s_rim = Rim::new();
    s_rim
        .add(id("k_script.ncs"), b"ncs from _s.rim".to_vec())
        .unwrap();
    s_rim
        .add(id("hello.utc"), b"hello from _s.rim".to_vec())
        .unwrap();
    fs::write(
        root.join("modules/testmod_s.rim"),
        s_rim.to_bytes().unwrap(),
    )
    .unwrap();

    let mut bif = Bif::new();
    bif.push(BifMember::owned(
        0,
        ResType::TwoDa,
        b"2da from the BIF".to_vec(),
    ));
    bif.push(BifMember::owned(
        1,
        ResType::Utc,
        b"hello from the BIF".to_vec(),
    ));
    bif.push(BifMember::owned(2, ResType::Nss, b"int NWSCRIPT;".to_vec()));
    bif.push(BifMember::owned(3, ResType::Nss, b"int BIF;".to_vec()));
    let bif_bytes = bif.to_bytes().unwrap();
    fs::write(root.join("data/templates.bif"), &bif_bytes).unwrap();

    let mut key = Key::new();
    key.add_bif(BifRef {
        size: bif_bytes.len() as u32,
        name: b"data\\templates.bif".to_vec(),
        drives: 1,
    });
    for (i, name) in ["appearance.2da", "hello.utc", "nwscript.nss", "k_inc_x.nss"]
        .iter()
        .enumerate()
    {
        key.add(KeyEntry {
            id: id(name),
            bif: 0,
            index: i as u32,
        })
        .unwrap();
    }
    fs::write(root.join("chitin.key"), key.to_bytes().unwrap()).unwrap();
    root
}

#[test]
fn override_beats_the_module_which_beats_the_bif() {
    let root = make("order");
    let install = Install::open(&root, Profile::K1).unwrap();
    let module = ModuleName::new("TESTMOD");
    let r = install.resolve(id("hello.utc"), Some(&module)).unwrap();
    let layers: Vec<Layer> = r.candidates.iter().map(|c| c.layer).collect();
    assert_eq!(
        layers,
        [Layer::Override, Layer::Module, Layer::Module, Layer::Chitin]
    );
    let winner = r.winner().unwrap();
    assert_eq!(winner.hash, ContentHash::of(b"hello from Override"));
    assert_eq!(winner.len, 19);
    assert_eq!(winner.verdict, Verdict::Wins);
    assert!(!r.is_ambiguous());
    let reasons: Vec<&str> = r.candidates[1..]
        .iter()
        .map(|c| match &c.verdict {
            Verdict::Shadowed { by: 0, reason } => reason.as_str(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        reasons,
        [
            "Override outranks the module's capsules",
            "Override outranks the module's capsules",
            "Override outranks the BIFs"
        ]
    );
    assert!(matches!(
        &r.candidates[3].location,
        Location::Bif { index: 1, .. }
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn dot_mod_beats_dot_rim_beats_s_rim() {
    let root = make("capsules");
    let install = Install::open(&root, Profile::K1).unwrap();
    let module = ModuleName::new("testmod");
    assert_eq!(install.modules(), std::slice::from_ref(&module));
    let names: Vec<String> = install
        .module_capsules(&module)
        .unwrap()
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["TestMod.mod", "testmod.rim", "testmod_s.rim"]);

    let r = install.resolve(id("K_SCRIPT.NCS"), Some(&module)).unwrap();
    assert_eq!(r.candidates.len(), 3);
    assert_eq!(r.winner().unwrap().hash, ContentHash::of(b"ncs from .mod"));
    assert_eq!(
        r.candidates[1].verdict,
        Verdict::Shadowed {
            by: 0,
            reason: "TestMod.mod outranks testmod.rim".into()
        }
    );
    assert_eq!(
        r.candidates[2].verdict,
        Verdict::Shadowed {
            by: 0,
            reason: "TestMod.mod outranks testmod_s.rim".into()
        }
    );
    // Without the module, a module-only resource is not found.
    let r = install.resolve(id("k_script.ncs"), None).unwrap();
    assert!(r.winner().is_none());
    assert_eq!(r.not_found().unwrap().code(), codes::NOT_FOUND);
    // An unknown module is an error with its code.
    let e = install
        .resolve(id("k_script.ncs"), Some(&ModuleName::new("nowhere")))
        .unwrap_err();
    assert_eq!(e.code(), codes::NO_SUCH_MODULE);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_bif_is_the_fallback() {
    let root = make("bif");
    let install = Install::open(&root, Profile::K1).unwrap();
    let r = install.resolve(id("appearance.2da"), None).unwrap();
    assert_eq!(r.candidates.len(), 1);
    let w = r.winner().unwrap();
    assert_eq!(w.layer, Layer::Chitin);
    assert_eq!(w.hash, ContentHash::of(b"2da from the BIF"));
    match &w.location {
        Location::Bif { path, key, index } => {
            assert_eq!(path, &root.join("data/templates.bif"));
            assert_eq!(key, &root.join("chitin.key"));
            assert_eq!(*index, 0);
        }
        other => panic!("{other:?}"),
    }
    let (location, bytes) = install.read(id("appearance.2da"), None).unwrap().unwrap();
    assert_eq!(bytes, b"2da from the BIF");
    assert_eq!(location.short(), "templates.bif[0]");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn two_case_variants_in_override_are_both_candidates_and_the_winner_is_ambiguous() {
    let root = make("case");
    let install = Install::open(&root, Profile::K1).unwrap();
    let r = install.resolve(id("plc_invis.utp"), None).unwrap();
    assert_eq!(r.candidates.len(), 2);
    assert!(r.is_ambiguous());
    let variants = r.case_variants();
    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0].verdict, Verdict::Wins);
    assert_eq!(variants[1].verdict, Verdict::Ambiguous);
    let paths: Vec<&std::path::Path> = variants.iter().map(|c| c.location.path()).collect();
    assert_eq!(
        paths,
        [
            root.join("Override/plc_invis.utp"),
            root.join("Override/PLC_Invis.utp")
        ]
    );
    assert_ne!(variants[0].hash, variants[1].hash);
    assert_eq!(
        install.override_case_variants(),
        vec![vec![
            root.join("Override/PLC_Invis.utp"),
            root.join("Override/plc_invis.utp")
        ]]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_install_resolves_includes_for_nwscript() {
    let root = make("resolver");
    let mut install = Install::open(&root, Profile::K1).unwrap();
    let ns = install.nwscript().unwrap().unwrap();
    assert_eq!(&*ns.bytes, b"int NWSCRIPT;");
    assert!(ns.path.ends_with("templates.bif[2]"), "{}", ns.path);
    let inc = install.include("K_INC_X").unwrap().unwrap();
    assert_eq!(&*inc.bytes, b"int OVERRIDE;", "Override outranks the BIF");
    assert!(install.include("k_inc_missing").unwrap().is_none());
    assert!(
        install
            .include("a_name_far_longer_than_sixteen_bytes")
            .unwrap()
            .is_none()
    );
    let roles: Vec<&str> = install.inputs().iter().map(|i| i.role.as_str()).collect();
    assert_eq!(roles, ["nwscript", "include"]);
    assert_eq!(
        install.inputs()[1].sha256,
        ContentHash::of(b"int OVERRIDE;").to_hex()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_folder_without_chitin_key_is_not_an_install() {
    let root = std::env::temp_dir().join(format!("rakata-game-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let e = Install::open(&root, Profile::K1).unwrap_err();
    assert_eq!(e.code(), codes::NOT_AN_INSTALL);
    assert!(e.to_string().starts_with("RE0401: "), "{e}");
    fs::remove_dir_all(root).unwrap();
}
