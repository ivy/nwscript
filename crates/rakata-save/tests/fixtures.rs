//! A save made here, no game bytes: a `SAVEGAME.sav` holding the globals,
//! the party table, a party member, the factions and one module's state,
//! written to a directory and read by name.

use std::path::PathBuf;

use rakata_archive::{Erf, ErfKind};
use rakata_gff::{Gff, Label, Struct, Value};
use rakata_res::{ResId, ResRef, ResType};
use rakata_save::{GlobalValue, Param, Save};

/// `bytes`, kept as a seed for the fuzz targets in the directory
/// `$RAKATA_FUZZ_SEEDS` names, when it is set.
fn seed(bytes: &[u8]) -> &[u8] {
    if let Some(dir) = std::env::var_os("RAKATA_FUZZ_SEEDS") {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut h = DefaultHasher::new();
        bytes.hash(&mut h);
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{:016x}", h.finish())), bytes).unwrap();
    }
    bytes
}

fn l(t: &str) -> Label {
    Label::new(t).unwrap()
}

fn id(name: &str, ty: ResType) -> ResId {
    ResId::new(ResRef::new(name).unwrap(), ty)
}

fn with(fields: &[(&str, Value)]) -> Struct {
    let mut s = Struct::new(0);
    for (label, value) in fields {
        s.push(l(label), value.clone());
    }
    s
}

fn gff(ty: &[u8; 4], fields: &[(&str, Value)]) -> Vec<u8> {
    let mut g = Gff::new(*ty);
    g.root = with(fields);
    g.to_bytes().unwrap()
}

/// Two booleans, B0 set, and a number.
fn globals() -> Vec<u8> {
    let named = |n: &str| with(&[("Name", Value::string(n))]);
    gff(
        b"GVT ",
        &[
            ("CatBoolean", Value::List(vec![named("B0"), named("B1")])),
            ("CatNumber", Value::List(vec![named("N0")])),
            ("CatLocation", Value::List(vec![])),
            ("CatString", Value::List(vec![])),
            ("ValBoolean", Value::Void(vec![0x80])),
            ("ValNumber", Value::Void(vec![7])),
            ("ValLocation", Value::Void(vec![])),
            ("ValString", Value::List(vec![])),
        ],
    )
}

fn creature(tag: &str, object_id: u32) -> Struct {
    let param = with(&[("Type", Value::Dword(3)), ("Value", Value::Dword(1))]);
    let action = with(&[
        ("ActionId", Value::Dword(24)),
        ("Paramaters", Value::List(vec![param])),
    ]);
    with(&[
        ("Tag", Value::string(tag)),
        ("ObjectId", Value::Dword(object_id)),
        ("FactionID", Value::Word(0)),
        ("ActionList", Value::List(vec![action])),
    ])
}

/// The save's `SAVEGAME.sav`.
fn savegame() -> Vec<u8> {
    let event = with(&[
        ("Day", Value::Dword(1)),
        ("ObjectId", Value::Dword(2)),
        ("EventId", Value::Dword(10)),
        (
            "EventData",
            Value::Struct(with(&[("Name", Value::string("k_ev"))])),
        ),
    ]);
    let mut module = Erf::new(ErfKind::Sav);
    module
        .add(
            id("module", ResType::Ifo),
            gff(
                b"IFO ",
                &[
                    ("Mod_PlayerList", Value::List(vec![creature("player", 1)])),
                    ("EventQueue", Value::List(vec![event])),
                ],
            ),
        )
        .unwrap();
    module
        .add(
            id("m14aa", ResType::Git),
            gff(
                b"GIT ",
                &[("Creature List", Value::List(vec![creature("mission", 2)]))],
            ),
        )
        .unwrap();

    let member = with(&[
        ("PT_MEMBER_ID", Value::Int(0)),
        ("PT_IS_LEADER", Value::Byte(0)),
    ]);
    let avail = with(&[
        ("PT_NPC_AVAIL", Value::Byte(1)),
        ("PT_NPC_SELECT", Value::Byte(1)),
    ]);
    let mut outer = Erf::new(ErfKind::Sav);
    outer
        .add(id("GLOBALVARS", ResType::Res), globals())
        .unwrap();
    outer
        .add(
            id("PARTYTABLE", ResType::Res),
            gff(
                b"PT  ",
                &[
                    ("PT_MEMBERS", Value::List(vec![member])),
                    ("PT_AVAIL_NPCS", Value::List(vec![avail])),
                    ("PT_GOLD", Value::Dword(100)),
                ],
            ),
        )
        .unwrap();
    outer
        .add(
            id("AVAILNPC0", ResType::Utc),
            gff(b"UTC ", &[("Tag", Value::string("carth"))]),
        )
        .unwrap();
    outer
        .add(
            id("REPUTE", ResType::Fac),
            gff(
                b"FAC ",
                &[(
                    "FactionList",
                    Value::List(vec![with(&[("FactionName", Value::string("PC"))])]),
                )],
            ),
        )
        .unwrap();
    outer
        .add(id("danm14aa", ResType::Sav), module.to_bytes().unwrap())
        .unwrap();
    outer.to_bytes().unwrap()
}

#[test]
fn a_save_made_by_hand_reads_by_name() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rakata-save-fixture");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SAVEGAME.sav"), seed(&savegame())).unwrap();
    let save = Save::open(&dir).unwrap();

    let g = save.globals().unwrap();
    assert_eq!(g.get("b0").unwrap().value, GlobalValue::Boolean(true));
    assert_eq!(g.get("B1").unwrap().value, GlobalValue::Boolean(false));
    assert_eq!(g.get("N0").unwrap().value, GlobalValue::Number(7));

    let party = save.party().unwrap();
    assert_eq!(party.members.len(), 1);
    assert_eq!(party.members[0].tag.as_deref(), Some("carth"));
    assert_eq!(party.gold, Some(100));

    assert_eq!(save.modules().unwrap(), ["danm14aa"]);
    let tags: Vec<_> = save
        .creatures(None)
        .unwrap()
        .into_iter()
        .map(|c| (c.tag, c.faction_name))
        .collect();
    assert_eq!(
        tags,
        [
            ("mission".to_owned(), Some("PC".to_owned())),
            ("player".to_owned(), Some("PC".to_owned())),
            ("carth".to_owned(), None),
        ]
    );
    let mission = &save.find_creatures("MISSION", None).unwrap()[0];
    let Param::Object(target) = &mission.actions[0].params[0] else {
        panic!("an object parameter")
    };
    assert_eq!(target.tag.as_deref(), Some("player"));

    let events = save.events("danm14aa").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].script.as_deref(), Some("k_ev"));
    assert_eq!(events[0].object.tag.as_deref(), Some("mission"));
    assert_eq!(save.gff_members().unwrap().len(), 6);
}
