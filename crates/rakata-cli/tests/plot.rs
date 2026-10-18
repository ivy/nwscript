//! `rakata flags` and `rakata quest` over an install made here, and over
//! the vanilla game when `$RAKATA_GAME_K1` names it. No game bytes are
//! committed.

use std::path::Path;

use assert_cmd::Command;
use rakata::twoda::TwoDa;
use rakata_archive::{Erf, ErfKind, Key};
use rakata_gff::{Gff, Label, LocString, Struct, Value};
use serde_json::Value as Json;

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn l(t: &str) -> Label {
    Label::new(t).unwrap()
}

fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

/// The engine's routines the scripts below call, in this order.
const NWSCRIPT: &str = "\
int GetLocalBoolean(object oObject, int nIndex);
void SetLocalBoolean(object oObject, int nIndex, int nValue);
object GetObjectByTag(string sTag, int nNth=0);
void AddJournalQuestEntry(string szPlotID, int nState, int bAllowOverrideHigher=FALSE);
void SetGlobalNumber(string sIdentifier, int nValue);
void RemoveJournalQuestEntry(string szPlotID);
";

fn ncs(asm: &str) -> Vec<u8> {
    let p = nwscript_ncs::asm::parse(nwscript_diag::FileId(0), asm.as_bytes()).unwrap();
    nwscript_ncs::encode(&p)
}

/// `int SW_PLOT_BOOLEAN_01 = 0;` as a global, and
/// `UT_SetPlotBooleanFlag(OBJECT_SELF, SW_PLOT_BOOLEAN_01, value)` through
/// a wrapper of the script's own, as `k_inc_utility` compiles.
fn set_flag(value: i32) -> Vec<u8> {
    ncs(&format!(
        "JSR globals\nRETN\nglobals:\nRSADDI\nCONSTI 0\nCPDOWNSP -8, 4\nMOVSP -4\nSAVEBP\nJSR main\nRESTOREBP\nMOVSP -4\nRETN\nmain:\nCONSTI {value}\nCPTOPBP -4, 4\nCONSTO 0\nJSR wrap\nRETN\nwrap:\nCPTOPSP -12, 4\nCPTOPSP -12, 4\nCPTOPSP -12, 4\nACTION 1, 3\nMOVSP -12\nRETN\n"
    ))
}

fn set_global(name: &str, value: i32) -> String {
    format!("CONSTI {value}\nCONSTS \"{name}\"\nACTION 4, 2\n")
}

fn script(body: &str) -> Vec<u8> {
    ncs(&format!("JSR main\nRETN\nmain:\n{body}RETN\n"))
}

struct Node<'a> {
    script: &'a str,
    quest: Option<(&'a str, u32)>,
}

/// A dialogue of one entry offering `replies`, each link with its
/// condition.
fn dlg(replies: &[(Node, &str)]) -> Vec<u8> {
    let mut g = Gff::new(*b"DLG ");
    let node = |n: &Node, links: Vec<Struct>, list: &str| {
        let mut st = Struct::new(0);
        st.push(l("Text"), Value::LocString(LocString::none()))
            .push(l("Script"), Value::resref(n.script).unwrap())
            .push(l("Quest"), Value::string(n.quest.map_or("", |q| q.0)));
        if let Some((_, e)) = n.quest {
            st.push(l("QuestEntry"), Value::Dword(e));
        }
        st.push(l(list), Value::List(links));
        st
    };
    let link = |i: u32, active: &str| {
        let mut st = Struct::new(0);
        st.push(l("Index"), Value::Dword(i))
            .push(l("Active"), Value::resref(active).unwrap());
        st
    };
    let entry = node(
        &Node {
            script: "",
            quest: None,
        },
        replies
            .iter()
            .enumerate()
            .map(|(i, (_, c))| link(i as u32, c))
            .collect(),
        "RepliesList",
    );
    let reply_nodes: Vec<Struct> = replies
        .iter()
        .map(|(n, _)| node(n, Vec::new(), "EntriesList"))
        .collect();
    g.root
        .push(l("EntryList"), Value::List(vec![entry]))
        .push(l("ReplyList"), Value::List(reply_nodes))
        .push(l("StartingList"), Value::List(vec![link(0, "")]));
    g.to_bytes().unwrap()
}

fn blueprint(kind: &[u8; 4], fields: &[(&str, &str)]) -> Vec<u8> {
    let mut g = Gff::new(*kind);
    for (k, v) in fields {
        let v = if *k == "Tag" {
            Value::string(v)
        } else {
            Value::resref(v).unwrap()
        };
        g.root.push(l(k), v);
    }
    g.to_bytes().unwrap()
}

fn journal() -> Vec<u8> {
    let mut g = Gff::new(*b"JRL ");
    let entries: Vec<Struct> = [(10, 0), (20, 1), (30, 0)]
        .iter()
        .map(|(id, end)| {
            let mut e = Struct::new(0);
            e.push(l("ID"), Value::Dword(*id))
                .push(l("End"), Value::Word(*end))
                .push(l("Text"), Value::LocString(LocString::strref(*id)));
            e
        })
        .collect();
    let mut cat = Struct::new(0);
    cat.push(l("Name"), Value::LocString(LocString::strref(1)))
        .push(l("Tag"), Value::string("q_test"))
        .push(l("Comment"), Value::string(""))
        .push(l("EntryList"), Value::List(entries));
    g.root.push(l("Categories"), Value::List(vec![cat]));
    g.to_bytes().unwrap()
}

/// The install: a statue whose dialogue sets its flag 0 through the
/// wrapper, a guide whose dialogue reads it by tag and writes the quest,
/// a sibling reply that sets the same global without writing it, a write
/// to an entry the journal lacks, and an area that removes the quest.
fn install(root: &Path) {
    write(root, "chitin.key", &Key::new().to_bytes().unwrap());
    write(root, "Override/nwscript.nss", NWSCRIPT.as_bytes());
    write(
        root,
        "Override/k_inc_utility.nss",
        b"int SW_PLOT_BOOLEAN_01 = 0;\nint SW_PLOT_BOOLEAN_02 = 1;\n",
    );
    let mut cat = TwoDa::new(["name", "type"]);
    cat.add_row("0", &[("name", "Q_STATE"), ("type", "Number")])
        .unwrap();
    write(root, "Override/globalcat.2da", &cat.to_bytes().unwrap());
    write(root, "Override/global.jrl", &journal());
    let mut m = Erf::new(ErfKind::Mod);
    let mut add = |name: &str, bytes: Vec<u8>| {
        m.add(rakata::res::ResId::parse(name).unwrap(), bytes)
            .unwrap();
    };
    add(
        "statue1.utp",
        blueprint(b"UTP ", &[("Tag", "statue1"), ("Conversation", "statue")]),
    );
    add(
        "statue.dlg",
        dlg(&[
            (
                Node {
                    script: "set_true",
                    quest: None,
                },
                "",
            ),
            (
                Node {
                    script: "set_false",
                    quest: None,
                },
                "",
            ),
        ]),
    );
    add("set_true.ncs", set_flag(1));
    add("set_false.ncs", set_flag(0));
    add(
        "guide.utc",
        blueprint(
            b"UTC ",
            &[
                ("Tag", "guide"),
                ("Conversation", "guide"),
                ("ScriptUserDefine", "journal_add"),
            ],
        ),
    );
    add(
        "guide.dlg",
        dlg(&[
            (
                Node {
                    script: "give_a",
                    quest: Some(("q_test", 10)),
                },
                "read_flag",
            ),
            (
                Node {
                    script: "give_b",
                    quest: None,
                },
                "",
            ),
            (
                Node {
                    script: "",
                    quest: Some(("q_test", 99)),
                },
                "",
            ),
        ]),
    );
    add(
        "read_flag.ncs",
        script("CONSTI 0\nCONSTI 0\nCONSTS \"statue1\"\nACTION 2, 2\nACTION 0, 2\nMOVSP -4\n"),
    );
    add("give_a.ncs", script(&set_global("Q_STATE", 1)));
    add("give_b.ncs", script(&set_global("Q_STATE", 3)));
    add(
        "journal_add.ncs",
        script(&format!(
            "CONSTI 0\nCONSTI 20\nCONSTS \"q_test\"\nACTION 3, 3\n{}",
            set_global("Q_STATE", 2)
        )),
    );
    add("remove.ncs", script("CONSTS \"q_test\"\nACTION 5, 1\n"));
    add(
        "test_area.are",
        blueprint(b"ARE ", &[("Tag", "test_area"), ("OnEnter", "remove")]),
    );
    write(root, "modules/test_m.mod", &m.to_bytes().unwrap());
}

fn run(args: &[&str]) -> (Json, String) {
    let out = rakata()
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        serde_json::from_slice(&out.stdout).unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

fn built() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let game = dir.path().join("game");
    install(&game);
    let db = dir.path().join("idx.sqlite");
    run(&[
        "index",
        "build",
        "--game",
        s(&game),
        "--db",
        s(&db),
        "--tree",
        "t",
    ]);
    (dir, db)
}

#[test]
fn flags_of_an_object_through_a_wrapper_by_owner_and_by_tag() {
    let (_dir, db) = built();
    let (j, _) = run(&["flags", "statue1", "--module", "test_m", "--db", s(&db)]);
    let r = &j["result"];
    assert_eq!(r["of"], "object");
    assert_eq!(r["objects"][0]["resource"], "statue1.utp");
    let f = &r["flags"][0];
    assert_eq!(f["kind"], "boolean");
    assert_eq!(f["index"], 0);
    assert_eq!(f["names"], serde_json::json!(["SW_PLOT_BOOLEAN_01"]));
    let writes: Vec<(String, i64)> = f["writes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| {
            (
                w["call"]["script"].as_str().unwrap().to_string(),
                w["value"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        writes,
        [("set_false".to_string(), 0), ("set_true".to_string(), 1)]
    );
    let w = &f["writes"][1];
    assert_eq!(w["object"]["kind"], "self_object");
    assert_eq!(w["call"]["path"].as_array().unwrap().len(), 2);
    assert_eq!(w["call"]["run_by"][0]["runner"]["dialog"], "statue");
    assert_eq!(w["call"]["run_by"][0]["runner"]["node"], "R0");
    assert_eq!(w["call"]["run_by"][0]["objects"][0]["tag"], "statue1");
    let read = &f["reads"][0];
    assert_eq!(read["call"]["script"], "read_flag");
    assert_eq!(
        read["object"],
        serde_json::json!({"kind": "tag", "value": "statue1"})
    );
    assert_eq!(read["call"]["run_by"][0]["runner"]["kind"], "condition");

    let out = rakata()
        .args(["flags", "statue1", "--db", s(&db)])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("boolean 0 (SW_PLOT_BOOLEAN_01)"), "{text}");
    assert!(
        text.contains("= 1  set_true @") && text.contains("(OBJECT_SELF, run by statue.dlg R0)"),
        "{text}"
    );
    assert!(text.contains("read_flag @"), "{text}");
}

#[test]
fn flags_of_a_global_and_of_nothing() {
    let (_dir, db) = built();
    let (j, _) = run(&["flags", "q_state", "--db", s(&db)]);
    let r = &j["result"];
    assert_eq!(r["of"], "global");
    assert_eq!(r["thing"], "Q_STATE");
    let values: Vec<(String, i64)> = r["flags"][0]["writes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| {
            (
                w["call"]["script"].as_str().unwrap().to_string(),
                w["value"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        values,
        [
            ("give_a".to_string(), 1),
            ("give_b".to_string(), 3),
            ("journal_add".to_string(), 2)
        ]
    );
    let (j, _) = run(&["flags", "nobody", "--db", s(&db)]);
    assert_eq!(j["diagnostics"][0]["code"], "RW2104");
}

#[test]
fn a_quest_with_its_writers_remover_and_lints() {
    let (_dir, db) = built();
    let (j, _) = run(&["quest", "Q_TEST", "--db", s(&db)]);
    let q = &j["result"]["quests"][0];
    assert_eq!(q["tag"], "q_test");
    let ids: Vec<i64> = q["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, [10, 20, 30]);
    let w10 = &q["entries"][0]["writers"][0];
    assert_eq!(w10["kind"], "node");
    assert_eq!(w10["node"], "R0");
    assert_eq!(w10["conditions"], serde_json::json!([["E0", "read_flag"]]));
    assert_eq!(w10["globals"], serde_json::json!([["Q_STATE", 1]]));
    let w20 = &q["entries"][1]["writers"][0];
    assert_eq!(w20["kind"], "script");
    assert_eq!(w20["call"]["script"], "journal_add");
    assert_eq!(
        w20["call"]["run_by"][0]["runner"]["field"],
        "ScriptUserDefine"
    );
    assert_eq!(w20["globals"], serde_json::json!([["Q_STATE", 2]]));
    assert!(q["entries"][2]["writers"].as_array().unwrap().is_empty());
    assert_eq!(q["other_writes"][0][0], 99);
    assert_eq!(q["removed_by"][0]["script"], "remove");
    assert_eq!(
        q["removed_by"][0]["run_by"][0]["runner"]["resource"],
        "test_area.are"
    );
    let codes: Vec<&str> = j["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert_eq!(
        codes,
        ["RW2101", "RW2102", "RW2103"],
        "{}",
        j["diagnostics"]
    );
    let msg = j["diagnostics"][2]["message"].as_str().unwrap();
    assert!(msg.contains("guide.dlg R1 (runs give_b)"), "{msg}");

    let (j, _) = run(&["quest", "--all", "--db", s(&db)]);
    assert_eq!(j["result"]["categories"], 1);
    assert_eq!(j["diagnostics"].as_array().unwrap().len(), 3);
    let out = rakata()
        .args(["--format", "json", "quest", "nope", "--db", s(&db)])
        .output()
        .unwrap();
    let j: Json = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(j["diagnostics"][0]["code"], "RE2102");
    assert_eq!(out.status.code(), Some(1));
}

/// The field report's two traces over an index of the vanilla game.
#[test]
fn the_sword_quest_over_vanilla() {
    let Some(game) = std::env::var_os("RAKATA_GAME_K1") else {
        eprintln!("skipped: RAKATA_GAME_K1 does not name a vanilla KotOR 1 install");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("idx.sqlite");
    run(&[
        "index",
        "build",
        "--game",
        game.to_str().unwrap(),
        "--db",
        s(&db),
        "--tree",
        "vanilla",
    ]);
    let (j, _) = run(&[
        "flags",
        "k37_inv_statue",
        "--module",
        "korr_m37aa",
        "--db",
        s(&db),
    ]);
    let f = &j["result"]["flags"][0];
    assert_eq!(f["index"], 0);
    assert_eq!(f["names"], serde_json::json!(["SW_PLOT_BOOLEAN_01"]));
    let writes: Vec<(String, i64)> = f["writes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| {
            (
                w["call"]["script"].as_str().unwrap().to_string(),
                w["value"].as_i64().unwrap(),
            )
        })
        .collect();
    assert!(
        writes.contains(&("k37_sta_taketrue".to_string(), 1)),
        "{writes:?}"
    );
    assert!(
        writes.contains(&("k37_sta_takefak1".to_string(), 0)),
        "{writes:?}"
    );
    let readers: Vec<&str> = f["reads"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["call"]["script"].as_str().unwrap())
        .collect();
    for s in ["k37_frd_rightswo", "k37_frd_wrongswd", "k37_ud_freedon"] {
        assert!(readers.contains(&s), "{readers:?}");
    }

    let (j, _) = run(&["quest", "kor37_ajuntapall", "--db", s(&db)]);
    let q = &j["result"]["quests"][0];
    let ids: Vec<i64> = q["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, [5, 10, 20, 30, 40, 50, 60]);
    for e in q["entries"].as_array().unwrap() {
        assert!(!e["writers"].as_array().unwrap().is_empty(), "{e}");
    }
    assert_eq!(q["removed_by"][0]["script"], "k_pebn_remove");
    let lint = j["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "RW2103")
        .expect("Shaardan's R12 is linted");
    assert!(
        lint["message"]
            .as_str()
            .unwrap()
            .starts_with("kor37_shaardan.dlg R12"),
        "{lint}"
    );
}
