//! Hand-made dialogues built with `rakata-gff`: the model's reading, the
//! keys and their refusals, and the transcript's shape. No game bytes.

use rakata_dialog::{
    Dialog, Embedded, Error, Kind, Lookup, NodeKey, NodeRef, Options, Script, ScriptStatus,
    Transcript, Voice, codes,
};
use rakata_gff::{Gff, Label, LocString, Struct, Value};
use rakata_res::ResRef;

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

fn link(index: u32, active: &str, child: bool) -> Struct {
    let mut s = Struct::new(0);
    s.push(label("Index"), Value::Dword(index))
        .push(label("Active"), Value::resref(active).unwrap())
        .push(label("IsChild"), Value::Byte(u8::from(child)));
    s
}

fn text(t: &str) -> LocString {
    let mut l = LocString::none();
    l.set(0, t.as_bytes());
    l
}

/// An entry or reply with the fields BioWare's files carry.
fn node(kind: Kind, speaker: &str, said: LocString, vo: &str, links: Vec<Struct>) -> Struct {
    let mut s = Struct::new(0);
    if kind == Kind::Entry {
        s.push(label("Speaker"), Value::string(speaker));
    }
    let mut anim = Struct::new(0);
    anim.push(label("Participant"), Value::string("Mission"))
        .push(label("Animation"), Value::Word(10055));
    s.push(label("AnimList"), Value::List(vec![anim]))
        .push(label("Text"), Value::LocString(said))
        .push(label("VO_ResRef"), Value::resref(vo).unwrap())
        .push(label("Script"), Value::resref("").unwrap())
        .push(label("Delay"), Value::Dword(u32::MAX))
        .push(label("Listener"), Value::string(""))
        .push(label("CameraAngle"), Value::Dword(0))
        .push(label(kind.links()), Value::List(links))
        // A field the model does not name, written twice as BioWare's
        // tool does.
        .push(label("SoundExists"), Value::Byte(1))
        .push(label("SoundExists"), Value::Byte(1));
    s
}

/// Two starts: E0 (conditioned) and E2. E0 -> R0 -> E1 -> R1, E0 -> R1
/// again (a reference), E2 -> R2 -> E1 (a link, shown above). E3 is
/// reached from no start. E0 and E1 both say "Hello.".
fn fixture() -> Dialog {
    let mut g = Gff::new(*b"DLG ");
    let entries = vec![
        node(
            Kind::Entry,
            "",
            text("Hello."),
            "oaj_hello",
            vec![link(0, "", false), link(1, "k_con_test", false)],
        ),
        node(
            Kind::Entry,
            "Carth",
            text(" Hello. "),
            "",
            vec![link(1, "", false)],
        ),
        node(
            Kind::Entry,
            "Mission",
            LocString::strref(38729),
            "nglobebant00089_",
            vec![link(2, "", false)],
        ),
        node(Kind::Entry, "", text("Alone."), "", vec![]),
    ];
    let replies = vec![
        node(Kind::Reply, "", text("Hi."), "", vec![link(1, "", false)]),
        node(Kind::Reply, "", text("Bye."), "", vec![]),
        node(Kind::Reply, "", text("Again."), "", vec![link(1, "", true)]),
    ];
    let mut start0 = Struct::new(0);
    start0
        .push(label("Index"), Value::Dword(0))
        .push(label("Active"), Value::resref("k_con_start").unwrap());
    let mut start1 = Struct::new(1);
    start1
        .push(label("Index"), Value::Dword(2))
        .push(label("Active"), Value::resref("").unwrap());
    g.root
        .push(label("DelayEntry"), Value::Dword(0))
        .push(label("Skippable"), Value::Byte(1))
        .push(label("ConversationType"), Value::Int(0))
        .push(label("EntryList"), Value::List(entries))
        .push(label("ReplyList"), Value::List(replies))
        .push(label("StartingList"), Value::List(vec![start0, start1]));
    let bytes = g.to_bytes().unwrap();
    Dialog::parse("fixture.dlg", seed(&bytes)).unwrap()
}

#[test]
fn the_model_reads_nodes_links_and_starts() {
    let d = fixture();
    assert_eq!((d.len(Kind::Entry), d.len(Kind::Reply)), (4, 3));
    let e0 = d.node(NodeRef::entry(0)).unwrap();
    assert_eq!(e0.speaker().as_deref(), Some(""));
    assert_eq!(e0.voice(), Some(ResRef::new("OAJ_Hello").unwrap()));
    assert_eq!(e0.embedded_text().as_deref(), Some("Hello."));
    assert_eq!(e0.animations()[0].animation, 10055);
    assert_eq!(e0.animations()[0].participant, "Mission");
    let links = e0.links();
    assert_eq!(links.len(), 2);
    assert_eq!(links[1].target, NodeRef::reply(1));
    assert_eq!(
        links[1].condition().map(|c| c.to_string()).as_deref(),
        Some("k_con_test")
    );
    assert!(!links[1].is_child());
    let starts = d.starts();
    assert_eq!(starts[0].target, NodeRef::entry(0));
    assert_eq!(starts[1].target, NodeRef::entry(2));
    assert_eq!(d.node(NodeRef::entry(2)).unwrap().strref(), Some(38729));
    assert_eq!(d.info().skippable, Some(true));
}

#[test]
fn a_dialogue_writes_back_as_read_with_the_fields_it_does_not_name() {
    let d = fixture();
    let bytes = d.to_bytes().unwrap();
    let again = Dialog::parse("fixture.dlg", seed(&bytes)).unwrap();
    assert_eq!(again.to_bytes().unwrap(), bytes);
    let e0 = again.node(NodeRef::entry(0)).unwrap();
    let n = e0
        .raw()
        .fields
        .iter()
        .filter(|f| f.label.is("SoundExists"))
        .count();
    assert_eq!(n, 2);
}

#[test]
fn a_gff_of_another_type_is_refused_with_its_code() {
    let g = Gff::new(*b"UTC ");
    let e = Dialog::new("p_bastila.utc", g).unwrap_err();
    assert_eq!(e.code(), codes::NOT_DIALOG);
    assert!(e.message().contains("p_bastila.utc"), "{e}");
}

#[test]
fn a_voice_key_resolves_to_its_one_node_without_regard_to_case() {
    let d = fixture();
    let key = NodeKey::Voice(ResRef::new("OAJ_HELLO").unwrap());
    assert_eq!(key.resolve(&d).unwrap(), NodeRef::entry(0));
    assert!(!key.is_fragile());
}

#[test]
fn a_key_that_matches_nothing_names_the_key_and_the_dialogue() {
    let d = fixture();
    let key = NodeKey::Voice(ResRef::new("oaj_missing").unwrap());
    let e = key.resolve(&d).unwrap_err();
    assert_eq!(e.code(), "RE1102");
    let m = e.to_string();
    assert!(m.contains("voice:oaj_missing"), "{m}");
    assert!(m.contains("fixture.dlg"), "{m}");
}

#[test]
fn a_text_key_matching_two_nodes_fails_naming_both() {
    let d = fixture();
    // E1's text is " Hello. ": both compare after trimming.
    let e = NodeKey::Text("Hello.".into()).resolve(&d).unwrap_err();
    assert_eq!(e.code(), codes::AMBIGUOUS_KEY);
    match &e {
        Error::AmbiguousKey { nodes, .. } => {
            assert_eq!(nodes, &[NodeRef::entry(0), NodeRef::entry(1)]);
        }
        other => panic!("{other:?}"),
    }
    let m = e.to_string();
    assert!(m.contains("E0, E1"), "{m}");
}

#[test]
fn a_text_key_compares_through_the_talk_table() {
    let d = fixture();
    let key = NodeKey::Text("What the -?? Hey, that wasn't funny!".into());
    assert_eq!(key.resolve(&d).unwrap_err().code(), "RE1102");
    let mut talk = |s: &rakata_res::LocalizedString| {
        (s.strref.raw() == 38729).then(|| "What the -?? Hey, that wasn't funny!".to_string())
    };
    assert_eq!(key.resolve_with(&d, &mut talk).unwrap(), NodeRef::entry(2));
}

#[test]
fn strref_and_index_keys_resolve_and_the_index_is_fragile() {
    let d = fixture();
    assert_eq!(
        NodeKey::StrRef(38729).resolve(&d).unwrap(),
        NodeRef::entry(2)
    );
    let key = NodeKey::Index(Kind::Reply, 2);
    assert_eq!(key.resolve(&d).unwrap(), NodeRef::reply(2));
    assert!(key.is_fragile());
    assert!(key.to_string().contains("fragile"));
    assert_eq!(
        NodeKey::Index(Kind::Reply, 9)
            .resolve(&d)
            .unwrap_err()
            .code(),
        "RE1102"
    );
}

#[test]
fn keys_parse_from_text() {
    assert_eq!(
        "voice:nm14acjuha07024_".parse::<NodeKey>().unwrap(),
        NodeKey::Voice(ResRef::new("nm14acjuha07024_").unwrap())
    );
    assert_eq!("strref:5".parse::<NodeKey>().unwrap(), NodeKey::StrRef(5));
    assert_eq!(
        "E3".parse::<NodeKey>().unwrap(),
        NodeKey::Index(Kind::Entry, 3)
    );
    assert_eq!(
        "reply:0".parse::<NodeKey>().unwrap(),
        NodeKey::Index(Kind::Reply, 0)
    );
    assert_eq!(
        "text: Hi.".parse::<NodeKey>().unwrap(),
        NodeKey::Text(" Hi.".into())
    );
    for bad in ["strref:x", "voice:seventeen_bytes_x", "Q1", "nonsense"] {
        let e = bad.parse::<NodeKey>().unwrap_err();
        assert_eq!(e.code(), codes::BAD_KEY, "{bad}");
    }
}

#[test]
fn the_transcript_shows_every_node_once_and_references_the_rest() {
    let d = fixture();
    let t = Transcript::build(&d, &mut Embedded, Options::default());
    let order: Vec<&str> = t.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(order, ["E0", "R0", "E1", "R1", "E2", "R2", "E3"]);
    let rows: Vec<(&str, bool, bool)> = t
        .tree
        .iter()
        .map(|r| (r.node.as_str(), r.repeat, r.unreached))
        .collect();
    assert_eq!(
        rows,
        [
            ("E0", false, false),
            ("R0", false, false),
            ("E1", false, false),
            ("R1", false, false),
            ("R1", true, false),
            ("E2", false, false),
            ("R2", false, false),
            ("E1", true, false),
            ("E3", false, true),
        ]
    );
    assert_eq!(t.tree[0].condition.as_deref(), Some("k_con_start"));
    assert_eq!(t.tree[4].condition.as_deref(), Some("k_con_test"));
    assert!(t.tree[7].is_child);
    let e0 = t.node("E0").unwrap();
    assert_eq!(e0.speaker_role, "owner");
    assert_eq!(t.node("E1").unwrap().speaker_role, "tag");
    assert_eq!(t.node("R0").unwrap().speaker_role, "player");
    let text = t.text();
    assert!(
        text.contains("E0 [voice:oaj_hello] (owner): \"Hello.\""),
        "{text}"
    );
    assert!(text.contains("-> R1 (shown above)"), "{text}");
    assert!(text.contains("-> E1 (shown above, a link)"), "{text}");
    assert!(text.contains("animation 10055 on Mission"), "{text}");
    assert!(text.contains("Reached from no start:"), "{text}");
    assert!(text.contains("if k_con_start [not looked up]"), "{text}");
    // No delay of 0xFFFFFFFF in the text, all of it in the JSON.
    assert!(!text.contains("delay"), "{text}");
    let json = serde_json::to_value(&t).unwrap();
    assert_eq!(json["nodes"][0]["delay"], u32::MAX);
    assert_eq!(json["nodes"][0]["animations"][0]["animation"], 10055);
    assert_eq!(json["scripts"][0]["name"], "k_con_start");
    assert_eq!(json["scripts"][0]["status"], "unresolved");
}

#[test]
fn each_node_carries_the_key_nodekey_chooses_and_nodes_text_shows_its_links() {
    let d = fixture();
    let t = Transcript::build(&d, &mut Embedded, Options::default());
    for n in &t.nodes {
        let at = NodeRef {
            kind: if n.kind == "entry" {
                Kind::Entry
            } else {
                Kind::Reply
            },
            index: n.index,
        };
        let key = NodeKey::of(&d, at);
        assert_eq!(n.key, key.to_string(), "{}", n.id);
        assert_eq!(n.fragile, key.is_fragile(), "{}", n.id);
    }
    assert_eq!(t.node("E0").unwrap().key, "voice:oaj_hello");
    assert_eq!(t.node("E2").unwrap().key, "voice:nglobebant00089_");
    // E1's text is E0's, so only its index names it alone.
    assert_eq!(t.node("E1").unwrap().key, "index:E1 (fragile)");
    assert!(t.node("E1").unwrap().fragile);
    let text = t.text();
    assert!(text.contains("E1 [index:E1 (fragile)] Carth"), "{text}");

    let one = t.nodes_text(&["E0", "R2"]);
    assert!(
        one.starts_with("E0 [voice:oaj_hello] (owner): \"Hello.\"\n"),
        "{one}"
    );
    assert!(one.contains("  -> R0 [text:\"Hi.\"]\n"), "{one}");
    assert!(
        one.contains("  if k_con_test [not looked up]\n  -> R1 [text:\"Bye.\"]\n"),
        "{one}"
    );
    assert!(one.contains("\n\nR2 [text:\"Again.\"] (player)"), "{one}");
    assert!(
        one.contains("  -> E1 [index:E1 (fragile)] (a link)"),
        "{one}"
    );
    assert!(!one.contains("E2"), "{one}");
}

#[test]
fn a_link_past_the_end_is_a_warning_and_a_row() {
    let mut g = Gff::new(*b"DLG ");
    let e = node(Kind::Entry, "", text("x"), "", vec![link(7, "", false)]);
    let mut start = Struct::new(0);
    start
        .push(label("Index"), Value::Dword(0))
        .push(label("Active"), Value::resref("").unwrap());
    g.root
        .push(label("EntryList"), Value::List(vec![e]))
        .push(label("ReplyList"), Value::List(vec![]))
        .push(label("StartingList"), Value::List(vec![start]));
    let d = Dialog::new("broken.dlg", g).unwrap();
    let t = Transcript::build(&d, &mut Embedded, Options::default());
    assert_eq!(t.warnings.len(), 1);
    assert_eq!(t.warnings[0].code, codes::DANGLING_LINK);
    assert!(t.warnings[0].message.contains("R7"));
    assert!(t.tree[1].missing);
    assert!(t.text().contains("-> R7 (no such node)"));
}

/// A lookup with no voice files and every script it is asked to decompile
/// decompiled to a one-line source naming it.
#[derive(Default)]
struct Scripts {
    decompiled: Vec<String>,
}

impl Lookup for Scripts {
    fn text(&mut self, s: &rakata_res::LocalizedString) -> Option<String> {
        Embedded.text(s)
    }

    fn voice(&mut self, _: &ResRef) -> Option<Voice> {
        None
    }

    fn script(&mut self, name: &ResRef, decompile: bool) -> Script {
        if !decompile {
            return Script::named(name, ScriptStatus::Found);
        }
        self.decompiled.push(name.to_string());
        let mut s = Script::named(name, ScriptStatus::Verified);
        s.functions = 1;
        s.verified = 1;
        s.source = Some(format!(
            "int StartingConditional() {{ return 1; }} // {name}"
        ));
        s
    }
}

fn list<'g>(g: &'g mut Gff, name: &str) -> &'g mut Vec<Struct> {
    match g.root.get_mut(name) {
        Some(Value::List(l)) => l,
        _ => panic!("{name} is a list"),
    }
}

#[test]
fn a_voice_with_no_file_is_noted_only_where_the_line_is_voiced() {
    let mut g = fixture().into_gff();
    // E2 names a voice-over with SoundExists 0: nobody recorded it. R0, a
    // reply, names a placeholder.
    list(&mut g, "EntryList")[2].set(label("SoundExists"), Value::Byte(0));
    list(&mut g, "ReplyList")[0].set(
        label("VO_ResRef"),
        Value::resref("_m37aafree04001_").unwrap(),
    );
    let d = Dialog::new("voices.dlg", g).unwrap();
    let t = Transcript::build(&d, &mut Scripts::default(), Options::default());
    let text = t.text();
    assert!(
        text.contains("voice oaj_hello, no file under streamwaves/"),
        "{text}"
    );
    assert!(!text.contains("voice nglobebant00089_"), "{text}");
    assert!(!text.contains("voice _m37aafree04001_"), "{text}");
    let expected = |id: &str| t.node(id).unwrap().voice.as_ref().unwrap().expected;
    assert!(expected("E0"));
    assert!(!expected("E2"));
    assert!(!expected("R0"));
}

#[test]
fn the_starts_show_each_condition_s_source_and_brief_shows_names_only() {
    let d = fixture();
    let mut lookup = Scripts::default();
    let t = Transcript::build(&d, &mut lookup, Options::default());
    let text = t.text();
    assert!(
        text.contains(
            "  1. E0 if k_con_start [verified, 1 of 1 functions]\n    | int StartingConditional() { return 1; } // k_con_start\n  2. E2 always\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("\nif k_con_start [verified, 1 of 1 functions] (source above)\n"),
        "{text}"
    );

    let mut lookup = Scripts::default();
    let t = Transcript::build(
        &d,
        &mut lookup,
        Options {
            brief: true,
            scripts: true,
        },
    );
    assert!(lookup.decompiled.is_empty(), "{:?}", lookup.decompiled);
    let text = t.text();
    assert!(text.contains("  1. E0 if k_con_start\n"), "{text}");
    assert!(
        text.contains("\nif k_con_start\nE0 [voice:oaj_hello] (owner): \"Hello.\"\n"),
        "{text}"
    );
    assert!(!text.contains("animation"), "{text}");
    assert!(!text.contains("voice oaj_hello"), "{text}");
    assert!(!text.contains('|'), "{text}");
}
