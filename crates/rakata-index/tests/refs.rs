//! `refs` over a tree made here: the area-transition chain in miniature
//! (a strref in a dialogue, the dialogue a placeable's conversation, the
//! placeable found by its tag from a script) and a global read by a
//! dialogue condition, in both directions, with shadowed copies marked.

use std::fs;
use std::path::{Path, PathBuf};

use rakata_2da::TwoDa;
use rakata_archive::{Erf, ErfKind, Rim};
use rakata_gff::{Gff, Label, LocString, Struct, Value};
use rakata_index::{
    ArgType, BuildOptions, Direction, Param, RefKind, RefsQuery, Routine, build, codes, open, refs,
};
use rakata_res::{ResId, ResRef};

fn label(s: &str) -> Label {
    Label::new(s).unwrap()
}

fn resref(s: &str) -> Value {
    Value::ResRef(ResRef::new(s).unwrap())
}

/// A placeable whose conversation is `k_transabort`.
fn utp() -> Vec<u8> {
    let mut g = Gff::new(*b"UTP ");
    g.root
        .push(label("Tag"), Value::string("k_trans_abort"))
        .push(label("TemplateResRef"), resref("k_trans_abort"))
        .push(label("Conversation"), resref("k_transabort"));
    g.to_bytes().unwrap()
}

/// A dialogue: one entry, one reply with strref `strref`, the link to it
/// gated by the condition `k_c_done`.
fn dlg(strref: u32) -> Vec<u8> {
    let mut g = Gff::new(*b"DLG ");
    let mut link = Struct::new(0);
    link.push(label("Index"), Value::Dword(0))
        .push(label("Active"), resref("k_c_done"));
    let mut entry = Struct::new(0);
    entry
        .push(label("Speaker"), Value::string(""))
        .push(label("Text"), Value::LocString(LocString::none()))
        .push(label("RepliesList"), Value::List(vec![link]));
    let mut reply = Struct::new(0);
    reply
        .push(label("Text"), Value::LocString(LocString::strref(strref)))
        .push(label("EntriesList"), Value::List(Vec::new()));
    let mut start = Struct::new(0);
    start.push(label("Index"), Value::Dword(0));
    g.root
        .push(label("EntryList"), Value::List(vec![entry]))
        .push(label("ReplyList"), Value::List(vec![reply]))
        .push(label("StartingList"), Value::List(vec![start]));
    g.to_bytes().unwrap()
}

fn ncs(asm: &str) -> Vec<u8> {
    let p = nwscript_ncs::asm::parse(nwscript_diag::FileId(0), asm.as_bytes()).unwrap();
    nwscript_ncs::encode(&p)
}

fn routines() -> Vec<Routine> {
    let p = |name: &str, ty| Param {
        name: name.into(),
        ty,
    };
    vec![
        Routine {
            name: "GetNearestObjectByTag".into(),
            params: vec![
                p("sTag", ArgType::String),
                p("oTarget", ArgType::Object),
                p("nNth", ArgType::Int),
            ],
            ret: ArgType::Object,
        },
        Routine {
            name: "GetGlobalBoolean".into(),
            params: vec![p("sIdentifier", ArgType::String)],
            ret: ArgType::Int,
        },
        Routine {
            name: "GetFirstPC".into(),
            params: vec![],
            ret: ArgType::Object,
        },
        Routine {
            name: "GetItemPossessedBy".into(),
            params: vec![
                p("oCreature", ArgType::Object),
                p("sItemTag", ArgType::String),
            ],
            ret: ArgType::Object,
        },
        Routine {
            name: "SetLocalString".into(),
            params: vec![
                p("oObject", ArgType::Object),
                p("sVarName", ArgType::String),
                p("sValue", ArgType::String),
            ],
            ret: ArgType::Void,
        },
    ]
}

fn write(root: &Path, rel: &str, bytes: &[u8]) -> PathBuf {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, bytes).unwrap();
    p
}

/// The tree, and the files of its walk.
fn tree(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut cat = TwoDa::new(["name", "type"]);
    cat.add_row("0", &[("name", "OAJ_DONE"), ("type", "Boolean")])
        .unwrap();
    files.push(write(
        root,
        "Override/globalcat.2da",
        &cat.to_bytes().unwrap(),
    ));
    files.push(write(root, "Override/k_transabort.dlg", &dlg(42724)));
    files.push(write(
        root,
        "Override/k_trg_transfail.ncs",
        &ncs("JSR main\nRETN\nmain:\nCONSTI 1\nCONSTO 0\nCONSTS \"k_trans_abort\"\nACTION 0, 3\nMOVSP -4\nRETN\n"),
    ));
    files.push(write(
        root,
        "Override/k_c_done.ncs",
        &ncs("JSR main\nRETN\nmain:\nCONSTS \"OAJ_DONE\"\nACTION 1, 1\nMOVSP -4\nRETN\n"),
    ));
    // The module: its .mod replaces its .rim; the .mod's dialogue is
    // shadowed by the Override one.
    let mut m = Erf::new(ErfKind::Mod);
    m.add(ResId::parse("k_trans_abort.utp").unwrap(), utp())
        .unwrap();
    m.add(ResId::parse("k_transabort.dlg").unwrap(), dlg(42725))
        .unwrap();
    files.push(write(root, "modules/m01aa.mod", &m.to_bytes().unwrap()));
    let mut r = Rim::new();
    r.add(ResId::parse("k_trans_abort.utp").unwrap(), utp())
        .unwrap();
    files.push(write(root, "modules/m01aa.rim", &r.to_bytes().unwrap()));
    files.sort();
    files
}

fn q(thing: &str, kind: Option<RefKind>, direction: Direction) -> RefsQuery<'_> {
    RefsQuery {
        thing,
        kind,
        direction,
        tree: None,
        direct: false,
    }
}

#[test]
fn the_transition_chain_and_a_global_in_both_directions() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("game");
    let files = tree(&root);
    let db = dir.path().join("idx.sqlite");
    let routines = routines();
    build(&BuildOptions {
        root: &root,
        files: &files,
        db: &db,
        label: "t",
        parent: None,
        changed: None,
        routines: &routines,
        threads: 1,
    })
    .unwrap();
    let conn = open(&db).unwrap();

    // The strref names the Override dialogue only: the module's copy
    // carries another line.
    let a = refs(&conn, &q("42724", Some(RefKind::StrRef), Direction::To)).unwrap();
    assert_eq!(a.matched, ["strref"]);
    assert_eq!(a.hits.len(), 1, "{a:?}");
    assert_eq!(a.hits[0].resource, "k_transabort.dlg");
    assert_eq!(a.hits[0].site, "ReplyList[0].Text");
    assert_eq!(a.hits[0].locations[0].path, "Override/k_transabort.dlg");
    let a = refs(&conn, &q("42725", None, Direction::To)).unwrap();
    assert_eq!(
        a.hits[0].locations[0].shadowed_by.as_deref(),
        Some("Override/k_transabort.dlg")
    );

    // The dialogue is the placeable's conversation, in the .mod and in the
    // .rim the .mod replaces.
    let a = refs(
        &conn,
        &q("k_transabort", Some(RefKind::ResRef), Direction::To),
    )
    .unwrap();
    assert_eq!(a.hits.len(), 1, "{a:?}");
    let h = &a.hits[0];
    assert_eq!(
        (h.resource.as_str(), h.site.as_str()),
        ("k_trans_abort.utp", "Conversation")
    );
    let places: Vec<(&str, Option<&str>)> = h
        .locations
        .iter()
        .map(|l| (l.path.as_str(), l.shadowed_by.as_deref()))
        .collect();
    assert_eq!(
        places,
        [
            ("modules/m01aa.mod", None),
            ("modules/m01aa.rim", Some("modules/m01aa.mod"))
        ]
    );

    // The placeable is found by its tag: the script, through the tag.
    let a = refs(
        &conn,
        &q("k_trans_abort", Some(RefKind::ResRef), Direction::To),
    )
    .unwrap();
    let script = a
        .hits
        .iter()
        .find(|h| h.resource == "k_trg_transfail.ncs")
        .unwrap_or_else(|| panic!("{a:?}"));
    assert_eq!(script.site, "@50 GetNearestObjectByTag.sTag");
    assert_eq!(script.operand, Some("CONSTS"));
    assert_eq!(
        script.via.as_deref(),
        Some("tag k_trans_abort of k_trans_abort.utp")
    );
    let mut direct = q("k_trans_abort", Some(RefKind::ResRef), Direction::To);
    direct.direct = true;
    let a = refs(&conn, &direct).unwrap();
    assert!(a.hits.iter().all(|h| h.via.is_none()), "{a:?}");

    // A global: its declaration, the condition script that reads it, and
    // through that script the dialogue link it gates.
    let a = refs(&conn, &q("oaj_done", None, Direction::To)).unwrap();
    assert_eq!(a.tried, ["resref", "tag", "global"]);
    assert_eq!(a.matched, ["global"]);
    let got: Vec<(&str, &str, Option<&str>)> = a
        .hits
        .iter()
        .map(|h| (h.resource.as_str(), h.site.as_str(), h.via.as_deref()))
        .collect();
    assert_eq!(
        got,
        [
            ("globalcat.2da", "row 0", None),
            ("k_c_done.ncs", "@33 GetGlobalBoolean.sIdentifier", None),
            (
                "k_transabort.dlg",
                "EntryList[0].RepliesList[0].Active",
                Some("script k_c_done")
            ),
            (
                "k_transabort.dlg",
                "EntryList[0].RepliesList[0].Active",
                Some("script k_c_done")
            ),
        ]
    );

    // From the script: what it names.
    let a = refs(&conn, &q("K_TRG_TRANSFAIL.ncs", None, Direction::From)).unwrap();
    let got: Vec<(&str, &str)> = a
        .hits
        .iter()
        .map(|h| (h.kind.as_str(), h.value.as_str()))
        .collect();
    assert_eq!(got, [("tag", "k_trans_abort")]);

    // Nothing, and the errors.
    let a = refs(&conn, &q("k_nothere", None, Direction::To)).unwrap();
    assert!(a.hits.is_empty() && a.matched.is_empty());
    let e = refs(&conn, &q("abc", Some(RefKind::StrRef), Direction::To)).unwrap_err();
    assert_eq!(e.code(), codes::SYMBOL);
    let e = refs(&conn, &q("nope.ncs", None, Direction::From)).unwrap_err();
    assert_eq!(e.code(), codes::NO_SUCH_RESOURCE);
}
