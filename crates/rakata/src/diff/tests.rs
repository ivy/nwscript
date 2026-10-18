//! The diff over fixtures built here: GFF fields, 2DA rows, archives by
//! member, dialogues by node, and trees with renames, links and Override
//! files compared with the copies they replace.

use std::path::{Path, PathBuf};

use rakata_2da::TwoDa;
use rakata_archive::{Erf, ErfKind, Key, Rim};
use rakata_gff::{Gff, Label, LocString, Struct, Value};
use rakata_res::ResId;

use super::*;

fn l(t: &str) -> Label {
    Label::new(t).unwrap()
}

fn utc(tag: &str) -> Gff {
    let mut g = Gff::new(*b"UTC ");
    g.root
        .push(l("Tag"), Value::string(tag))
        .push(l("Plot"), Value::Byte(0));
    g
}

fn bytes(g: &Gff) -> Vec<u8> {
    g.to_bytes().unwrap()
}

/// A fresh, empty directory under the system's temporary directory.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rakata-diff-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, data: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, data).unwrap();
}

fn resources(a: (&str, &[u8]), b: (&str, &[u8])) -> Diff {
    diff_resources(a, b, &Options::default())
}

#[test]
fn two_gffs_differ_by_field_path() {
    let d = resources(
        ("a.utc", &bytes(&utc("one"))),
        ("a.utc", &bytes(&utc("two"))),
    );
    assert_eq!(d.entries.len(), 1);
    let e = &d.entries[0];
    assert_eq!(e.format.as_deref(), Some("gff"));
    assert!(e.compared);
    match &e.sites[..] {
        [Site::Field(f)] => {
            assert_eq!(f.path, "Tag");
            assert_eq!(f.before.as_ref().unwrap().value, "\"one\"");
            assert_eq!(f.after.as_ref().unwrap().value, "\"two\"");
        }
        s => panic!("{s:?}"),
    }
    assert!(d.text().contains("Tag: \"one\" -> \"two\""), "{}", d.text());
}

#[test]
fn fields_in_another_order_are_layout_not_a_difference() {
    let a = utc("one");
    let mut b = Gff::new(*b"UTC ");
    b.root
        .push(l("Plot"), Value::Byte(0))
        .push(l("Tag"), Value::string("one"));
    let d = resources(("a.utc", &bytes(&a)), ("a.utc", &bytes(&b)));
    assert!(d.is_same(), "{d:?}");
    assert_eq!(d.same_data, 1);
}

#[test]
fn a_repeated_label_with_its_first_value_is_not_a_difference() {
    let mut a = utc("one");
    a.root.push(l("Plot"), Value::Byte(0));
    let d = resources(("a.utc", &bytes(&a)), ("a.utc", &bytes(&utc("one"))));
    assert!(d.is_same(), "{d:?}");
    // A repeat with a value of its own is data.
    let mut c = utc("one");
    c.root.push(l("Plot"), Value::Byte(1));
    let d = resources(("a.utc", &bytes(&c)), ("a.utc", &bytes(&utc("one"))));
    assert!(!d.is_same());
}

#[test]
fn two_tables_differ_by_row_and_cell() {
    let mut a = TwoDa::new(["name", "type"]);
    a.add_row("0", &[("name", "K_A"), ("type", "Number")])
        .unwrap();
    let mut b = a.clone();
    b.add_row("1", &[("name", "OAJ_DONE"), ("type", "Boolean")])
        .unwrap();
    b.set(0, "type", "Boolean").unwrap();
    let d = resources(
        ("globalcat.2da", &a.to_bytes().unwrap()),
        ("globalcat.2da", &b.to_bytes().unwrap()),
    );
    let sites = &d.entries[0].sites;
    assert!(sites.contains(&Site::Cell {
        row: "0".into(),
        column: "type".into(),
        before: "Number".into(),
        after: "Boolean".into(),
    }));
    assert!(sites.contains(&Site::Row {
        change: Change::Added,
        row: "1".into(),
        cells: vec![
            Cell {
                column: "name".into(),
                value: "OAJ_DONE".into()
            },
            Cell {
                column: "type".into(),
                value: "Boolean".into()
            },
        ],
    }));
}

fn module(members: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut m = Erf::new(ErfKind::Mod);
    for (name, data) in members {
        m.add(ResId::parse(name).unwrap(), data.clone()).unwrap();
    }
    m.to_bytes().unwrap()
}

#[test]
fn a_mod_with_one_member_replaced_names_the_member_and_its_field() {
    let script = b"NCS V1.0 not really".to_vec();
    let a = module(&[
        ("juhani.utc", bytes(&utc("one"))),
        ("k_s.ncs", script.clone()),
    ]);
    let b = module(&[("juhani.utc", bytes(&utc("two"))), ("k_s.ncs", script)]);
    let d = resources(("danm14ac.mod", &a), ("danm14ac.mod", &b));
    assert_eq!(d.entries.len(), 1, "{}", d.text());
    let e = &d.entries[0];
    assert_eq!(e.path, "danm14ac.mod!juhani.utc");
    assert_eq!(e.change, Change::Changed);
    assert!(matches!(&e.sites[..], [Site::Field(f)] if f.path == "Tag"));
    assert_eq!(d.compared, 3, "the archive and its two members");
    assert_eq!(d.same_data, 0, "the archive is listed by its member");
}

#[test]
fn members_added_and_removed_and_a_format_without_a_crate_by_hash() {
    let a = module(&[("k_s.ncs", b"one".to_vec()), ("gone.utc", bytes(&utc("x")))]);
    let b = module(&[("k_s.ncs", b"two".to_vec()), ("new.utc", bytes(&utc("x")))]);
    let d = resources(("m.mod", &a), ("m.mod", &b));
    let by = |p: &str| d.entries.iter().find(|e| e.path == p).unwrap();
    assert_eq!(by("m.mod!gone.utc").change, Change::Removed);
    assert_eq!(by("m.mod!new.utc").change, Change::Added);
    let ncs = by("m.mod!k_s.ncs");
    assert!(!ncs.compared);
    assert_ne!(
        ncs.a.as_ref().unwrap().sha256,
        ncs.b.as_ref().unwrap().sha256
    );
    assert!(d.text().contains("contents not compared"), "{}", d.text());
}

/// A link struct: to `index`, under `cond`.
fn link(index: u32, cond: &str) -> Struct {
    let mut s = Struct::new(0);
    s.push(l("Index"), Value::Dword(index))
        .push(l("Active"), Value::resref(cond).unwrap())
        .push(l("IsChild"), Value::Byte(0));
    s
}

fn start(index: u32, cond: &str) -> Struct {
    let mut s = Struct::new(0);
    s.push(l("Index"), Value::Dword(index))
        .push(l("Active"), Value::resref(cond).unwrap());
    s
}

/// A node voiced `vo` (or with embedded `text` when `vo` is empty),
/// linking to `links`.
fn node(vo: &str, text: &str, links_label: &str, links: Vec<Struct>) -> Struct {
    let mut t = LocString::none();
    if !text.is_empty() {
        t.set(0, text.as_bytes().to_vec());
    }
    let mut s = Struct::new(0);
    s.push(l("Text"), Value::LocString(t))
        .push(l("VO_ResRef"), Value::resref(vo).unwrap())
        .push(l("Speaker"), Value::string(""))
        .push(l(links_label), Value::List(links));
    s
}

fn dlg(entries: Vec<Struct>, replies: Vec<Struct>, starts: Vec<Struct>) -> Vec<u8> {
    let mut g = Gff::new(*b"DLG ");
    g.root
        .push(l("EntryList"), Value::List(entries))
        .push(l("ReplyList"), Value::List(replies))
        .push(l("StartingList"), Value::List(starts));
    bytes(&g)
}

#[test]
fn a_dialogue_differs_by_node_under_stable_keys() {
    // A: E0 "hello" (voiced) -> R0 "bye".
    let a = dlg(
        vec![node("vo_hello", "", "RepliesList", vec![link(0, "")])],
        vec![node("", "Bye.", "EntriesList", vec![])],
        vec![start(0, "")],
    );
    // B inserts a new entry first and a start for it before the old one,
    // so every old index moves; and E1 gains a reply.
    let b = dlg(
        vec![
            node("vo_new", "", "RepliesList", vec![]),
            node(
                "vo_hello",
                "",
                "RepliesList",
                vec![link(0, ""), link(1, "k_cond")],
            ),
        ],
        vec![
            node("", "Bye.", "EntriesList", vec![]),
            node("", "", "EntriesList", vec![]),
        ],
        vec![start(0, "k_new"), start(1, "")],
    );
    let d = resources(("x.dlg", &a), ("x.dlg", &b));
    let e = &d.entries[0];
    assert_eq!(e.format.as_deref(), Some("dlg"));
    let nodes: Vec<&dialog::NodeChange> = e
        .sites
        .iter()
        .filter_map(|s| match s {
            Site::Node(n) => Some(n),
            _ => None,
        })
        .collect();
    let added: Vec<&str> = nodes
        .iter()
        .filter(|n| n.change == Change::Added)
        .map(|n| n.key.as_str())
        .collect();
    assert_eq!(
        added,
        ["voice:vo_new", "index:R1 (fragile)"],
        "{}",
        d.text()
    );
    assert!(nodes.iter().any(|n| n.fragile));
    let changed: Vec<&dialog::NodeChange> = nodes
        .iter()
        .copied()
        .filter(|n| n.change == Change::Changed)
        .collect();
    assert_eq!(changed.len(), 1, "{}", d.text());
    assert_eq!(changed[0].key, "voice:vo_hello");
    assert_eq!(changed[0].a.as_deref(), Some("E0"));
    assert_eq!(changed[0].b.as_deref(), Some("E1"));
    assert_eq!(
        changed[0].fields.len(),
        1,
        "only the link added: {}",
        d.text()
    );
    // The new start is one change; the old one moved and is not.
    let starts: Vec<&FieldChange> = e
        .sites
        .iter()
        .filter_map(|s| match s {
            Site::Field(f) if f.path.starts_with("StartingList") => Some(f),
            _ => None,
        })
        .collect();
    assert_eq!(starts.len(), 1, "{}", d.text());
    assert!(starts[0].before.is_none());
    assert!(
        starts[0]
            .after
            .as_ref()
            .unwrap()
            .value
            .contains("voice:vo_new")
    );
}

/// A tree with an empty `chitin.key` and a module capsule.
fn tree(root: &Path, module_utc: &Gff) {
    write(&root.join("chitin.key"), &Key::new().to_bytes().unwrap());
    let mut rim = Rim::new();
    rim.add(ResId::parse("juhani.utc").unwrap(), bytes(module_utc))
        .unwrap();
    write(&root.join("modules/danm14ac.rim"), &rim.to_bytes().unwrap());
}

#[test]
fn an_override_file_is_compared_with_the_copy_it_replaces() {
    let dir = scratch("override");
    let (a, b) = (dir.join("a"), dir.join("b"));
    tree(&a, &utc("one"));
    tree(&b, &utc("one"));
    write(&b.join("Override/juhani.utc"), &bytes(&utc("two")));
    write(&b.join("Override/new.ncs"), b"NCS");
    let d = diff_trees(&a, &b, &Options::default()).unwrap();
    assert_eq!(d.entries.len(), 2, "{}", d.text());
    let e = &d.entries[0];
    assert_eq!(e.path, "Override/juhani.utc");
    assert_eq!(e.change, Change::Added);
    assert_eq!(e.against, ["modules/danm14ac.rim!juhani.utc"]);
    assert!(
        matches!(&e.sites[..], [Site::Field(f)] if f.path == "Tag"),
        "{}",
        d.text()
    );
    let n = &d.entries[1];
    assert_eq!(n.path, "Override/new.ncs");
    assert!(n.against.is_empty() && n.b.is_some() && n.a.is_none());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn trees_pair_in_any_case_and_report_a_case_only_rename() {
    let dir = scratch("case");
    let (a, b) = (dir.join("a"), dir.join("b"));
    write(&a.join("Override/Juhani.utc"), &bytes(&utc("one")));
    write(&b.join("Override/juhani.utc"), &bytes(&utc("one")));
    write(&a.join("same.txt"), b"x");
    write(&b.join("same.txt"), b"x");
    write(&a.join("gone.txt"), b"x");
    let d = diff_trees(&a, &b, &Options::default()).unwrap();
    let by = |p: &str| d.entries.iter().find(|e| e.path == p);
    let r = by("Override/juhani.utc").expect("the rename");
    assert_eq!(r.change, Change::Renamed);
    assert_eq!(r.renamed_from.as_deref(), Some("Override/Juhani.utc"));
    assert_eq!(by("gone.txt").unwrap().change, Change::Removed);
    assert!(by("same.txt").is_none());
    assert_eq!(d.entries.len(), 2, "{}", d.text());
    let _ = fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn a_linked_directory_is_not_descended_and_a_hard_link_is_not_read() {
    let dir = scratch("links");
    let (a, b) = (dir.join("a"), dir.join("b"));
    write(&a.join("Override/x.utc"), &bytes(&utc("one")));
    fs::create_dir_all(b.join("Override")).unwrap();
    fs::hard_link(a.join("Override/x.utc"), b.join("Override/x.utc")).unwrap();
    std::os::unix::fs::symlink("Override", a.join("override")).unwrap();
    std::os::unix::fs::symlink("Override", b.join("override")).unwrap();
    let d = diff_trees(&a, &b, &Options::default()).unwrap();
    assert!(d.is_same(), "{}", d.text());
    assert_eq!(d.compared, 2, "the file and the link");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn only_keeps_the_paths_named() {
    let dir = scratch("only");
    let (a, b) = (dir.join("a"), dir.join("b"));
    fs::create_dir_all(&a).unwrap();
    write(&b.join("Override/x.utc"), &bytes(&utc("one")));
    write(&b.join("streamwaves/x.wav"), b"RIFF");
    let only = |p: &[&str]| Options {
        only: p.iter().map(|s| s.to_string()).collect(),
        ..Options::default()
    };
    let paths = |o: &Options| -> Vec<String> {
        diff_trees(&a, &b, o)
            .unwrap()
            .entries
            .into_iter()
            .map(|e| e.path)
            .collect()
    };
    assert_eq!(paths(&only(&["override"])), ["Override/x.utc"]);
    assert_eq!(paths(&only(&["*.wav"])), ["streamwaves/x.wav"]);
    assert_eq!(paths(&Options::default()).len(), 2);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_directory_against_a_file_is_re1601() {
    let dir = scratch("kinds");
    write(&dir.join("f.utc"), &bytes(&utc("x")));
    let e = diff(
        dir.to_str().unwrap(),
        dir.join("f.utc").to_str().unwrap(),
        &Options::default(),
    )
    .unwrap_err();
    assert_eq!(e.code.id, "RE1601");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_resource_that_does_not_read_is_compared_by_hash_with_rw1601() {
    let d = resources(("a.utc", b"not a gff"), ("a.utc", b"not a gff either"));
    assert_eq!(d.entries.len(), 1);
    assert!(!d.entries[0].compared);
    assert_eq!(d.warnings[0].code, "RW1601");
}
