//! The authoring API over the user's own installs: the Juhani mod's guards,
//! table rows, globals, clones and dialogue checks against vanilla KotOR 1,
//! written into a mirror of it whose `Override/` and `modules/` are the
//! test's own, a dialogue written into one Taris module's `.mod`, and the
//! checks over the modded build's dialogues read in place.
//!
//! Reads `$RAKATA_GAME_K1` (a vanilla KotOR 1 install) and `$RAKATA_BUILD`
//! (a modded build holding the Juhani mod), and skips, naming the variable,
//! when one is unset. Nothing is written to either install, and no game
//! bytes are committed.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use rakata::author::dialog::{self as dlg, Text};
use rakata::author::globals::{self, GlobalType};
use rakata::author::twoda::{self, RowKey};
use rakata::author::{Mod, blueprint, codes};
use rakata_2da::TwoDa;
use rakata_dialog::{Dialog, NewNode, NodeKey};
use rakata_game::{Install, Profile};
use rakata_gff::{Gff, Value};
use rakata_res::{ContentHash, ResId, ResRef};

/// The game's scripts the Juhani mod replaces, as `danm14ac_s.rim` ships
/// them (its `make.py`'s `REPLACED_SHA256`).
const REPLACED: [(&str, &str); 2] = [
    (
        "k_pdan_juhani31.ncs",
        "f684480ac7b9ace19d012f2107eb1f28f916d2f7d349965fe0185559ec44ef28",
    ),
    (
        "k_pdan_14juh_d.ncs",
        "02c307ed6ea5ae358fd85ce351ab85dce4f00ec6ecc8d32488f5942351dcef7e",
    ),
];

fn tree(var: &str) -> Option<PathBuf> {
    match std::env::var(var) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${var} is unset; set it to a KotOR 1 install");
            None
        }
    }
}

fn id(text: &str) -> ResId {
    ResId::parse(text).unwrap()
}

/// A mirror of `game`: every entry linked, except `Override/` and
/// `modules/`, whose files are linked one by one into folders of the
/// mirror's own, so a write replaces a link or adds a file and never
/// touches the install.
fn mirror(game: &Path, name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("rakata-author-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for e in fs::read_dir(game).unwrap() {
        let e = e.unwrap();
        let n = e.file_name();
        let own = ["override", "modules"]
            .iter()
            .any(|d| n.to_string_lossy().eq_ignore_ascii_case(d));
        if own {
            let dir = root.join(&n);
            fs::create_dir(&dir).unwrap();
            for f in fs::read_dir(e.path()).unwrap() {
                let f = f.unwrap();
                symlink(f.path(), dir.join(f.file_name())).unwrap();
            }
        } else {
            symlink(e.path(), root.join(&n)).unwrap();
        }
    }
    root
}

#[test]
fn the_juhani_mods_operations_against_vanilla() {
    let Some(game) = tree("RAKATA_GAME_K1") else {
        return;
    };
    let root = mirror(&game, "vanilla");
    let install = Install::open(&root, Profile::K1).unwrap();
    let mut m = Mod::open(install, Some("danm14ac")).unwrap();

    for (name, hash) in REPLACED {
        m.guard_replaced(id(name), hash.parse::<ContentHash>().unwrap())
            .unwrap();
    }
    let e = m
        .guard_replaced(id(REPLACED[0].0), ContentHash::of(b"another"))
        .unwrap_err();
    assert_eq!(e.code(), codes::REPLACED_DIFFERS);
    assert!(e.to_string().contains("danm14ac_s.rim"), "{e}");

    // The hidden appearance: a copy of the brith's row, a simple model.
    let mut app = TwoDa::parse(&m.loaded(id("appearance.2da")).unwrap().bytes).unwrap();
    let from = RowKey::Label("Creature_Brith".into());
    let brith = twoda::find(&app, &from).unwrap();
    assert_eq!(app.get(brith, "modeltype").unwrap(), "S");
    let rows = app.rows.len();
    let hidden = twoda::add_row_copying(
        &mut app,
        &from,
        "Juhani_Clones_Hidden",
        &[("race".into(), "PLC_Invis".into())],
    )
    .unwrap();
    assert_eq!(hidden, rows);
    m.write_override("appearance.2da", &app.to_bytes().unwrap())
        .unwrap();
    let back = TwoDa::parse(&fs::read(root.join("Override/appearance.2da")).unwrap()).unwrap();
    assert_eq!(back.get(hidden, "race").unwrap(), "PLC_Invis");
    assert_eq!(back.rows[hidden].label, hidden.to_string());

    // The story's flags.
    let mut cat = TwoDa::parse(&m.loaded(id("globalcat.2da")).unwrap().bytes).unwrap();
    for n in ["OAJ_SPECIAL", "OAJ_DONE", "OAJ_BANTER"] {
        globals::declare(&mut cat, n, GlobalType::Boolean).unwrap();
    }
    let e = globals::declare(&mut cat, "DAN_JUHANI_DONE", GlobalType::Boolean).unwrap_err();
    assert_eq!(e.code(), codes::GLOBAL_DECLARED);
    m.write_override("globalcat.2da", &cat.to_bytes().unwrap())
        .unwrap();

    // A clone of Juhani, her ability scores halved as make.py halves them.
    let juhani = Gff::parse(&m.loaded(id("dan14_juhani.utc")).unwrap().bytes).unwrap();
    let mut clone = blueprint::clone(&juhani, "oaj_juhani1").unwrap();
    blueprint::set(&mut clone, "Tag", Value::string("oaj_juhani1")).unwrap();
    for f in ["Str", "Dex", "Con", "Int", "Wis", "Cha"] {
        let Value::Byte(v) = blueprint::get(&clone, f).unwrap() else {
            panic!("{f} is a BYTE in a UTC");
        };
        blueprint::set(&mut clone, f, Value::Byte((v / 2).max(1))).unwrap();
    }
    blueprint::set_each(&mut clone, "ItemList", "Dropable", Value::Byte(0)).unwrap();
    m.write_override("oaj_juhani1.utc", &clone.to_bytes().unwrap())
        .unwrap();
    let written = Gff::parse(&fs::read(root.join("Override/oaj_juhani1.utc")).unwrap()).unwrap();
    assert_eq!(
        blueprint::get(&written, "TemplateResRef").unwrap(),
        Value::resref("oaj_juhani1").unwrap()
    );

    // A decompiled BioWare script compiles in BioWare's mode against the
    // game's own declarations.
    let src = rakata::author::scripts::Source::new(
        "k_oaj_probe",
        b"void main() { SetGlobalBoolean(\"OAJ_DONE\", TRUE); }".to_vec(),
    )
    .unwrap();
    let out =
        rakata::author::scripts::compile(&m, &[src], &[], rakata::author::scripts::Flags::BIOWARE)
            .unwrap();
    assert!(out[0].ncs.starts_with(b"NCS V1.0"));

    // A new banter from the game's: a node running a script not yet
    // compiled is refused, and the file is not written.
    let banter = Dialog::parse("banter.dlg", &m.loaded(id("banter.dlg")).unwrap().bytes).unwrap();
    let mut ours = dlg::empty_like(&banter, "oaj_banter.dlg");
    let e0 = dlg::add_entry(
        &mut ours,
        &NewNode {
            text: Text::English("Hey, Carth?".into()),
            speaker: "Mission".into(),
            script: Some(ResRef::new("k_oaj_beat").unwrap()),
            ..NewNode::default()
        },
    )
    .unwrap();
    dlg::start(&mut ours, &e0.into(), Some("k_oaj_c_done"), None).unwrap();
    let e = m
        .write_override("oaj_banter.dlg", &ours.to_bytes().unwrap())
        .unwrap_err();
    assert_eq!(e.code(), codes::DIALOG_REFUSED);
    let text = e.to_string();
    assert!(
        text.contains("k_oaj_beat") && text.contains("text:\"Hey, Carth?\""),
        "{text}"
    );
    assert!(!root.join("Override/oaj_banter.dlg").exists());

    // A vanilla dialogue's own faults are the game's: an addition that adds
    // none is written, and they come back as warnings.
    let name = "k_hcar_dialog.dlg";
    let mut carth = Dialog::parse(name, &m.loaded(id(name)).unwrap().bytes).unwrap();
    let before = m.check_dialog(&carth).len();
    let r = dlg::add_reply(
        &mut carth,
        &NewNode {
            text: Text::English("About what happened on Dantooine...".into()),
            ..NewNode::default()
        },
    )
    .unwrap();
    let first = NodeKey::Index(rakata_dialog::Kind::Entry, 0);
    dlg::link(&mut carth, &first, &r.into(), None, None).unwrap();
    let w = m.write_override(name, &carth.to_bytes().unwrap()).unwrap();
    assert_eq!(w.warnings.len(), before);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_build_already_holds_the_juhani_mods_replacements_and_its_dialogues_check_clean() {
    let Some(build) = tree("RAKATA_BUILD") else {
        return;
    };
    let install = Install::open(&build, Profile::K1).unwrap();
    let mut m = Mod::open(install, Some("danm14ac")).unwrap();
    if !build.join("Override/k_pdan_juhani31.ncs").exists() {
        eprintln!("skipped: the build at $RAKATA_BUILD does not hold the Juhani mod");
        return;
    }
    let (name, hash) = REPLACED[0];
    let e = m
        .guard_replaced(id(name), hash.parse::<ContentHash>().unwrap())
        .unwrap_err();
    assert_eq!(e.code(), codes::OVERRIDE_HOLDS);

    // Every reference the mod's own nodes make resolves: no finding names
    // one of its scripts, voices or globals.
    for d in [
        "dan14_cutjuh",
        "dan14_juhani",
        "k_hcar_dialog",
        "k_hmis_dialog",
        "oaj_banter",
    ] {
        let name = format!("{d}.dlg");
        let dialog = Dialog::parse(&name, &m.loaded(id(&name)).unwrap().bytes).unwrap();
        let findings = m.check_dialog(&dialog);
        let ours: Vec<String> = findings
            .iter()
            .filter(|f| {
                let s = f.subject.to_ascii_lowercase();
                s.starts_with("oaj") || s.starts_with("k_oaj")
            })
            .map(|f| f.message(&name))
            .collect();
        assert!(ours.is_empty(), "{ours:#?}");
    }
}

/// The Calo Nord plan's Scene 6: `tar02_carbast` edited for `tar_m02af`,
/// whose copy differs from `tar_m02aa`'s, written into `tar_m02af.mod`.
#[test]
fn tar02_carbast_is_edited_for_the_hideout_alone() {
    use rakata::diff::{self, Change, Options};
    use rakata_game::ModuleName;

    let Some(game) = tree("RAKATA_GAME_K1") else {
        return;
    };
    let root = mirror(&game, "carbast");
    let name = "tar02_carbast.dlg";
    let mut m = Mod::open(
        Install::open(&root, Profile::K1).unwrap(),
        Some("tar_m02af"),
    )
    .unwrap();
    let e = m.guard_other_modules(id(name)).unwrap_err();
    assert_eq!(e.code(), codes::OTHER_MODULE_DIFFERS);
    assert!(e.to_string().contains("tar_m02aa"), "{e}");

    let base = m.loaded(id(name)).unwrap();
    assert!(
        base.location.starts_with("modules/tar_m02af"),
        "{}",
        base.location
    );
    let mut d = Dialog::parse(name, &base.bytes).unwrap();
    let r = dlg::add_reply(
        &mut d,
        &NewNode {
            text: Text::English("A reply of the test's own.".into()),
            ..NewNode::default()
        },
    )
    .unwrap();
    let anchor = NodeKey::Voice(ResRef::new("nm02aacarb01002_").unwrap());
    dlg::link(&mut d, &anchor, &r.into(), None, Some(0)).unwrap();
    let ours = d.to_bytes().unwrap();
    let w = m.write_module(name, &ours).unwrap();
    assert!(
        w.path.ends_with("modules/tar_m02af.mod"),
        "{}",
        w.path.display()
    );

    let fresh = Install::open(&root, Profile::K1).unwrap();
    let vanilla = Install::open(&game, Profile::K1).unwrap();
    let read = |i: &Install, module: &str| {
        i.read(id(name), Some(&ModuleName::new(module)))
            .unwrap()
            .unwrap()
            .1
    };
    assert_eq!(read(&fresh, "tar_m02af"), ours);
    assert_eq!(read(&fresh, "tar_m02aa"), read(&vanilla, "tar_m02aa"));

    let options = Options {
        only: vec!["modules".into()],
        ..Options::default()
    };
    let d = diff::diff_trees(&game, &root, &options).unwrap();
    let text = d.text();
    let paths: Vec<&str> = d.entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "modules/tar_m02af.mod",
            "modules/tar_m02af.mod!tar02_carbast.dlg"
        ],
        "{text}"
    );
    assert_eq!(d.entries[1].change, Change::Added);
    assert!(text.contains("A reply of the test's own."), "{text}");
    fs::remove_dir_all(&root).unwrap();
}
