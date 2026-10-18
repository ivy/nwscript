//! The dialogue lints over small dialogues built here, each with one fault,
//! against an install that answers from a list.

use std::collections::HashSet;

use rakata_dialog::{Dialog, GlobalReads, NodeRef, Place, References, codes, lint};
use rakata_gff::{Gff, Label, LocString, Struct, Value};
use rakata_res::{ResRef, ResType};

fn label(s: &str) -> Label {
    Label::new(s).unwrap()
}

fn link(index: u32, active: &str) -> Struct {
    let mut s = Struct::new(0);
    s.push(label("Index"), Value::Dword(index))
        .push(label("Active"), Value::resref(active).unwrap())
        .push(label("IsChild"), Value::Byte(0));
    s
}

/// A node's fields, as a test names them.
#[derive(Default)]
struct N {
    vo: &'static str,
    sound: &'static str,
    script: &'static str,
    sound_exists: Option<u8>,
    links: Vec<(u32, &'static str)>,
}

fn node(i: usize, n: &N, links_label: &str) -> Struct {
    let mut text = LocString::none();
    text.set(0, format!("Line {i}.").as_bytes());
    let mut s = Struct::new(0);
    s.push(label("Text"), Value::LocString(text))
        .push(label("VO_ResRef"), Value::resref(n.vo).unwrap())
        .push(label("Sound"), Value::resref(n.sound).unwrap())
        .push(label("Script"), Value::resref(n.script).unwrap())
        .push(
            label(links_label),
            Value::List(n.links.iter().map(|(t, c)| link(*t, c)).collect()),
        );
    if let Some(f) = n.sound_exists {
        s.push(label("SoundExists"), Value::Byte(f));
    }
    s
}

fn dialog(entries: &[N], replies: &[N], starts: &[(u32, &str)]) -> Dialog {
    let mut g = Gff::new(*b"DLG ");
    let e = entries
        .iter()
        .enumerate()
        .map(|(i, n)| node(i, n, "RepliesList"))
        .collect();
    let r = replies
        .iter()
        .enumerate()
        .map(|(i, n)| node(i, n, "EntriesList"))
        .collect();
    let s = starts
        .iter()
        .map(|(t, c)| {
            let mut l = Struct::new(0);
            l.push(label("Index"), Value::Dword(*t))
                .push(label("Active"), Value::resref(c).unwrap());
            l
        })
        .collect();
    g.root
        .push(label("EntryList"), Value::List(e))
        .push(label("ReplyList"), Value::List(r))
        .push(label("StartingList"), Value::List(s));
    Dialog::parse("fixture.dlg", &g.to_bytes().unwrap()).unwrap()
}

/// An install: the resources it holds, `globalcat.2da`'s names, and what
/// each condition reads.
struct Fake {
    have: HashSet<(String, ResType)>,
    declared: Vec<&'static str>,
}

impl Fake {
    fn new(have: &[(&str, ResType)]) -> Self {
        Fake {
            have: have.iter().map(|(n, t)| (n.to_string(), *t)).collect(),
            declared: vec!["K_DECLARED", "OAJ_DOWN_0", "OAJ_DOWN_1"],
        }
    }
}

impl References for Fake {
    fn resolves(&mut self, name: &ResRef, ty: ResType) -> bool {
        self.have.contains(&(name.to_string(), ty))
    }
    fn talk_table_len(&mut self) -> Option<u32> {
        Some(1000)
    }
    fn globals_read(&mut self, condition: &ResRef) -> Vec<String> {
        self.global_reads(condition).names
    }
    fn global_reads(&mut self, condition: &ResRef) -> GlobalReads {
        let mut r = GlobalReads::default();
        match condition.to_string().as_str() {
            "k_con_declared" => r.names.push("K_Declared".into()),
            "k_con_undeclared" => r.names.push("K_NOWHERE".into()),
            "k_con_family" => r.prefixes.push("OAJ_DOWN_".into()),
            "k_con_nofamily" => r.prefixes.push("OAJ_UP_".into()),
            "k_con_computed" => r.computed.push("GetGlobalBoolean".into()),
            _ => {}
        }
        r
    }
    fn declares_global(&mut self, name: &str) -> Option<bool> {
        Some(self.declared.iter().any(|d| d.eq_ignore_ascii_case(name)))
    }
    fn declares_prefix(&mut self, prefix: &str) -> Option<bool> {
        Some(self.declared.iter().any(|d| {
            d.to_ascii_uppercase()
                .starts_with(&prefix.to_ascii_uppercase())
        }))
    }
}

/// Every script the fixtures name, present.
fn scripts() -> Vec<(&'static str, ResType)> {
    [
        "k_con_declared",
        "k_con_undeclared",
        "k_con_family",
        "k_con_nofamily",
        "k_con_computed",
        "k_act_present",
    ]
    .into_iter()
    .map(|s| (s, ResType::Ncs))
    .collect()
}

fn codes_of(d: &Dialog, refs: &mut Fake) -> Vec<(&'static str, Place)> {
    lint(d, refs)
        .into_iter()
        .map(|f| (f.code, f.place))
        .collect()
}

/// E0 -> R0 -> E1, one start into E0.
fn clean_entries() -> Vec<N> {
    vec![
        N {
            links: vec![(0, "")],
            ..N::default()
        },
        N::default(),
    ]
}

fn clean_replies() -> Vec<N> {
    vec![N {
        links: vec![(1, "")],
        ..N::default()
    }]
}

#[test]
fn a_clean_dialogue_has_no_findings() {
    let d = dialog(&clean_entries(), &clean_replies(), &[(0, "k_con_declared")]);
    assert_eq!(codes_of(&d, &mut Fake::new(&scripts())), vec![]);
}

#[test]
fn a_dangling_script_is_rw1102_and_nothing_else() {
    let mut e = clean_entries();
    e[1].script = "k_act_absent";
    let d = dialog(&e, &clean_replies(), &[(0, "")]);
    assert_eq!(
        codes_of(&d, &mut Fake::new(&scripts())),
        vec![(codes::MISSING_SCRIPT, Place::Node(NodeRef::entry(1)))]
    );
}

#[test]
fn an_orphan_entry_is_rw1107_and_nothing_else() {
    let mut e = clean_entries();
    // E2 links on to R0 but nothing leads to E2.
    e.push(N {
        links: vec![(0, "")],
        ..N::default()
    });
    let d = dialog(&e, &clean_replies(), &[(0, "")]);
    assert_eq!(
        codes_of(&d, &mut Fake::new(&scripts())),
        vec![(codes::UNREACHED_NODE, Place::Node(NodeRef::entry(2)))]
    );
}

#[test]
fn a_chain_only_an_orphan_reaches_is_unreached_too() {
    let mut e = clean_entries();
    e.push(N {
        links: vec![(1, "")],
        ..N::default()
    });
    let mut r = clean_replies();
    r.push(N::default());
    let d = dialog(&e, &r, &[(0, "")]);
    assert_eq!(
        codes_of(&d, &mut Fake::new(&scripts())),
        vec![
            (codes::UNREACHED_NODE, Place::Node(NodeRef::entry(2))),
            (codes::UNREACHED_NODE, Place::Node(NodeRef::reply(1))),
        ]
    );
}

#[test]
fn an_undeclared_global_in_a_condition_is_rw1106_and_nothing_else() {
    let mut e = clean_entries();
    e[0].links = vec![(0, "k_con_undeclared")];
    let d = dialog(&e, &clean_replies(), &[(0, "")]);
    let found = lint(&d, &mut Fake::new(&scripts()));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, codes::UNDECLARED_GLOBAL);
    assert_eq!(found[0].subject, "K_NOWHERE");
    assert_eq!(found[0].via.as_deref(), Some("k_con_undeclared"));
}

#[test]
fn a_name_built_on_a_declared_prefix_is_clean_and_on_an_undeclared_one_is_rw1106() {
    let d = dialog(&clean_entries(), &clean_replies(), &[(0, "k_con_family")]);
    assert_eq!(codes_of(&d, &mut Fake::new(&scripts())), vec![]);
    let d = dialog(&clean_entries(), &clean_replies(), &[(0, "k_con_nofamily")]);
    let found = lint(&d, &mut Fake::new(&scripts()));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, codes::UNDECLARED_GLOBAL);
    assert_eq!(found[0].subject, "OAJ_UP_*");
}

#[test]
fn a_computed_name_is_undecidable_never_clean() {
    let d = dialog(&clean_entries(), &clean_replies(), &[(0, "k_con_computed")]);
    let found = lint(&d, &mut Fake::new(&scripts()));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, codes::UNDECIDABLE_GLOBAL);
    assert_eq!(found[0].via.as_deref(), Some("GetGlobalBoolean"));
}

#[test]
fn a_dangling_link_is_rw1101() {
    let mut e = clean_entries();
    e[1].links = vec![(7, "")];
    let d = dialog(&e, &clean_replies(), &[(0, "")]);
    assert_eq!(
        codes_of(&d, &mut Fake::new(&scripts())),
        vec![(
            codes::DANGLING_LINK,
            Place::Link {
                from: NodeRef::entry(1),
                to: NodeRef::reply(7)
            }
        )]
    );
}

#[test]
fn a_voiced_node_wants_its_wave_and_its_lip_each_under_its_code() {
    let mut e = clean_entries();
    e[1].vo = "vo_line";
    e[1].sound_exists = Some(1);
    let d = dialog(&e, &clean_replies(), &[(0, "")]);
    let e1 = Place::Node(NodeRef::entry(1));
    assert_eq!(
        codes_of(&d, &mut Fake::new(&scripts())),
        vec![
            (codes::MISSING_VOICE, e1.clone()),
            (codes::MISSING_LIP, e1.clone())
        ]
    );
    let mut have = scripts();
    have.push(("vo_line", ResType::Wav));
    assert_eq!(
        codes_of(&d, &mut Fake::new(&have)),
        vec![(codes::MISSING_LIP, e1)]
    );
    have.push(("vo_line", ResType::Lip));
    assert_eq!(codes_of(&d, &mut Fake::new(&have)), vec![]);
}

#[test]
fn a_placeholder_voice_or_one_a_sound_replaces_is_not_voiced() {
    let mut e = clean_entries();
    // BioWare's tool writes SoundExists 0 on a line it has no recording
    // for, and a Wookiee's line plays its Sound.
    e[0].vo = "_m00aaplay01000_";
    e[0].sound_exists = Some(0);
    e[1].vo = "nm00aawook01000_";
    e[1].sound = "n_gwwook_comm1";
    e[1].sound_exists = Some(3);
    let d = dialog(&e, &clean_replies(), &[(0, "")]);
    assert_eq!(codes_of(&d, &mut Fake::new(&scripts())), vec![]);
}

#[test]
fn sound_exists_disagreeing_with_the_audio_is_rw1110() {
    let mut e = clean_entries();
    e[0].sound_exists = Some(1);
    e[1].vo = "vo_line";
    e[1].sound_exists = Some(0);
    let mut have = scripts();
    have.push(("vo_line", ResType::Wav));
    let d = dialog(&e, &clean_replies(), &[(0, "")]);
    let found = lint(&d, &mut Fake::new(&have));
    let got: Vec<_> = found.iter().map(|f| (f.code, f.place.clone())).collect();
    assert_eq!(
        got,
        vec![
            (codes::SOUND_EXISTS, Place::Node(NodeRef::entry(0))),
            (codes::SOUND_EXISTS, Place::Node(NodeRef::entry(1))),
        ]
    );
    assert!(found[0].message("fixture.dlg").contains("names neither"));
    assert!(found[1].message("fixture.dlg").contains("is present"));
}
