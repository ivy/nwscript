//! Append-only edits and the reference checks, over hand-made dialogues.
//! No game bytes.

use std::collections::HashSet;

use rakata_dialog::{
    Dialog, Finding, Kind, NewNode, NodeKey, NodeRef, Place, References, Text, check, codes,
};
use rakata_gff::{Gff, Label, LocString, Struct, Value};
use rakata_res::{ResRef, ResType};

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

fn label(s: &str) -> Label {
    Label::new(s).unwrap()
}

fn r(s: &str) -> ResRef {
    ResRef::new(s).unwrap()
}

fn link(index: u32, active: &str) -> Struct {
    let mut s = Struct::new(0);
    s.push(label("Index"), Value::Dword(index))
        .push(label("Active"), Value::resref(active).unwrap())
        .push(label("IsChild"), Value::Byte(0));
    s
}

fn said(t: &str) -> LocString {
    let mut l = LocString::none();
    l.set(0, t.as_bytes());
    l
}

/// Ten nodes: entries E0..E5 and replies R0..R3, a chain E0 -> R0 -> E1 ->
/// R1 -> E2 ..., one start into E0 under `k_con_start`, E3 run by
/// `k_act_three`, E4 voiced by `vo_four`, E5 showing strref 900.
fn ten() -> Dialog {
    let mut entries = Vec::new();
    for i in 0..6u32 {
        let mut s = Struct::new(i);
        let text = if i == 5 {
            LocString::strref(900)
        } else {
            said(&format!("Entry {i}."))
        };
        s.push(label("Speaker"), Value::string(""))
            .push(label("Text"), Value::LocString(text))
            .push(
                label("VO_ResRef"),
                Value::resref(if i == 4 { "vo_four" } else { "" }).unwrap(),
            )
            .push(
                label("Script"),
                Value::resref(if i == 3 { "k_act_three" } else { "" }).unwrap(),
            )
            .push(
                label("RepliesList"),
                Value::List(if i < 4 { vec![link(i, "")] } else { vec![] }),
            );
        entries.push(s);
    }
    let mut replies = Vec::new();
    for i in 0..4u32 {
        let mut s = Struct::new(i);
        s.push(
            label("Text"),
            Value::LocString(said(&format!("Reply {i}."))),
        )
        .push(label("Script"), Value::resref("").unwrap())
        .push(label("EntriesList"), Value::List(vec![link(i + 1, "")]));
        replies.push(s);
    }
    let mut start = Struct::new(0);
    start
        .push(label("Index"), Value::Dword(0))
        .push(label("Active"), Value::resref("k_con_start").unwrap());
    let mut g = Gff::new(*b"DLG ");
    g.root
        .push(label("Skippable"), Value::Byte(1))
        .push(label("EndConversation"), Value::resref("k_end").unwrap())
        .push(label("EntryList"), Value::List(entries))
        .push(label("ReplyList"), Value::List(replies))
        .push(label("StartingList"), Value::List(vec![start]));
    Dialog::parse("ten.dlg", seed(&g.to_bytes().unwrap())).unwrap()
}

fn line(text: &str) -> NewNode {
    NewNode {
        text: Text::English(text.into()),
        ..NewNode::default()
    }
}

#[test]
fn appending_an_entry_leaves_every_node_at_its_index() {
    let mut d = ten();
    let before: Vec<(NodeRef, Struct)> = d.all().map(|n| (n.at, n.raw().clone())).collect();
    assert_eq!(before.len(), 10);
    let at = d.push_entry(&line("A new line.")).unwrap();
    assert_eq!(at, NodeRef::entry(6));
    let reply = d.push_reply(&NewNode::default()).unwrap();
    assert_eq!(reply, NodeRef::reply(4));
    let bytes = d.to_bytes().unwrap();
    let again = Dialog::parse("ten.dlg", seed(&bytes)).unwrap();
    for (at, raw) in &before {
        assert_eq!(again.node(*at).unwrap().raw(), raw, "{at} moved or changed");
    }
    assert_eq!(
        again.node(at).unwrap().embedded_text().as_deref(),
        Some("A new line.")
    );
}

#[test]
fn a_new_entry_carries_the_fields_bioware_writes_in_their_order() {
    let mut d = ten();
    let at = d
        .push_entry(&NewNode {
            text: Text::StrRef(42),
            speaker: "Juhani".into(),
            listener: "Carth".into(),
            voice: Some(r("oaj_juh01")),
            script: Some(r("k_oaj_beat")),
            animations: vec![("Juhani".into(), 10055)],
            quest: Some(("dan_trials".into(), 45)),
            camera: Some(13),
            comment: "Oops! All Juhani".into(),
        })
        .unwrap();
    let node = d.node(at).unwrap();
    let labels: Vec<String> = node
        .raw()
        .fields
        .iter()
        .map(|f| f.label.to_string())
        .collect();
    assert_eq!(
        labels,
        [
            "AnimList",
            "Text",
            "VO_ResRef",
            "Script",
            "Delay",
            "Comment",
            "Sound",
            "Quest",
            "PlotIndex",
            "PlotXPPercentage",
            "Listener",
            "WaitFlags",
            "CameraAngle",
            "FadeType",
            "RepliesList",
            "Speaker",
            "SoundExists",
            "QuestEntry",
            "CameraID",
            "CamVidEffect"
        ]
    );
    assert_eq!(node.strref(), Some(42));
    assert_eq!(node.camera_angle(), Some(rakata_dialog::CAMERA_STATIC));
    assert_eq!(node.camera_id(), Some(13));
    assert_eq!(node.quest().as_deref(), Some("dan_trials"));
    assert_eq!(node.delay(), Some(rakata_dialog::NO_DELAY));
    assert_eq!(node.raw().get("SoundExists"), Some(&Value::Byte(1)));
    assert_eq!(node.animations()[0].animation, 10055);
}

#[test]
fn links_and_starts_go_where_asked_and_refuse_a_link_within_one_kind() {
    let mut d = ten();
    let e = d.push_entry(&line("Onward.")).unwrap();
    // A link from R1 to the new entry, first among R1's links.
    let at = d
        .push_link(NodeRef::reply(1), e, Some(r("k_con_new")), Some(0))
        .unwrap();
    assert_eq!(at, 0);
    let links = d.node(NodeRef::reply(1)).unwrap().links();
    assert_eq!(links[0].target, e);
    assert_eq!(links[0].condition(), Some(r("k_con_new")));
    assert_eq!(links[1].target, NodeRef::entry(2));
    d.push_start(e, None, 0).unwrap();
    let starts = d.starts();
    assert_eq!(starts[0].target, e);
    assert_eq!(starts[1].target, NodeRef::entry(0));

    let err = d
        .push_link(NodeRef::entry(0), NodeRef::entry(1), None, None)
        .unwrap_err();
    assert_eq!(err.code(), codes::BAD_LINK);
    let err = d.push_start(NodeRef::reply(0), None, 0).unwrap_err();
    assert_eq!(err.code(), codes::BAD_LINK);
    let err = d
        .push_link(NodeRef::entry(0), NodeRef::reply(0), None, Some(9))
        .unwrap_err();
    assert_eq!(err.code(), codes::BAD_LINK);
    let err = d
        .push_link(NodeRef::entry(0), NodeRef::reply(40), None, None)
        .unwrap_err();
    assert_eq!(err.code(), codes::NO_SUCH_NODE);
}

#[test]
fn reordering_rewrites_every_link_to_follow_its_node() {
    let mut d = ten();
    // Swap E0 and E1.
    d.reorder(Kind::Entry, &[1, 0, 2, 3, 4, 5]).unwrap();
    assert_eq!(
        d.node(NodeRef::entry(1))
            .unwrap()
            .embedded_text()
            .as_deref(),
        Some("Entry 0.")
    );
    assert_eq!(d.starts()[0].target, NodeRef::entry(1));
    // R0 led to the old E1, now E0.
    assert_eq!(
        d.node(NodeRef::reply(0)).unwrap().links()[0].target,
        NodeRef::entry(0)
    );
    let err = d.reorder(Kind::Entry, &[0, 0, 2, 3, 4, 5]).unwrap_err();
    assert_eq!(err.code(), codes::BAD_ORDER);
    let err = d.reorder(Kind::Reply, &[0, 1]).unwrap_err();
    assert_eq!(err.code(), codes::BAD_ORDER);
}

#[test]
fn an_emptied_dialogue_keeps_the_settings_and_drops_the_lines() {
    let d = ten();
    let e = d.emptied("new.dlg");
    assert_eq!(e.name(), "new.dlg");
    assert_eq!(e.len(Kind::Entry) + e.len(Kind::Reply), 0);
    assert!(e.starts().is_empty());
    assert_eq!(e.info().skippable, Some(true));
    assert_eq!(e.info().end_conversation, None);
    let again = Dialog::parse("new.dlg", seed(&e.to_bytes().unwrap())).unwrap();
    assert_eq!(again.len(Kind::Entry), 0);
}

#[test]
fn a_node_text_outside_the_code_page_is_refused() {
    let mut d = ten();
    let err = d.push_entry(&line("\u{263a}")).unwrap_err();
    assert_eq!(err.code(), codes::UNENCODABLE);
    assert_eq!(d.len(Kind::Entry), 6);
}

#[test]
fn the_stable_key_of_a_node_prefers_voice_then_strref_then_text() {
    let d = ten();
    assert_eq!(
        NodeKey::of(&d, NodeRef::entry(4)),
        NodeKey::Voice(r("vo_four"))
    );
    assert_eq!(NodeKey::of(&d, NodeRef::entry(5)), NodeKey::StrRef(900));
    assert_eq!(
        NodeKey::of(&d, NodeRef::entry(0)),
        NodeKey::Text("Entry 0.".into())
    );
    assert_eq!(
        NodeKey::from(NodeRef::reply(2)),
        NodeKey::Index(Kind::Reply, 2)
    );
}

/// An install as a set of names, a talk table length and globalcat's
/// names; `k_con_global` reads `OAJ_DONE` and `OAJ_UNDECLARED`.
struct Fake {
    have: HashSet<(String, ResType)>,
    tlk: Option<u32>,
    globals: Option<Vec<&'static str>>,
}

impl References for Fake {
    fn resolves(&mut self, name: &ResRef, ty: ResType) -> bool {
        self.have
            .contains(&(name.to_string().to_ascii_lowercase(), ty))
    }
    fn talk_table_len(&mut self) -> Option<u32> {
        self.tlk
    }
    fn globals_read(&mut self, condition: &ResRef) -> Vec<String> {
        if condition == &r("k_con_global") {
            vec!["OAJ_DONE".into(), "OAJ_UNDECLARED".into()]
        } else {
            vec![]
        }
    }
    fn declares_global(&mut self, name: &str) -> Option<bool> {
        self.globals
            .as_ref()
            .map(|g| g.iter().any(|n| n.eq_ignore_ascii_case(name)))
    }
}

fn fake(have: &[(&str, ResType)]) -> Fake {
    Fake {
        have: have.iter().map(|(n, t)| (n.to_string(), *t)).collect(),
        tlk: Some(500),
        globals: Some(vec!["oaj_done"]),
    }
}

#[test]
fn the_checks_find_each_missing_reference_with_its_code_and_key() {
    let mut d = ten();
    d.push_link(
        NodeRef::entry(4),
        NodeRef::reply(0),
        Some(r("k_con_global")),
        None,
    )
    .unwrap();
    let mut refs = fake(&[
        ("k_con_start", ResType::Ncs),
        ("k_con_global", ResType::Ncs),
        ("k_end", ResType::Ncs),
    ]);
    let found = check(&d, &mut refs);
    let codes_found: Vec<(&str, String)> =
        found.iter().map(|f| (f.code, f.subject.clone())).collect();
    assert_eq!(
        codes_found,
        [
            (codes::MISSING_SCRIPT, "k_act_three".to_string()),
            (codes::MISSING_VOICE, "vo_four".to_string()),
            (codes::STRREF_PAST_TLK, "900".to_string()),
            (codes::UNDECLARED_GLOBAL, "OAJ_UNDECLARED".to_string()),
        ]
    );
    assert_eq!(found[0].place, Place::Node(NodeRef::entry(3)));
    assert_eq!(found[0].key, Some(NodeKey::Text("Entry 3.".into())));
    let m = found[0].message("ten.dlg");
    assert!(
        m.contains("k_act_three") && m.contains("text:\"Entry 3.\""),
        "{m}"
    );
    assert_eq!(
        found[3].place,
        Place::Link {
            from: NodeRef::entry(4),
            to: NodeRef::reply(0)
        }
    );
    assert_eq!(found[3].via.as_deref(), Some("k_con_global"));
    assert!(found[3].message("ten.dlg").contains("voice:vo_four"));
}

#[test]
fn a_missing_condition_is_found_on_its_start_and_nothing_is_asked_without_tables() {
    let d = ten();
    let mut refs = fake(&[
        ("k_act_three", ResType::Ncs),
        ("vo_four", ResType::Wav),
        ("k_end", ResType::Ncs),
    ]);
    refs.tlk = None;
    refs.globals = None;
    let found: Vec<Finding> = check(&d, &mut refs);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, codes::MISSING_SCRIPT);
    assert_eq!(
        found[0].place,
        Place::Start {
            to: NodeRef::entry(0)
        }
    );
    assert!(found[0].same_as(&found[0].clone()));
}
