//! `rakata save show` over a save built here, in the layout the game writes
//! (`docs/formats/save.md`), and over the user's own saves when
//! `$RAKATA_SAVES` names them. No save bytes are committed.

use assert_cmd::Command;
use rakata_archive::{Erf, ErfKind};
use rakata_gff::{Gff, Label, LocString, Struct, SubString, Value};
use rakata_res::{ResId, ResRef, ResType};
use std::path::{Path, PathBuf};

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

fn l(t: &str) -> Label {
    Label::new(t).unwrap()
}

fn id(name: &str, ty: ResType) -> ResId {
    ResId::new(ResRef::new(name).unwrap(), ty)
}

fn named(name: &str) -> Struct {
    let mut s = Struct::new(0);
    s.push(l("Name"), Value::string(name));
    s
}

fn word(v: u32) -> Struct {
    let mut s = Struct::new(0);
    s.push(l("Variable"), Value::Dword(v));
    s
}

fn param(t: u32, v: Value) -> Struct {
    let mut s = Struct::new(1);
    s.push(l("Type"), Value::Dword(t)).push(l("Value"), v);
    s
}

/// A creature record: a tag, a first name, AI off (local boolean 62), and
/// an action queue of `actions` (id, params).
fn creature(object: u32, tag: &str, name: &str, actions: Vec<(u32, Vec<Struct>)>) -> Struct {
    let mut sw = Struct::new(0);
    sw.push(
        l("BitArray"),
        Value::List(vec![word(0), word(0x4000_0000), word(0)]),
    );
    let mut s = Struct::new(4);
    s.push(l("ObjectId"), Value::Dword(object))
        .push(
            l("FirstName"),
            Value::LocString(LocString {
                strref: LocString::NO_STRREF,
                substrings: vec![SubString {
                    id: 0,
                    text: name.as_bytes().to_vec(),
                }],
            }),
        )
        .push(l("Tag"), Value::string(tag))
        .push(l("FactionID"), Value::Word(5))
        .push(l("Animation"), Value::Int(10000))
        .push(l("SWVarTable"), Value::Struct(sw));
    let list = actions
        .into_iter()
        .enumerate()
        .map(|(g, (a, ps))| {
            let mut e = Struct::new(0);
            e.push(l("ActionId"), Value::Dword(a))
                .push(l("GroupActionId"), Value::Word(g as u16 + 1))
                .push(l("NumParams"), Value::Word(ps.len() as u16))
                .push(l("Paramaters"), Value::List(ps));
            e
        })
        .collect();
    s.push(l("ActionList"), Value::List(list))
        .push(l("Commandable"), Value::Byte(1));
    s
}

fn bytes(g: &Gff) -> Vec<u8> {
    g.to_bytes().unwrap()
}

/// A save directory: a module `danm14ac` with `carth` in its creature list,
/// the party member `Mission` with a standing action and a conversation
/// queued behind it, the faction table, and two globals.
fn build(dir: &Path) {
    let mut git = Gff::new(*b"GIT ");
    git.root.push(
        l("Creature List"),
        Value::List(vec![creature(561, "carth", "Carth", vec![])]),
    );
    let mut ifo = Gff::new(*b"IFO ");
    let mut ev = Struct::new(0xABCD);
    let mut data = Struct::new(0x7777);
    data.push(l("Name"), Value::string("k_oaj_fight"));
    ev.push(l("Day"), Value::Dword(0))
        .push(l("Time"), Value::Dword(348_666))
        .push(l("ObjectId"), Value::Dword(561))
        .push(l("CallerId"), Value::Dword(561))
        .push(l("EventId"), Value::Dword(1))
        .push(l("EventData"), Value::Struct(data));
    ifo.root.push(l("EventQueue"), Value::List(vec![ev]));
    let mut module = Erf::new(ErfKind::Mod);
    module.add(id("m14ac", ResType::Git), bytes(&git)).unwrap();
    module.add(id("Module", ResType::Ifo), bytes(&ifo)).unwrap();

    let npc = {
        let mut g = Gff::new(*b"UTC ");
        g.root = creature(
            0x7F00_0000,
            "mission",
            "Mission",
            vec![
                (61, vec![]),
                (
                    24,
                    vec![
                        param(3, Value::Dword(561)),
                        param(4, Value::string("oaj_banter")),
                        param(1, Value::Int(0)),
                    ],
                ),
            ],
        );
        g.root.id = Struct::ROOT_ID;
        g
    };
    let mut fac = Gff::new(*b"FAC ");
    let factions = [
        "Player",
        "Hostile_1",
        "Friendly_1",
        "Hostile_2",
        "Friendly_2",
        "Neutral",
    ]
    .iter()
    .map(|n| {
        let mut s = Struct::new(0);
        s.push(l("FactionName"), Value::string(n));
        s
    })
    .collect();
    fac.root.push(l("FactionList"), Value::List(factions));

    let mut outer = Erf::new(ErfKind::Mod);
    outer
        .add(id("AVAILNPC6", ResType::Utc), bytes(&npc))
        .unwrap();
    outer
        .add(id("danm14ac", ResType::Sav), module.to_bytes().unwrap())
        .unwrap();
    outer.add(id("REPUTE", ResType::Fac), bytes(&fac)).unwrap();
    std::fs::write(dir.join("SAVEGAME.sav"), outer.to_bytes().unwrap()).unwrap();

    let mut gvt = Gff::new(*b"GVT ");
    gvt.root
        .push(
            l("CatBoolean"),
            Value::List(vec![named("OAJ_DONE"), named("OAJ_DOWN_0"), named("TAR_X")]),
        )
        .push(l("CatNumber"), Value::List(vec![named("OAJ_COUNT")]))
        .push(l("CatLocation"), Value::List(vec![]))
        .push(l("CatString"), Value::List(vec![]))
        .push(l("ValBoolean"), Value::Void(vec![0b1010_0000]))
        .push(l("ValNumber"), Value::Void(vec![0xFF]))
        .push(l("ValLocation"), Value::Void(vec![]))
        .push(l("ValString"), Value::List(vec![]));
    std::fs::write(dir.join("GLOBALVARS.res"), bytes(&gvt)).unwrap();

    let mut pt = Gff::new(*b"PT  ");
    let mut m = Struct::new(0);
    m.push(l("PT_MEMBER_ID"), Value::Int(6))
        .push(l("PT_IS_LEADER"), Value::Byte(0));
    pt.root
        .push(l("PT_MEMBERS"), Value::List(vec![m]))
        .push(l("PT_AVAIL_NPCS"), Value::List(vec![]));
    std::fs::write(dir.join("PARTYTABLE.res"), bytes(&pt)).unwrap();

    let mut nfo = Gff::new(*b"NFO ");
    nfo.root
        .push(l("AREANAME"), Value::string("Dantooine - Grove"))
        .push(l("LASTMODULE"), Value::string("danm14ac"));
    std::fs::write(dir.join("savenfo.res"), bytes(&nfo)).unwrap();
}

/// A corpus whose one include script names local boolean 62, as the game's
/// `k_inc_generic.nss` does.
fn corpus(dir: &Path) -> PathBuf {
    let c = dir.join("corpus");
    std::fs::create_dir_all(c.join("nss")).unwrap();
    std::fs::write(
        c.join("nss/k_inc_generic.nss"),
        "int SW_FLAG_AI_OFF               = 62;\n",
    )
    .unwrap();
    c
}

fn show(save: &Path, corpus: &Path, args: &[&str]) -> (i32, String, String) {
    let out = rakata()
        .env("NWSCRIPT_CORPUS", corpus)
        .args(["save", "show"])
        .arg(save)
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn a_module_creature_and_a_party_member_are_found_by_tag_in_any_case() {
    let tmp = tempfile::tempdir().unwrap();
    let save = tmp.path().join("000001 - Game0");
    std::fs::create_dir(&save).unwrap();
    build(&save);
    let corpus = corpus(tmp.path());

    let (code, text, err) = show(
        &save,
        &corpus,
        &["--module", "danm14ac", "--creature", "CARTH"],
    );
    assert_eq!(code, 0, "{err}");
    assert!(
        text.contains("carth  danm14ac/m14ac.git Creature List[0]"),
        "{text}"
    );
    assert!(text.contains("\"carth\" (as saved)"), "{text}");
    assert!(text.contains("62 SW_FLAG_AI_OFF"), "{text}");
    assert!(text.contains("5 Neutral"), "{text}");
    assert!(text.contains("10000 (table unknown)"), "{text}");
    assert!(text.contains("script k_oaj_fight"), "{text}");

    let (code, text, err) = show(&save, &corpus, &["--creature", "Mission"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        text.contains("mission  SAVEGAME.sav/AVAILNPC6.utc"),
        "{text}"
    );
    let at61 = text.find("[0] 61 (table unknown)").expect(&text);
    let at24 = text
        .find("[1] 24 ActionStartConversation  group 2  (object 561 carth, string \"oaj_banter\", int 0)")
        .expect(&text);
    assert!(at61 < at24);
}

#[test]
fn globals_are_listed_by_pattern_with_their_values() {
    let tmp = tempfile::tempdir().unwrap();
    build(tmp.path());
    let (code, text, err) = show(tmp.path(), &corpus(tmp.path()), &["--globals", "oaj_"]);
    assert_eq!(code, 0, "{err}");
    assert!(text.contains("globals matching `oaj_` (3)"), "{text}");
    assert!(text.contains("boolean  OAJ_DONE    true"), "{text}");
    assert!(text.contains("boolean  OAJ_DOWN_0  false"), "{text}");
    assert!(text.contains("number   OAJ_COUNT   -1"), "{text}");
    assert!(!text.contains("TAR_X"), "{text}");
    assert!(text.contains("party    mission (AVAILNPC6)"), "{text}");
}

#[test]
fn json_carries_the_creatures_flags_by_name_and_the_queue() {
    let tmp = tempfile::tempdir().unwrap();
    build(tmp.path());
    let out = rakata()
        .env("NWSCRIPT_CORPUS", corpus(tmp.path()))
        .args(["--format", "json", "save", "show"])
        .arg(tmp.path())
        .args(["--creature", "mission"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["command"], "save show");
    let c = &doc["result"]["creatures"][0];
    assert_eq!(c["tag"], "mission");
    assert_eq!(c["place"]["kind"], "party");
    assert_eq!(c["booleans"][0]["index"], 62);
    assert_eq!(c["booleans"][0]["name"], "SW_FLAG_AI_OFF");
    assert_eq!(c["actions"][0]["id"], 61);
    assert_eq!(c["actions"][0]["name"], serde_json::Value::Null);
    assert_eq!(c["actions"][1]["name"], "ActionStartConversation");
    assert_eq!(c["actions"][1]["params"][1]["value"], "oaj_banter");
}

#[test]
fn without_the_corpus_locals_are_by_index_with_a_warning() {
    let tmp = tempfile::tempdir().unwrap();
    build(tmp.path());
    let (code, text, err) = show(
        tmp.path(),
        &tmp.path().join("absent"),
        &["--creature", "carth"],
    );
    assert_eq!(code, 0);
    assert!(err.contains("RW1201"), "{err}");
    assert!(text.contains("booleans    62\n"), "{text}");
}

#[test]
fn errors_carry_their_codes() {
    let tmp = tempfile::tempdir().unwrap();
    let (code, _, err) = show(tmp.path(), tmp.path(), &[]);
    assert_eq!(code, 1);
    assert!(err.contains("RE1201"), "{err}");
    build(tmp.path());
    let (code, _, err) = show(tmp.path(), tmp.path(), &["--module", "danm15"]);
    assert_eq!(code, 1);
    assert!(err.contains("RE1202") && err.contains("danm14ac"), "{err}");
    let (code, _, err) = show(tmp.path(), &corpus(tmp.path()), &["--creature", "bastila"]);
    assert_eq!(code, 1);
    assert!(err.contains("RE1204"), "{err}");
}

/// The user's own saves: every save with a module's creature list is
/// searched by tag in every case form. The three Juhani findings are checked
/// by hand (`docs/formats/save.md`), since what a save holds is the player's.
#[test]
fn the_users_saves_answer_by_tag() {
    let Ok(root) = std::env::var("RAKATA_SAVES") else {
        eprintln!("skipped: $RAKATA_SAVES is unset; set it to a folder of save-game directories");
        return;
    };
    let mut n = 0;
    for e in std::fs::read_dir(&root).unwrap() {
        let dir = e.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        let out = rakata()
            .args(["--format", "json", "save", "show"])
            .arg(&dir)
            .args(["--creature", "MISSION"])
            .output()
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let found = doc["result"]["creatures"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if !found.is_empty() {
            n += 1;
            assert!(
                found
                    .iter()
                    .all(|c| c["tag"].as_str().unwrap().eq_ignore_ascii_case("mission"))
            );
        }
    }
    eprintln!("{n} saves hold a creature tagged mission");
}
