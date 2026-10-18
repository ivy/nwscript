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

// `rakata save set`.

/// An install whose `Override/globalcat.2da` declares the fixture save's
/// globals, and `NEW_DECL`, which the save lacks.
fn install(dir: &Path) -> PathBuf {
    let game = dir.join("game");
    std::fs::create_dir_all(game.join("Override")).unwrap();
    std::fs::write(
        game.join("chitin.key"),
        rakata_archive::Key::new().to_bytes().unwrap(),
    )
    .unwrap();
    let mut t = rakata::twoda::TwoDa::new(["name", "type"]);
    for (i, (name, ty)) in [
        ("OAJ_DONE", "Boolean"),
        ("OAJ_DOWN_0", "Boolean"),
        ("TAR_X", "Boolean"),
        ("OAJ_COUNT", "Number"),
        ("NEW_DECL", "Number"),
    ]
    .iter()
    .enumerate()
    {
        t.add_row(i.to_string(), &[("name", name), ("type", ty)])
            .unwrap();
    }
    std::fs::write(game.join("Override/globalcat.2da"), t.to_bytes().unwrap()).unwrap();
    game
}

/// A save in `DIR/save` and an install in `DIR/game`.
fn fixture(dir: &Path) -> (PathBuf, PathBuf) {
    let src = dir.join("save");
    std::fs::create_dir(&src).unwrap();
    build(&src);
    (src, install(dir))
}

fn set(game: &Path, src: &Path, out: &Path, args: &[&str]) -> (i32, String, String) {
    let o = rakata()
        .arg("--game")
        .arg(game)
        .args(["save", "set"])
        .arg(src)
        .arg("--out")
        .arg(out)
        .args(args)
        .output()
        .unwrap();
    (
        o.status.code().unwrap(),
        String::from_utf8(o.stdout).unwrap(),
        String::from_utf8(o.stderr).unwrap(),
    )
}

/// The files of a save directory, by name, with their bytes.
fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read(&p).unwrap())
        })
        .collect();
    out.sort();
    out
}

/// Every file and every GFF member of the two saves is byte for byte the
/// same, but for the files named in `changed`; those the function returns,
/// each with the number of bytes that differ.
fn compare(src: &Path, dst: &Path, changed: &[&str]) -> Vec<(String, usize)> {
    let (a, b) = (files(src), files(dst));
    assert_eq!(
        a.iter().map(|f| &f.0).collect::<Vec<_>>(),
        b.iter().map(|f| &f.0).collect::<Vec<_>>(),
        "the same files"
    );
    let mut differ = Vec::new();
    for ((name, x), (_, y)) in a.iter().zip(&b) {
        if changed.iter().any(|c| c.eq_ignore_ascii_case(name)) {
            let n = x.iter().zip(y).filter(|(p, q)| p != q).count() + x.len().abs_diff(y.len());
            differ.push((name.clone(), n));
        } else {
            assert!(x == y, "{name} differs");
        }
    }
    let (sa, sb) = (
        rakata::save::Save::open(src).unwrap(),
        rakata::save::Save::open(dst).unwrap(),
    );
    let (ma, mb) = (sa.gff_members().unwrap(), sb.gff_members().unwrap());
    assert_eq!(ma.len(), mb.len());
    for (x, y) in ma.iter().zip(&mb) {
        assert_eq!(x.location, y.location);
        if !changed.iter().any(|c| c.eq_ignore_ascii_case(&x.location)) {
            assert!(x.bytes == y.bytes, "member {} differs", x.location);
        }
    }
    differ
}

#[test]
fn set_changes_one_boolean_and_copies_everything_else() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("000001 - Game0");
    std::fs::create_dir(&src).unwrap();
    build(&src);
    let game = install(tmp.path());
    let dst = tmp.path().join("000002 - Game1");

    // OAJ_DONE, OAJ_DOWN_0 and TAR_X share one byte, 1010_0000.
    let (code, text, err) = set(&game, &src, &dst, &["oaj_down_0=true"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        text.contains("global   OAJ_DOWN_0  boolean  false -> true"),
        "{text}"
    );
    assert!(
        text.contains("field    GLOBALVARS.res ValBoolean byte 0: 0xa0 -> 0xe0"),
        "{text}"
    );
    assert!(
        text.contains("rewrote GLOBALVARS.res; copied PARTYTABLE.res, SAVEGAME.sav, savenfo.res"),
        "{text}"
    );
    assert_eq!(
        compare(&src, &dst, &["GLOBALVARS.res"]),
        [("GLOBALVARS.res".to_owned(), 1)]
    );

    let (code, text, err) = show(&dst, &corpus(tmp.path()), &["--globals", "_"]);
    assert_eq!(code, 0, "{err}");
    assert!(text.contains("boolean  OAJ_DOWN_0  true"), "{text}");
    assert!(text.contains("boolean  OAJ_DONE    true"), "{text}");
    assert!(text.contains("boolean  TAR_X       true"), "{text}");
}

#[test]
fn an_undeclared_global_is_refused_by_name_and_declare_writes_it() {
    let tmp = tempfile::tempdir().unwrap();
    let (src, game) = fixture(tmp.path());
    let dst = tmp.path().join("out");

    let (code, _, err) = set(&game, &src, &dst, &["OAJ_NEW=boolean:1"]);
    assert_eq!(code, 1);
    assert!(err.contains("RE1205") && err.contains("`OAJ_NEW`"), "{err}");
    assert!(!dst.exists(), "nothing is written when an edit is refused");

    let (code, text, err) = set(&game, &src, &dst, &["OAJ_NEW=boolean:1", "--declare"]);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("RW1202") && err.contains("`OAJ_NEW`"), "{err}");
    assert!(
        text.contains("OAJ_NEW  boolean  (added to the save) -> true"),
        "{text}"
    );
    let (_, text, _) = show(&dst, &corpus(tmp.path()), &["--globals", "oaj_new"]);
    assert!(text.contains("boolean  OAJ_NEW  true"), "{text}");

    // A name the install declares and the save lacks is added without
    // --declare, with the install's type.
    let dst2 = tmp.path().join("out2");
    let (code, text, err) = set(&game, &src, &dst2, &["new_decl=-5"]);
    assert_eq!(code, 0, "{err}");
    assert!(!err.contains("RW1202"), "{err}");
    assert!(
        text.contains("NEW_DECL  number   (added to the save) -> -5"),
        "{text}"
    );
}

#[test]
fn party_rewrites_only_the_party_table() {
    let tmp = tempfile::tempdir().unwrap();
    let (src, game) = fixture(tmp.path());
    let dst = tmp.path().join("out");
    let (code, text, err) = set(&game, &src, &dst, &["--party", ""]);
    assert_eq!(code, 0, "{err}");
    assert!(
        text.contains("party    AVAILNPC6 -> (the player alone)"),
        "{text}"
    );
    let differ = compare(&src, &dst, &["PARTYTABLE.res"]);
    assert!(differ[0].1 > 0);
    let (_, text, _) = show(&dst, &corpus(tmp.path()), &[]);
    assert!(text.contains("party    (the player alone)"), "{text}");

    let dst2 = tmp.path().join("out2");
    let (code, _, err) = set(&game, &dst, &dst2, &["--party", "MISSION"]);
    assert_eq!(code, 0, "{err}");
    let (_, text, _) = show(&dst2, &corpus(tmp.path()), &[]);
    assert!(text.contains("party    mission (AVAILNPC6)"), "{text}");
}

#[test]
fn set_errors_carry_their_codes() {
    let tmp = tempfile::tempdir().unwrap();
    let (src, game) = fixture(tmp.path());
    let dst = tmp.path().join("out");
    for (args, code) in [
        (&["OAJ_DONE=maybe"][..], "RE1206"),
        (&["OAJ_COUNT=300"], "RE1206"),
        (&["OAJ_DONE=number:1"], "RE1206"),
        (&["OAJ_NEW=1", "--declare"], "RE1206"),
        (&["--party", "bastila"], "RE1207"),
        (&["--party", "6,mission"], "RE1208"),
    ] {
        let (status, _, err) = set(&game, &src, &dst, args);
        assert_eq!(status, 1, "{args:?}");
        assert!(err.contains(code), "{args:?}: {err}");
    }
    std::fs::create_dir(&dst).unwrap();
    std::fs::write(dst.join("x"), b"x").unwrap();
    let (status, _, err) = set(&game, &src, &dst, &["OAJ_DONE=0"]);
    assert_eq!(status, 1);
    assert!(err.contains("RE1209"), "{err}");
    let (status, _, err) = set(&game, &src, &src, &["OAJ_DONE=0"]);
    assert_eq!(status, 1);
    assert!(err.contains("RE1209"), "{err}");
    let (status, _, err) = set(&game, &src, &src.join("inner"), &["OAJ_DONE=0"]);
    assert_eq!(status, 1);
    assert!(err.contains("RE1209"), "{err}");

    // Without the install, or without anything to set, it is a usage error.
    let o = rakata()
        .args(["save", "set"])
        .arg(&src)
        .args(["--out", "x", "OAJ_DONE=0"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    let (status, _, _) = set(&game, &src, &tmp.path().join("y"), &[]);
    assert_eq!(status, 2);
}

#[test]
fn set_json_reports_the_globals_and_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let (src, game) = fixture(tmp.path());
    let dst = tmp.path().join("out");
    let o = rakata()
        .arg("--game")
        .arg(&game)
        .args(["--format", "json", "save", "set"])
        .arg(&src)
        .arg("--out")
        .arg(&dst)
        .args(["OAJ_COUNT=12", "OAJ_DONE=false"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0));
    let doc: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(doc["command"], "save set");
    let r = &doc["result"];
    assert_eq!(r["globals"][0]["name"], "OAJ_COUNT");
    assert_eq!(r["globals"][0]["before"], -1);
    assert_eq!(r["globals"][0]["after"], 12);
    assert_eq!(r["globals"][1]["after"], false);
    assert_eq!(r["rewritten"][0], "GLOBALVARS.res");
    let paths: Vec<_> = r["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(paths, ["ValBoolean", "ValNumber"]);
    assert_eq!(r["fields"][1]["bytes"][0]["before"], 0xFF);
    assert_eq!(r["fields"][1]["bytes"][0]["after"], 12);
}

/// The user's own saves, written through the user's build: one boolean
/// flipped changes one byte of `GLOBALVARS.res` and nothing else; a party
/// set changes `PARTYTABLE.res` and nothing else.
#[test]
fn the_users_saves_change_only_what_is_set() {
    let (Ok(root), Ok(game)) = (std::env::var("RAKATA_SAVES"), std::env::var("RAKATA_BUILD"))
    else {
        eprintln!(
            "skipped: $RAKATA_SAVES or $RAKATA_BUILD is unset; set them to a folder of save-game directories and the build they were made with"
        );
        return;
    };
    let game = PathBuf::from(game);
    let tmp = tempfile::tempdir().unwrap();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for (i, src) in dirs.iter().enumerate() {
        let save = rakata::save::Save::open(src).unwrap();
        let globals = save.globals().unwrap();
        let (name, was) = globals
            .vars
            .iter()
            .find_map(|g| match g.value {
                rakata::save::GlobalValue::Boolean(b) if g.name.starts_with("OAJ_") => {
                    Some((g.name.clone(), b))
                }
                _ => None,
            })
            .or_else(|| {
                globals.vars.iter().find_map(|g| match g.value {
                    rakata::save::GlobalValue::Boolean(b) => Some((g.name.clone(), b)),
                    _ => None,
                })
            })
            .expect("a save holds a boolean");
        let dst = tmp.path().join(format!("g{i}"));
        let (code, text, err) = set(&game, src, &dst, &[&format!("{name}={}", !was)]);
        assert_eq!(code, 0, "{}: {err}", src.display());
        let differ = compare(src, &dst, &["GLOBALVARS.res"]);
        assert_eq!(differ, [("GLOBALVARS.res".to_owned(), 1)], "{text}");
        let after = rakata::save::Save::open(&dst).unwrap().globals().unwrap();
        assert_eq!(
            after.get(&name).unwrap().value,
            rakata::save::GlobalValue::Boolean(!was)
        );
        let flipped = globals
            .vars
            .iter()
            .zip(&after.vars)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(flipped, 1, "only {name} changes");
        eprintln!(
            "{}: {name} {was} -> {}; {} files and {} GFF members compared, GLOBALVARS.res differs in 1 byte",
            src.display(),
            !was,
            files(src).len(),
            save.gff_members().unwrap().len()
        );

        let npc = save.party_gffs().unwrap()[0].0;
        let dst = tmp.path().join(format!("p{i}"));
        let (code, _, err) = set(&game, src, &dst, &["--party", &npc.to_string()]);
        assert_eq!(code, 0, "{}: {err}", src.display());
        compare(src, &dst, &["PARTYTABLE.res"]);
        let party = rakata::save::Save::open(&dst).unwrap().party().unwrap();
        assert_eq!(
            party.members.iter().map(|m| m.npc).collect::<Vec<_>>(),
            [i64::from(npc)]
        );
    }
}
