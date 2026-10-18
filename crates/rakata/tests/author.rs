//! The authoring API over a hand-made install: a KEY and a BIF with
//! `nwscript.nss`, an include, `appearance.2da`, `globalcat.2da`, a
//! creature and a ten-node dialogue; a module with the same script in its
//! `.rim` and `_s.rim`; a talk table and one voice-over. Every operation's
//! written bytes are asserted. No game bytes.

use std::fs;
use std::path::{Path, PathBuf};

use rakata::author::blueprint;
use rakata::author::dialog::{self as dlg, Text};
use rakata::author::globals::{self, GlobalType};
use rakata::author::scripts::{self, Flags, Source};
use rakata::author::twoda::{self, RowKey};
use rakata::author::voice::{self, Line, Take};
use rakata::author::{Mod, codes};
use rakata_2da::{Row, TwoDa};
use rakata_archive::{Bif, BifMember, BifRef, Key, KeyEntry, Rim};
use rakata_dialog::{Dialog, Kind, NewNode, NodeKey, NodeRef};
use rakata_game::{Install, Profile};
use rakata_gff::{Gff, Label, LocString, Struct, Value};
use rakata_lip::{Lip, Shape};
use rakata_res::{ContentHash, Language, ResId, ResRef, ResType};
use rakata_tlk::{Entry, Tlk};

const NWSCRIPT: &str = "int TRUE = 1;\r\nint FALSE = 0;\r\n\
int GetGlobalBoolean(string sIdentifier);\r\n\
void SetGlobalBoolean(string sIdentifier, int nValue);\r\n\
int GetGlobalNumber(string sIdentifier);\r\n";
const LIBRARY: &str = "int GameLibrary() { return TRUE; }\r\n";
const REPLACED: &[u8] = b"the game's k_replaced";

fn id(text: &str) -> ResId {
    ResId::parse(text).unwrap()
}

fn label(s: &str) -> Label {
    Label::new(s).unwrap()
}

fn table(columns: &[&str], rows: &[&[&str]]) -> TwoDa {
    let mut t = TwoDa::new(columns.iter().copied());
    for (i, r) in rows.iter().enumerate() {
        t.rows.push(Row {
            label: i.to_string(),
            cells: r.iter().map(|c| c.to_string()).collect(),
        });
    }
    t
}

fn appearance() -> TwoDa {
    table(
        &["label", "modeltype", "race"],
        &[
            &["Creature_Rancor", "F", "c_rancor"],
            &["Creature_Brith", "S", "c_brith"],
        ],
    )
}

fn globalcat() -> TwoDa {
    table(
        &["name", "type"],
        &[&["MAIN_PLOT", "Number"], &["OAJ_DONE", "Boolean"]],
    )
}

fn juhani() -> Gff {
    let mut g = Gff::new(*b"UTC ");
    let mut item = Struct::new(0);
    item.push(
        label("InventoryRes"),
        Value::resref("g_w_lghtsbr01").unwrap(),
    )
    .push(label("Dropable"), Value::Byte(1));
    g.root
        .push(label("Tag"), Value::string("Juhani"))
        .push(
            label("TemplateResRef"),
            Value::resref("dan14_juhani").unwrap(),
        )
        .push(label("Str"), Value::Byte(14))
        .push(label("HitPoints"), Value::Short(41))
        .push(
            label("ScriptSpawn"),
            Value::resref("k_def_spawn01").unwrap(),
        )
        .push(label("ItemList"), Value::List(vec![item]));
    g
}

fn said(t: &str) -> LocString {
    let mut l = LocString::none();
    l.set(0, t.as_bytes());
    l
}

fn link(index: u32) -> Struct {
    let mut s = Struct::new(0);
    s.push(label("Index"), Value::Dword(index))
        .push(label("Active"), Value::resref("").unwrap())
        .push(label("IsChild"), Value::Byte(0));
    s
}

/// Ten nodes, E0..E5 and R0..R3, chained; E1 and E2 say the same text; E3
/// is voiced by `vo_three`, which `streamwaves/` holds; E4 shows strref 1.
fn banter() -> Dialog {
    let mut entries = Vec::new();
    for i in 0..6u32 {
        let text = match i {
            1 | 2 => said("Again."),
            4 => LocString::strref(1),
            _ => said(&format!("Entry {i}.")),
        };
        let mut s = Struct::new(i);
        s.push(label("Speaker"), Value::string(""))
            .push(label("Text"), Value::LocString(text))
            .push(
                label("VO_ResRef"),
                Value::resref(if i == 3 { "vo_three" } else { "" }).unwrap(),
            )
            .push(label("Script"), Value::resref("").unwrap())
            .push(
                label("RepliesList"),
                Value::List(if i < 4 { vec![link(i)] } else { vec![] }),
            );
        entries.push(s);
    }
    let replies = (0..4u32)
        .map(|i| {
            let mut s = Struct::new(i);
            s.push(
                label("Text"),
                Value::LocString(said(&format!("Reply {i}."))),
            )
            .push(label("Script"), Value::resref("").unwrap())
            .push(label("EntriesList"), Value::List(vec![link(i + 1)]));
            s
        })
        .collect();
    let mut start = Struct::new(0);
    start
        .push(label("Index"), Value::Dword(0))
        .push(label("Active"), Value::resref("").unwrap());
    let mut g = Gff::new(*b"DLG ");
    g.root
        .push(label("Skippable"), Value::Byte(1))
        .push(label("EntryList"), Value::List(entries))
        .push(label("ReplyList"), Value::List(replies))
        .push(label("StartingList"), Value::List(vec![start]));
    Dialog::parse("banter.dlg", &g.to_bytes().unwrap()).unwrap()
}

/// The install, written fresh; with `differ`, the `_s.rim`'s copy of
/// `k_replaced.ncs` is another mod's.
fn make(name: &str, differ: bool) -> PathBuf {
    let root = std::env::temp_dir().join(format!("rakata-author-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for sub in ["Override", "modules", "data", "streamwaves/globe"] {
        fs::create_dir_all(root.join(sub)).unwrap();
    }
    let members: Vec<(&str, Vec<u8>)> = vec![
        ("nwscript.nss", NWSCRIPT.as_bytes().to_vec()),
        ("k_inc_game.nss", LIBRARY.as_bytes().to_vec()),
        ("appearance.2da", appearance().to_bytes().unwrap()),
        ("globalcat.2da", globalcat().to_bytes().unwrap()),
        ("dan14_juhani.utc", juhani().to_bytes().unwrap()),
        ("banter.dlg", banter().to_bytes().unwrap()),
    ];
    let mut bif = Bif::new();
    let mut key = Key::new();
    for (i, (n, bytes)) in members.iter().enumerate() {
        let rid = id(n);
        bif.push(BifMember::owned(i as u32, rid.ty, bytes.clone()));
        key.add(KeyEntry {
            id: rid,
            bif: 0,
            index: i as u32,
        })
        .unwrap();
    }
    let bif_bytes = bif.to_bytes().unwrap();
    fs::write(root.join("data/templates.bif"), &bif_bytes).unwrap();
    key.add_bif(BifRef {
        size: bif_bytes.len() as u32,
        name: b"data\\templates.bif".to_vec(),
        drives: 1,
    });
    fs::write(root.join("chitin.key"), key.to_bytes().unwrap()).unwrap();

    for (file, data) in [
        ("testmod.rim", REPLACED.to_vec()),
        (
            "testmod_s.rim",
            if differ {
                b"another mod's k_replaced".to_vec()
            } else {
                REPLACED.to_vec()
            },
        ),
    ] {
        let mut rim = Rim::new();
        rim.add(id("k_replaced.ncs"), data).unwrap();
        fs::write(root.join("modules").join(file), rim.to_bytes().unwrap()).unwrap();
    }

    let mut tlk = Tlk::new(Language::English);
    tlk.append(Entry::new("Hello there."));
    tlk.append(Entry::new("General Kenobi."));
    fs::write(root.join("dialog.tlk"), tlk.to_bytes().unwrap()).unwrap();
    fs::write(
        root.join("streamwaves/globe/vo_three.wav"),
        voice::streamwave(b"mp3"),
    )
    .unwrap();
    root
}

fn open(root: &Path) -> Mod {
    let install = Install::open(root, Profile::K1).unwrap();
    Mod::open(install, Some("testmod")).unwrap()
}

fn override_file(root: &Path, name: &str) -> Vec<u8> {
    fs::read(root.join("Override").join(name)).unwrap()
}

#[test]
fn loaded_finds_the_games_copy_and_then_the_mods_own_write() {
    let root = make("loaded", false);
    let mut m = open(&root);
    let l = m.loaded(id("appearance.2da")).unwrap();
    assert_eq!(l.bytes, appearance().to_bytes().unwrap());
    assert_eq!(l.location, "data/templates.bif[2]");
    assert!(!l.written);
    let l = m.loaded(id("k_replaced.ncs")).unwrap();
    assert_eq!(l.location, "modules/testmod.rim:k_replaced.ncs");

    let mut t = TwoDa::parse(&m.loaded(id("appearance.2da")).unwrap().bytes).unwrap();
    t.set(0, "race", "c_rancor2").unwrap();
    let w = m
        .write_override("appearance.2da", &t.to_bytes().unwrap())
        .unwrap();
    assert_eq!(w.path, root.join("Override/appearance.2da"));
    let l = m.loaded(id("appearance.2da")).unwrap();
    assert!(l.written);
    assert_eq!(l.bytes, t.to_bytes().unwrap());
    assert_eq!(l.location, "Override/appearance.2da");

    let e = m.loaded(id("nothere.utc")).unwrap_err();
    assert_eq!(e.code(), "RE0404");
    let e = Mod::open(Install::open(&root, Profile::K1).unwrap(), Some("nomod")).unwrap_err();
    assert_eq!(e.code(), "RE0403");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn guard_replaced_passes_when_every_module_copy_is_the_games() {
    let root = make("guard-ok", false);
    let m = open(&root);
    m.guard_replaced(id("k_replaced.ncs"), ContentHash::of(REPLACED))
        .unwrap();
    let e = m
        .guard_replaced(id("k_absent.ncs"), ContentHash::of(REPLACED))
        .unwrap_err();
    assert_eq!(e.code(), codes::NOT_IN_MODULE);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn guard_replaced_fails_naming_the_capsule_whose_copy_differs() {
    let root = make("guard-differs", true);
    let m = open(&root);
    let e = m
        .guard_replaced(id("k_replaced.ncs"), ContentHash::of(REPLACED))
        .unwrap_err();
    assert_eq!(e.code(), codes::REPLACED_DIFFERS);
    let text = e.to_string();
    assert!(text.contains("testmod_s.rim"), "{text}");
    assert!(!text.contains("testmod.rim'"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn guard_replaced_fails_when_override_holds_a_copy() {
    let root = make("guard-override", false);
    fs::write(root.join("Override/K_Replaced.ncs"), b"a mod's").unwrap();
    let m = open(&root);
    let e = m
        .guard_replaced(id("k_replaced.ncs"), ContentHash::of(REPLACED))
        .unwrap_err();
    assert_eq!(e.code(), codes::OVERRIDE_HOLDS);
    assert!(e.to_string().contains("K_Replaced.ncs"), "{e}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_row_copied_with_changed_cells_lands_at_the_end_of_the_table() {
    let root = make("twoda", false);
    let mut m = open(&root);
    let mut t = TwoDa::parse(&m.loaded(id("appearance.2da")).unwrap().bytes).unwrap();
    let from = RowKey::Label("Creature_Brith".into());
    let source = twoda::find(&t, &from).unwrap();
    assert_eq!(t.get(source, "modeltype").unwrap(), "S");
    let index = twoda::add_row_copying(
        &mut t,
        &from,
        "Juhani_Clones_Hidden",
        &[("race".into(), "PLC_Invis".into())],
    )
    .unwrap();
    assert_eq!(index, 2);
    m.write_override("appearance.2da", &t.to_bytes().unwrap())
        .unwrap();
    let mut expected = appearance();
    expected.rows.push(Row {
        label: "2".into(),
        cells: vec![
            "Juhani_Clones_Hidden".into(),
            "S".into(),
            "PLC_Invis".into(),
        ],
    });
    assert_eq!(
        override_file(&root, "appearance.2da"),
        expected.to_bytes().unwrap()
    );

    let e = twoda::add_row_copying(&mut t, &from, "Juhani_Clones_Hidden", &[]).unwrap_err();
    assert_eq!(e.code(), codes::LABEL_TAKEN);
    let e = twoda::add_row_copying(&mut t, &RowKey::Label("Creature_Bantha".into()), "x", &[])
        .unwrap_err();
    assert_eq!(e.code(), "RE0705");
    assert!(e.to_string().contains("Creature_Bantha"), "{e}");
    let e = twoda::add_row_copying(
        &mut t,
        &RowKey::Index(0),
        "y",
        &[("nope".into(), "1".into())],
    )
    .unwrap_err();
    assert_eq!(e.code(), "RE0706");
    assert_eq!(t.rows.len(), 3);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn declaring_a_global_that_exists_fails_naming_it_and_leaves_the_table() {
    let root = make("globals", false);
    let mut m = open(&root);
    let mut t = TwoDa::parse(&m.loaded(id("globalcat.2da")).unwrap().bytes).unwrap();
    let before = t.to_bytes().unwrap();
    let e = globals::declare(&mut t, "OAJ_DONE", GlobalType::Boolean).unwrap_err();
    assert_eq!(e.code(), codes::GLOBAL_DECLARED);
    assert!(e.to_string().contains("OAJ_DONE"), "{e}");
    let e = globals::declare(&mut t, "oaj_done", GlobalType::Boolean).unwrap_err();
    assert_eq!(e.code(), codes::GLOBAL_DECLARED);
    assert_eq!(t.to_bytes().unwrap(), before);

    assert_eq!(
        globals::declare(&mut t, "OAJ_SPECIAL", GlobalType::Number).unwrap(),
        2
    );
    m.write_override("globalcat.2da", &t.to_bytes().unwrap())
        .unwrap();
    let mut expected = globalcat();
    expected.rows.push(Row {
        label: "2".into(),
        cells: vec!["OAJ_SPECIAL".into(), "Number".into()],
    });
    assert_eq!(
        override_file(&root, "globalcat.2da"),
        expected.to_bytes().unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_cloned_blueprint_has_its_fields_set_by_path_and_keeps_their_types() {
    let root = make("blueprint", false);
    let mut m = open(&root);
    let source = Gff::parse(&m.loaded(id("dan14_juhani.utc")).unwrap().bytes).unwrap();
    let mut g = blueprint::clone(&source, "oaj_juhani1").unwrap();
    blueprint::set(&mut g, "Tag", Value::string("oaj_juhani1")).unwrap();
    blueprint::set(&mut g, "ScriptSpawn", Value::resref("k_oaj_clsp").unwrap()).unwrap();
    let Value::Byte(str_) = blueprint::get(&g, "Str").unwrap() else {
        panic!()
    };
    let old = blueprint::set(&mut g, "Str", Value::Byte((str_ / 2).max(1))).unwrap();
    assert_eq!(old, Some(Value::Byte(14)));
    assert_eq!(
        blueprint::set(&mut g, "Min1HP", Value::Byte(0)).unwrap(),
        None
    );
    assert_eq!(
        blueprint::set_each(&mut g, "ItemList", "Dropable", Value::Byte(0)).unwrap(),
        1
    );
    assert_eq!(
        blueprint::set_each(&mut g, "Equip_ItemList", "Dropable", Value::Byte(0)).unwrap(),
        0
    );
    let e = blueprint::set(&mut g, "HitPoints", Value::Word(20)).unwrap_err();
    assert_eq!(e.code(), codes::FIELD_TYPE);
    assert!(e.to_string().contains("HitPoints"), "{e}");
    let e = blueprint::set(&mut g, "ItemList[3].Dropable", Value::Byte(0)).unwrap_err();
    assert_eq!(e.code(), "RE0607");
    m.write_override("oaj_juhani1.utc", &g.to_bytes().unwrap())
        .unwrap();

    let mut expected = juhani();
    let r = &mut expected.root;
    r.set(label("Tag"), Value::string("oaj_juhani1"));
    r.set(
        label("TemplateResRef"),
        Value::resref("oaj_juhani1").unwrap(),
    );
    r.set(label("Str"), Value::Byte(7));
    r.set(label("ScriptSpawn"), Value::resref("k_oaj_clsp").unwrap());
    r.set(label("Min1HP"), Value::Byte(0));
    if let Some(Value::List(items)) = r.get_mut("ItemList") {
        items[0].set(label("Dropable"), Value::Byte(0));
    }
    let written = Gff::parse(&override_file(&root, "oaj_juhani1.utc")).unwrap();
    assert!(written.semantic_eq(&expected));
    assert_eq!(
        override_file(&root, "oaj_juhani1.utc"),
        g.to_bytes().unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}

fn line(text: &str) -> NewNode {
    NewNode {
        text: Text::English(text.into()),
        comment: "test".into(),
        ..NewNode::default()
    }
}

#[test]
fn an_added_entry_leaves_every_node_of_ten_at_its_index() {
    let root = make("dialog-append", false);
    let mut m = open(&root);
    let mut d = Dialog::parse("banter.dlg", &m.loaded(id("banter.dlg")).unwrap().bytes).unwrap();
    let before: Vec<(NodeRef, Struct)> = d.all().map(|n| (n.at, n.raw().clone())).collect();
    assert_eq!(before.len(), 10);
    let e = dlg::add_entry(&mut d, &line("Something new.")).unwrap();
    let r = dlg::add_reply(&mut d, &NewNode::default()).unwrap();
    dlg::link(
        &mut d,
        &NodeKey::Text("Entry 5.".into()),
        &r.into(),
        None,
        None,
    )
    .unwrap();
    dlg::link(&mut d, &r.into(), &e.into(), None, None).unwrap();
    dlg::start(&mut d, &e.into(), Some(""), None).unwrap();
    // The talk table's text names E4 for the mod's lookup.
    assert_eq!(
        m.find(&d, &NodeKey::Text("General Kenobi.".into()))
            .unwrap(),
        NodeRef::entry(4)
    );
    let w = m
        .write_override("banter.dlg", &d.to_bytes().unwrap())
        .unwrap();
    assert!(w.warnings.is_empty(), "{:?}", w.warnings);
    let bytes = override_file(&root, "banter.dlg");
    assert_eq!(bytes, d.to_bytes().unwrap());
    let again = Dialog::parse("banter.dlg", &bytes).unwrap();
    // Each of the ten keeps its index; only E5, which gained a link, differs.
    for (at, raw) in &before {
        let now = again.node(*at).unwrap();
        if *at == NodeRef::entry(5) {
            assert_eq!(now.links()[0].target, r);
        } else {
            assert_eq!(now.raw(), raw, "{at} moved or changed");
        }
    }
    assert_eq!(again.starts()[0].target, e);
    assert_eq!(again.starts()[1].target, NodeRef::entry(0));
    assert_eq!((again.len(Kind::Entry), again.len(Kind::Reply)), (7, 5));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_key_with_no_match_or_two_fails_with_the_dialogues_own_code() {
    let root = make("dialog-keys", false);
    let m = open(&root);
    let mut d = Dialog::parse("banter.dlg", &m.loaded(id("banter.dlg")).unwrap().bytes).unwrap();
    let to = NodeKey::Index(Kind::Reply, 0);
    let e = dlg::link(
        &mut d,
        &NodeKey::Text("Nobody says this.".into()),
        &to,
        None,
        None,
    )
    .unwrap_err();
    assert_eq!(e.code(), "RE1102");
    assert!(e.to_string().contains("text:\"Nobody says this.\""), "{e}");
    let e = dlg::link(&mut d, &NodeKey::Text("Again.".into()), &to, None, None).unwrap_err();
    assert_eq!(e.code(), "RE1103");
    assert!(e.to_string().contains("E1, E2"), "{e}");
    let e = dlg::link(
        &mut d,
        &NodeKey::Index(Kind::Entry, 0),
        &NodeKey::Index(Kind::Entry, 1),
        None,
        None,
    )
    .unwrap_err();
    assert_eq!(e.code(), "RE1106");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_node_whose_script_does_not_resolve_is_refused_and_nothing_is_written() {
    let root = make("dialog-refused", false);
    let mut m = open(&root);
    let mut d = Dialog::parse("banter.dlg", &m.loaded(id("banter.dlg")).unwrap().bytes).unwrap();
    let mut n = line("A beat.");
    n.voice = Some(ResRef::new("oaj_beat01").unwrap());
    n.script = Some(ResRef::new("k_oaj_missing").unwrap());
    dlg::add_entry(&mut d, &n).unwrap();
    let e = m
        .write_override("banter.dlg", &d.to_bytes().unwrap())
        .unwrap_err();
    assert_eq!(e.code(), codes::DIALOG_REFUSED);
    let text = e.to_string();
    assert!(text.contains("k_oaj_missing"), "{text}");
    assert!(text.contains("voice:oaj_beat01"), "{text}");
    assert!(text.contains("RW1102") && text.contains("RW1104"), "{text}");
    assert!(!root.join("Override/banter.dlg").exists());
    assert!(!m.loaded(id("banter.dlg")).unwrap().written);

    // The script compiled and the voice written, the same dialogue lands.
    let script = Source::new("k_oaj_missing", b"void main() { }".to_vec()).unwrap();
    let ncs = scripts::compile(&m, &[script], &[], Flags::default()).unwrap();
    m.write_override("k_oaj_missing.ncs", &ncs[0].ncs).unwrap();
    m.write_streamwaves("oaj_beat01", &voice::streamwave(b"mp3"))
        .unwrap();
    m.write_override("banter.dlg", &d.to_bytes().unwrap())
        .unwrap();
    assert_eq!(override_file(&root, "banter.dlg"), d.to_bytes().unwrap());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_condition_reading_an_undeclared_global_is_refused_until_it_is_declared() {
    let root = make("dialog-global", false);
    let mut m = open(&root);
    let cond = Source::new("k_oaj_c_new", b"int StartingConditional() { return GetGlobalBoolean(\"OAJ_NEW\") && GetGlobalBoolean(\"OAJ_DONE\"); }"
            .to_vec()).unwrap();
    let ncs = scripts::compile(&m, &[cond], &[], Flags::default()).unwrap();
    m.write_override("k_oaj_c_new.ncs", &ncs[0].ncs).unwrap();
    let mut d = Dialog::parse("banter.dlg", &m.loaded(id("banter.dlg")).unwrap().bytes).unwrap();
    dlg::start(
        &mut d,
        &NodeKey::Text("Entry 0.".into()),
        Some("k_oaj_c_new"),
        None,
    )
    .unwrap();
    let e = m
        .write_override("banter.dlg", &d.to_bytes().unwrap())
        .unwrap_err();
    assert_eq!(e.code(), codes::DIALOG_REFUSED);
    let text = e.to_string();
    assert!(
        text.contains("OAJ_NEW") && text.contains("RW1106"),
        "{text}"
    );
    assert!(!text.contains("global OAJ_DONE"), "{text}");

    let mut t = TwoDa::parse(&m.loaded(id("globalcat.2da")).unwrap().bytes).unwrap();
    globals::declare(&mut t, "OAJ_NEW", GlobalType::Boolean).unwrap();
    m.write_override("globalcat.2da", &t.to_bytes().unwrap())
        .unwrap();
    m.write_override("banter.dlg", &d.to_bytes().unwrap())
        .unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_reference_the_loaded_copy_already_lacked_is_a_warning() {
    let root = make("dialog-base", false);
    let mut m = open(&root);
    let mut d = Dialog::parse("banter.dlg", &m.loaded(id("banter.dlg")).unwrap().bytes).unwrap();
    // The game's copy, broken: E0 runs a script nothing holds.
    let mut g = d.gff().clone();
    g.set(
        &rakata_gff::Path::parse("EntryList[0].Script").unwrap(),
        Value::resref("k_gone").unwrap(),
    )
    .unwrap();
    fs::write(root.join("Override/banter.dlg"), g.to_bytes().unwrap()).unwrap();
    let mut m2 = open(&root);
    d = Dialog::parse("banter.dlg", &m2.loaded(id("banter.dlg")).unwrap().bytes).unwrap();
    dlg::add_entry(&mut d, &line("Fine.")).unwrap();
    let w = m2
        .write_override("banter.dlg", &d.to_bytes().unwrap())
        .unwrap();
    assert_eq!(w.warnings.len(), 1);
    assert_eq!(w.warnings[0].subject, "k_gone");
    m.check_dialog(&d);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reordering_is_its_own_operation_and_rewrites_the_links() {
    let root = make("dialog-reorder", false);
    let m = open(&root);
    let mut d = Dialog::parse("banter.dlg", &m.loaded(id("banter.dlg")).unwrap().bytes).unwrap();
    dlg::reorder(&mut d, Kind::Reply, &[3, 2, 1, 0]).unwrap();
    assert_eq!(
        d.node(NodeRef::entry(0)).unwrap().links()[0].target,
        NodeRef::reply(3)
    );
    let e = dlg::reorder(&mut d, Kind::Reply, &[0, 1, 2]).unwrap_err();
    assert_eq!(e.code(), "RE1107");
    let empty = dlg::empty_like(&d, "oaj_banter.dlg");
    assert_eq!(empty.len(Kind::Entry), 0);
    assert_eq!(empty.info().skippable, Some(true));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scripts_compile_against_the_mods_libraries_and_the_installs_includes() {
    let root = make("scripts", false);
    let mut m = open(&root);
    let lib = Source::new(
        "k_oaj_inc",
        b"#include \"k_inc_game\"\r\nint ModLibrary() { return GameLibrary(); }\r\n".to_vec(),
    )
    .unwrap();
    let script = Source::new("k_oaj_fight", b"#include \"k_oaj_inc\"\r\nvoid main() { SetGlobalBoolean(\"OAJ_DONE\", ModLibrary()); }\r\n"
            .to_vec()).unwrap();
    let out = scripts::compile(
        &m,
        std::slice::from_ref(&script),
        std::slice::from_ref(&lib),
        Flags::default(),
    )
    .unwrap();
    assert_eq!(out.len(), 1);
    let roles: Vec<(&str, &str)> = out[0]
        .inputs
        .iter()
        .map(|i| (i.role.as_str(), i.path.as_str()))
        .collect();
    assert!(
        roles.contains(&("include", "data/templates.bif[1]")),
        "{roles:?}"
    );
    assert!(roles.contains(&("include", "k_oaj_inc.nss")), "{roles:?}");

    // The same bytes as `nwscript` compiles them from memory.
    let mut r = nwscript::MemResolver::new()
        .with("nwscript.nss", NWSCRIPT)
        .with("k_inc_game.nss", LIBRARY)
        .with("k_oaj_inc.nss", lib.text().to_vec());
    let mut map = nwscript::diag::SourceMap::new();
    let mut c = nwscript::Checker::new();
    let checked = c.check(&mut r, &mut map, "k_oaj_fight.nss", script.text());
    let program =
        nwscript::compile::compile(&checked.analysis.unwrap().program, nwscript::Game::K1).unwrap();
    assert_eq!(out[0].ncs, nwscript::ncs::encode(&program));
    m.write_override("k_oaj_fight.ncs", &out[0].ncs).unwrap();
    assert_eq!(override_file(&root, "k_oaj_fight.ncs"), out[0].ncs);

    // A name the compiled script's resref cannot hold is refused at once.
    let e = Source::new("k_oaj_seventeen01", "void main() { }").unwrap_err();
    assert_eq!(e.code(), "RE0101");
    assert_eq!(
        e.to_string(),
        "RE0101: resource name `k_oaj_seventeen01` is 17 bytes; a resref holds at most 16"
    );
    assert!(Source::new("k_oaj_sixteen_01", "").is_ok());

    let bad = Source::new("k_oaj_bad", b"void main() { NoSuchRoutine(); }".to_vec()).unwrap();
    let e = scripts::compile(&m, &[bad], &[], Flags::default()).unwrap_err();
    assert_eq!(e.code(), codes::DOES_NOT_COMPILE);
    assert!(e.to_string().contains("NoSuchRoutine"), "{e}");
    let e = m
        .write_override("k_oaj_bad.ncs", b"not bytecode")
        .unwrap_err();
    assert_eq!(e.code(), codes::NOT_NCS);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_lip_from_rhubarbs_cues_is_written_as_its_keyframes() {
    let root = make("lip", false);
    let mut m = open(&root);
    let json = r#"{"metadata": {"soundFile": "line.wav", "duration": 1.50},
        "mouthCues": [{"start": 0.00, "end": 0.20, "value": "X"},
                      {"start": 0.20, "end": 0.45, "value": "B"},
                      {"start": 0.45, "end": 0.90, "value": "D"},
                      {"start": 0.90, "end": 1.50, "value": "A"}]}"#;
    let lip = voice::take_cues(json).unwrap();
    m.write_override("oaj_juh01.lip", &lip.to_bytes().unwrap())
        .unwrap();
    let expected = Lip::from_cues(
        [
            (0.0, Shape::Rest),
            (0.2, Shape::S),
            (0.45, Shape::Ah),
            (0.9, Shape::M),
        ],
        1.5,
    );
    assert_eq!(expected.keys.last().unwrap().shape, Shape::Rest);
    assert_eq!(
        override_file(&root, "oaj_juh01.lip"),
        expected.to_bytes().unwrap()
    );
    let same = rakata::author::lip::from_cues(
        &[
            (0.0, Shape::Rest),
            (0.2, Shape::S),
            (0.45, Shape::Ah),
            (0.9, Shape::M),
        ],
        1.5,
    );
    assert!(same.bits_eq(&lip));
    let e = voice::take_cues(
        r#"{"metadata": {"duration": 1}, "mouthCues": [{"start": 0, "value": "Q"}]}"#,
    )
    .unwrap_err();
    assert_eq!(e.code(), codes::BAD_CUES);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_voice_over_is_written_to_streamwaves_behind_the_games_header() {
    let root = make("streamwaves", false);
    let mut m = open(&root);
    let w = m
        .write_streamwaves("oaj_juh01", &voice::streamwave(b"ID3 mp3 frames"))
        .unwrap();
    assert_eq!(w.path, root.join("streamwaves/oaj_juh01.wav"));
    let bytes = fs::read(&w.path).unwrap();
    assert_eq!(&bytes[..58], &voice::WAV_HEADER);
    assert_eq!(&bytes[58..], b"ID3 mp3 frames");
    // The game's own voice-over and the new one both satisfy the check.
    use rakata_dialog::References;
    assert!(m.resolves(&ResRef::new("VO_THREE").unwrap(), ResType::Wav));
    assert!(m.resolves(&ResRef::new("oaj_juh01").unwrap(), ResType::Wav));
    let e = m
        .write_streamwaves("oaj_juh02", b"ID3 bare mp3")
        .unwrap_err();
    assert_eq!(e.code(), codes::NOT_A_WAVE);
    assert!(!root.join("streamwaves/oaj_juh02.wav").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn spans_cover_the_lines_segments_with_the_lead_and_a_stable_key() {
    let transcript = r#"{"results": {"audio_segments": [
        {"id": 0, "start_time": "0.1", "end_time": "1.25"},
        {"id": 1, "start_time": "1.9", "end_time": "3.0"},
        {"id": 2, "start_time": "3.5", "end_time": "4.75"}]}}"#;
    let take = Take::from_transcript("juhani_take1", b"the take's mp3", transcript).unwrap();
    let lines = vec![
        Line {
            resref: "oaj_juh01".into(),
            take: "juhani_take1".into(),
            text: "First.".into(),
            first: Some(0),
            last: Some(0),
        },
        Line {
            resref: "oaj_juh02".into(),
            take: "juhani_take1".into(),
            text: "Then the rest.".into(),
            first: Some(1),
            last: None,
        },
        Line {
            resref: "oaj_juh03".into(),
            take: "juhani_take1".into(),
            text: "All of it.".into(),
            first: None,
            last: None,
        },
    ];
    let spans = voice::spans(&lines, std::slice::from_ref(&take), voice::LEAD).unwrap();
    assert_eq!(
        spans
            .iter()
            .map(|s| (s.start_text(), s.end_text()))
            .collect::<Vec<_>>(),
        [
            ("0.000".to_string(), "1.400".to_string()),
            ("1.750".to_string(), "4.900".to_string()),
            ("0.000".to_string(), "4.900".to_string()),
        ]
    );
    assert_eq!(spans[0].key.len(), 64);
    assert_ne!(spans[0].key, spans[1].key);
    // The key is the take and the span: a third line over the same
    // stretch shares it, and another take does not.
    assert_eq!(
        voice::spans(&lines[2..], std::slice::from_ref(&take), voice::LEAD).unwrap()[0].key,
        spans[2].key
    );
    let other = Take::from_transcript("juhani_take1", b"a new recording", transcript).unwrap();
    assert_ne!(
        voice::spans(&lines[..1], &[other], voice::LEAD).unwrap()[0].key,
        spans[0].key
    );

    let mut dup = lines.clone();
    dup[1].resref = "OAJ_JUH01".into();
    assert_eq!(
        voice::spans(&dup, std::slice::from_ref(&take), voice::LEAD)
            .unwrap_err()
            .code(),
        codes::DUPLICATE_LINE
    );
    let mut far = lines[..1].to_vec();
    far[0].last = Some(7);
    let e = voice::spans(&far, std::slice::from_ref(&take), voice::LEAD).unwrap_err();
    assert_eq!(e.code(), codes::BAD_SPAN);
    let mut long = lines[..1].to_vec();
    long[0].resref = "a_resref_too_long_for_the_game".into();
    assert_eq!(
        voice::spans(&long, std::slice::from_ref(&take), voice::LEAD)
            .unwrap_err()
            .code(),
        "RE0101"
    );
    assert_eq!(
        Take::from_transcript("t", b"", "{}").unwrap_err().code(),
        codes::BAD_CUES
    );
}

#[test]
fn the_mod_and_its_errors_cross_threads_as_the_bindings_need() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Mod>();
    send_sync::<rakata::author::Error>();
    send_sync::<Dialog>();
}

/// A second module, `othermod`, whose `.rim` holds `members`.
fn other_module(root: &Path, members: &[(&str, &[u8])]) {
    let mut rim = Rim::new();
    for (n, data) in members {
        rim.add(id(n), data.to_vec()).unwrap();
    }
    fs::write(root.join("modules/othermod.rim"), rim.to_bytes().unwrap()).unwrap();
}

#[test]
fn a_mod_moved_to_another_module_keeps_its_writes_and_refuses_an_unknown_one() {
    let root = make("set-module", false);
    other_module(&root, &[("k_other.ncs", b"other's")]);
    let mut m = open(&root);
    let mut t = TwoDa::parse(&m.loaded(id("globalcat.2da")).unwrap().bytes).unwrap();
    globals::declare(&mut t, "CN_MET", GlobalType::Boolean).unwrap();
    m.write_override("globalcat.2da", &t.to_bytes().unwrap())
        .unwrap();
    assert!(m.loaded(id("k_other.ncs")).is_err());

    m.set_module(Some("othermod")).unwrap();
    assert_eq!(
        m.module().map(ToString::to_string).as_deref(),
        Some("othermod")
    );
    assert_eq!(
        m.loaded(id("k_other.ncs")).unwrap().location,
        "modules/othermod.rim:k_other.ncs"
    );
    assert!(m.loaded(id("globalcat.2da")).unwrap().written);
    assert!(m.loaded(id("k_replaced.ncs")).is_err());

    let e = m.set_module(Some("nomod")).unwrap_err();
    assert_eq!(e.code(), "RE0403");
    assert_eq!(
        m.module().map(ToString::to_string).as_deref(),
        Some("othermod")
    );
    m.set_module(None).unwrap();
    assert!(m.module().is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_override_write_is_guarded_against_another_modules_own_copy() {
    let root = make("other-modules", false);
    other_module(
        &root,
        &[
            ("k_replaced.ncs", b"othermod's own k_replaced"),
            ("appearance.2da", b"othermod's own appearance"),
            ("k_same.ncs", REPLACED),
        ],
    );
    let m = open(&root);
    // testmod's copy is its .rim's; othermod's differs
    let e = m.guard_other_modules(id("k_replaced.ncs")).unwrap_err();
    assert_eq!(e.code(), codes::OTHER_MODULE_DIFFERS);
    let text = e.to_string();
    assert!(
        text.contains("othermod.rim holds othermod's own k_replaced.ncs"),
        "{text}"
    );
    assert!(text.contains("the copy testmod loads"), "{text}");
    // A global resource a module carries its own copy of
    let e = m.guard_other_modules(id("appearance.2da")).unwrap_err();
    assert_eq!(e.code(), codes::OTHER_MODULE_DIFFERS);
    // No other module holds it, or holds the same bytes
    m.guard_other_modules(id("globalcat.2da")).unwrap();
    m.guard_other_modules(id("k_nowhere.ncs")).unwrap();
    // A copy in Override already serves every module
    fs::write(root.join("Override/k_replaced.ncs"), b"a mod's").unwrap();
    open(&root)
        .guard_other_modules(id("k_replaced.ncs"))
        .unwrap();
    fs::remove_dir_all(&root).unwrap();

    let root = make("other-modules-same", false);
    other_module(&root, &[("k_replaced.ncs", REPLACED)]);
    open(&root)
        .guard_other_modules(id("k_replaced.ncs"))
        .unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_nodes_script_is_replaced_only_when_it_runs_the_one_named() {
    let root = make("replace-script", false);
    let m = open(&root);
    let mut d = Dialog::parse("banter.dlg", &m.loaded(id("banter.dlg")).unwrap().bytes).unwrap();
    let e3 = NodeKey::Voice(ResRef::new("vo_three").unwrap());
    let before = d.to_bytes().unwrap();
    let e = dlg::replace_script(&mut d, &e3, Some("k_old"), Some("k_new")).unwrap_err();
    assert_eq!(e.code(), codes::SCRIPT_DIFFERS);
    assert!(e.to_string().contains("runs no script, not `k_old`"), "{e}");
    assert_eq!(d.to_bytes().unwrap(), before);
    let e = dlg::replace_script(
        &mut d,
        &NodeKey::Text("Nobody.".into()),
        None,
        Some("k_new"),
    )
    .unwrap_err();
    assert_eq!(e.code(), "RE1102");

    dlg::replace_script(&mut d, &e3, None, Some("k_new")).unwrap();
    assert_eq!(
        d.node(NodeRef::entry(3))
            .unwrap()
            .script()
            .unwrap()
            .to_string(),
        "k_new"
    );
    dlg::replace_script(&mut d, &e3, Some("K_NEW"), Some("k_newer")).unwrap();
    assert_eq!(
        d.node(NodeRef::entry(3))
            .unwrap()
            .script()
            .unwrap()
            .to_string(),
        "k_newer"
    );
    // Only the one field changed, in place
    let a = Dialog::parse("banter.dlg", &before).unwrap();
    for n in a.all() {
        let now = d.node(n.at).unwrap();
        if n.at == NodeRef::entry(3) {
            let fields = |s: &Struct| {
                s.fields
                    .iter()
                    .map(|f| f.label.to_string())
                    .collect::<Vec<_>>()
            };
            assert_eq!(fields(now.raw()), fields(n.raw()));
        } else {
            assert_eq!(now.raw(), n.raw(), "{} changed", n.at);
        }
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_globals_type_is_its_declared_one_and_an_undeclared_global_is_refused() {
    let root = make("global-type", false);
    let m = open(&root);
    let t = TwoDa::parse(&m.loaded(id("globalcat.2da")).unwrap().bytes).unwrap();
    assert_eq!(
        globals::type_of(&t, "MAIN_PLOT").unwrap(),
        GlobalType::Number
    );
    assert_eq!(
        globals::type_of(&t, "oaj_done").unwrap(),
        GlobalType::Boolean
    );
    let e = globals::type_of(&t, "CN_UNDECLARED").unwrap_err();
    assert_eq!(e.code(), codes::GLOBAL_UNDECLARED);
    assert!(
        e.to_string().contains("does not declare CN_UNDECLARED"),
        "{e}"
    );
    let e = globals::type_of(&TwoDa::new(["name"]), "MAIN_PLOT").unwrap_err();
    assert_eq!(e.code(), "RE0706");
    fs::remove_dir_all(root).unwrap();
}
