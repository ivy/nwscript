//! A tree made here, from the format crates' writers: built cold, built
//! again from its parent and a changed list, and compared by digest with a
//! cold build of the same tree.

use std::fs;
use std::path::{Path, PathBuf};

use rakata_2da::TwoDa;
use rakata_archive::{Erf, ErfKind};
use rakata_gff::{Gff, Label, LocString, Struct, Value};
use rakata_index::{ArgType, BuildOptions, Param, Routine, build, coverage, digest, open, query};
use rakata_res::{ResId, ResRef};

fn label(s: &str) -> Label {
    Label::new(s).unwrap()
}

fn utc(tag: &str) -> Vec<u8> {
    let mut g = Gff::new(*b"UTC ");
    g.root
        .push(label("Tag"), Value::string(tag))
        .push(
            label("Conversation"),
            Value::ResRef(ResRef::new("dan14_juhani").unwrap()),
        )
        .push(
            label("FirstName"),
            Value::LocString(LocString::strref(42724)),
        );
    g.to_bytes().unwrap()
}

fn dlg(text: &str) -> Vec<u8> {
    let mut g = Gff::new(*b"DLG ");
    let mut entry = Struct::new(0);
    let mut t = LocString::none();
    t.set(0, text.as_bytes().to_vec());
    entry
        .push(label("Speaker"), Value::string("Juhani"))
        .push(label("Text"), Value::LocString(t))
        .push(
            label("Script"),
            Value::ResRef(ResRef::new("k_pdan_juhani27").unwrap()),
        )
        .push(label("RepliesList"), Value::List(Vec::new()));
    let mut start = Struct::new(0);
    start.push(label("Index"), Value::Dword(0));
    g.root
        .push(label("EntryList"), Value::List(vec![entry]))
        .push(label("ReplyList"), Value::List(Vec::new()))
        .push(label("StartingList"), Value::List(vec![start]));
    g.to_bytes().unwrap()
}

fn ncs() -> Vec<u8> {
    let p = nwscript_ncs::asm::parse(
        nwscript_diag::FileId(0),
        b"JSR main\nRETN\nmain:\nCONSTI -1\nCONSTO 0\nCONSTS \"k_pman_aioff\"\nACTION 0, 3\nRETN\n",
    )
    .unwrap();
    nwscript_ncs::encode(&p)
}

fn routines() -> Vec<Routine> {
    let p = |name: &str, ty| Param {
        name: name.into(),
        ty,
    };
    vec![Routine {
        name: "ExecuteScript".into(),
        params: vec![
            p("sScript", ArgType::String),
            p("oTarget", ArgType::Object),
            p("nScriptVar", ArgType::Int),
        ],
    }]
}

fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}

/// A small tree: Override/ with a table, a blueprint, a dialogue and a
/// script; a module holding the same blueprint and a second dialogue; and
/// an executable outside the walk.
fn tree(root: &Path) {
    let mut cat = TwoDa::new(["name", "type"]);
    cat.add_row("0", &[("name", "K_JUHANI_CLONES"), ("type", "Number")])
        .unwrap();
    write(root, "Override/globalcat.2da", &cat.to_bytes().unwrap());
    write(root, "Override/p_juhani.utc", &utc("Juhani"));
    write(root, "Override/dan14_juhani.dlg", &dlg("Bad Dog"));
    write(root, "Override/k_pdan_juhani27.ncs", &ncs());
    let mut m = Erf::new(ErfKind::Mod);
    m.add(ResId::parse("p_juhani.utc").unwrap(), utc("Juhani"))
        .unwrap();
    m.add(
        ResId::parse("dan14_cutjuh.dlg").unwrap(),
        dlg("rakghouls below"),
    )
    .unwrap();
    write(root, "modules/danm14ac.mod", &m.to_bytes().unwrap());
    write(root, "swkotor.exe", b"MZ");
}

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for dir in ["Override", "modules"] {
        for e in fs::read_dir(root.join(dir)).unwrap() {
            v.push(e.unwrap().path());
        }
    }
    v.sort();
    v
}

fn opts<'a>(
    root: &'a Path,
    files: &'a [PathBuf],
    db: &'a Path,
    label: &'a str,
    routines: &'a [Routine],
) -> BuildOptions<'a> {
    BuildOptions {
        root,
        files,
        db,
        label,
        parent: None,
        changed: None,
        routines,
        threads: 2,
    }
}

#[test]
fn an_incremental_build_equals_a_cold_one_and_parses_only_what_changed() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    tree(&a);
    tree(&b);
    // The child tree changes the blueprint and adds a dialogue.
    write(&b, "Override/p_juhani.utc", &utc("Juhani2"));
    write(&b, "Override/oaj_banter.dlg", &dlg("Bad Dog again"));
    let routines = routines();

    let chain = dir.path().join("chain.sqlite");
    let files_a = walk(&a);
    let ra = build(&opts(&a, &files_a, &chain, "a", &routines)).unwrap();
    assert_eq!(ra.files, 5);
    assert_eq!(ra.outside, 1);
    assert_eq!(ra.members_read, 2);
    // The module's blueprint is the Override one, byte for byte: parsed once.
    assert_eq!(ra.hashes_reused, 1);
    assert_eq!(ra.coverage["container"], 1);
    assert_eq!(ra.coverage["outside"], 1);
    assert_eq!(ra.coverage["parsed"], 6);

    let files_b = walk(&b);
    let changed = vec![
        "override/p_juhani.utc".to_string(),
        "Override/oaj_banter.dlg".to_string(),
    ];
    let rb = build(&BuildOptions {
        parent: Some("a"),
        changed: Some(&changed),
        ..opts(&b, &files_b, &chain, "b", &routines)
    })
    .unwrap();
    assert_eq!(rb.files_reused, 4);
    assert_eq!(rb.files_read, 2);
    assert_eq!(rb.hashes_parsed, 2);
    assert!(rb.warnings.is_empty(), "{:?}", rb.warnings);

    let cold = dir.path().join("cold.sqlite");
    build(&opts(&b, &files_b, &cold, "b", &routines)).unwrap();
    let chain_db = open(&chain).unwrap();
    let cold_db = open(&cold).unwrap();
    assert_eq!(
        digest(&chain_db, Some("b")).unwrap(),
        digest(&cold_db, Some("b")).unwrap()
    );
    assert_ne!(
        digest(&chain_db, Some("a")).unwrap(),
        digest(&chain_db, Some("b")).unwrap()
    );

    // The new dialogue first appears in the child.
    let rows = query(
        &chain_db,
        "SELECT t.label FROM resources r JOIN trees t ON t.id = r.first_tree WHERE r.name = 'oaj_banter'",
    )
    .unwrap();
    assert_eq!(rows.rows, [[serde_json::json!("b")]]);

    // The call argument, and full text over nodes.
    let rows = query(
        &chain_db,
        "SELECT DISTINCT l.path FROM calls c JOIN locations l ON l.hash = c.hash WHERE json_extract(c.args, '$[0]') = 'k_pman_aioff'",
    )
    .unwrap();
    assert_eq!(
        rows.rows,
        [[serde_json::json!("Override/k_pdan_juhani27.ncs")]]
    );
    let rows = query(
        &chain_db,
        "SELECT DISTINCT l.member, s.site FROM search s JOIN locations l ON l.hash = s.hash WHERE search MATCH 'rakghoul*'",
    )
    .unwrap();
    assert_eq!(
        rows.rows,
        [[
            serde_json::json!("dan14_cutjuh.dlg"),
            serde_json::json!("E0")
        ]]
    );
    let rows = query(
        &chain_db,
        "SELECT kind, value FROM refs WHERE site = 'row 0' OR value IN ('dan14_juhani', '42724') GROUP BY 1, 2 ORDER BY 1",
    )
    .unwrap();
    assert_eq!(
        rows.rows,
        [
            [
                serde_json::json!("global"),
                serde_json::json!("K_JUHANI_CLONES")
            ],
            [
                serde_json::json!("resref"),
                serde_json::json!("dan14_juhani")
            ],
            [serde_json::json!("strref"), serde_json::json!("42724")],
        ]
    );

    // Every file of the tree has a coverage row.
    let cov = coverage(&chain_db, Some("b"), true).unwrap();
    assert_eq!(cov[0].files, 7);
    assert_eq!(cov[0].members, 2);
    let rows = cov[0].rows.as_ref().unwrap();
    assert!(
        rows.iter()
            .any(|r| r.path == "swkotor.exe" && r.status == "outside")
    );

    // A read-only database refuses a write.
    let e = query(&chain_db, "DELETE FROM refs").unwrap_err();
    assert_eq!(e.code(), rakata_index::codes::QUERY);
}

#[test]
fn a_missing_parent_and_unlisted_files_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    tree(&a);
    let routines = routines();
    let db = dir.path().join("x.sqlite");
    let files = walk(&a);
    let e = build(&BuildOptions {
        parent: Some("nope"),
        ..opts(&a, &files, &db, "a", &routines)
    })
    .unwrap_err();
    assert_eq!(e.code(), rakata_index::codes::NO_SUCH_TREE);

    build(&opts(&a, &files, &db, "a", &routines)).unwrap();
    write(
        &a,
        "Override/new.2da",
        &TwoDa::new(["x"]).to_bytes().unwrap(),
    );
    let files = walk(&a);
    let changed = vec!["Override/gone.ncs".to_string()];
    let r = build(&BuildOptions {
        parent: Some("a"),
        changed: Some(&changed),
        ..opts(&a, &files, &db, "a2", &routines)
    })
    .unwrap();
    let codes: Vec<&str> = r.warnings.iter().map(|w| w.code).collect();
    assert_eq!(codes, ["RW1501", "RW1504"]);
    assert_eq!(r.files_read, 1);

    // Building a tree again under its label replaces it.
    let r = build(&opts(&a, &files, &db, "a", &routines)).unwrap();
    assert!(r.replaced);
    let c = open(&db).unwrap();
    let rows = query(&c, "SELECT count(*) FROM trees").unwrap();
    assert_eq!(rows.rows, [[serde_json::json!(2)]]);
}
