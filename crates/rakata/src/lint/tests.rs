//! The rules over installs built here, walked and indexed.

use std::fs;
use std::path::{Path, PathBuf};

use rakata_2da::TwoDa;
use rakata_archive::Key;
use rakata_gff::{Gff, Label, LocString, Value};
use rakata_tlk::{Entry, Tlk};

use super::*;

fn l(t: &str) -> Label {
    Label::new(t).unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rakata-lint-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Override")).unwrap();
    fs::write(dir.join("chitin.key"), Key::new().to_bytes().unwrap()).unwrap();
    dir
}

fn write(path: &Path, data: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, data).unwrap();
}

/// An install whose Override holds a creature naming two scripts (one
/// there, one not), a strref past the talk table, appearance.2da with a
/// model nothing holds, and baseitems.2da with a strref past the table.
fn install(name: &str) -> PathBuf {
    let dir = scratch(name);
    let mut tlk = Tlk::default();
    tlk.append(Entry::new("Bad StrRef"));
    tlk.append(Entry::new("Carth"));
    write(&dir.join("dialog.tlk"), &tlk.to_bytes().unwrap());

    let mut g = Gff::new(*b"UTC ");
    g.root
        .push(l("Tag"), Value::string("carth"))
        .push(l("ScriptSpawn"), Value::resref("k_here").unwrap())
        .push(l("ScriptDeath"), Value::resref("k_gone").unwrap())
        .push(l("ScriptHeartbeat"), Value::resref("").unwrap())
        .push(l("FirstName"), Value::LocString(LocString::strref(1)))
        .push(l("LastName"), Value::LocString(LocString::strref(2)));
    write(&dir.join("Override/carth.utc"), &g.to_bytes().unwrap());
    write(&dir.join("Override/k_here.ncs"), b"NCS V1.0B\0\0\0\x0d");
    write(&dir.join("Override/n_carth.mdl"), b"");

    let mut a = TwoDa::new(["label", "modela", "texa"]);
    a.add_row(
        "0",
        &[
            ("label", "carth"),
            ("modela", "N_Carth"),
            ("texa", "N_CarthT"),
        ],
    )
    .unwrap();
    a.add_row(
        "1",
        &[
            ("label", "nobody"),
            ("modela", "N_Nobody"),
            ("texa", "****"),
        ],
    )
    .unwrap();
    write(&dir.join("Override/appearance.2da"), &a.to_bytes().unwrap());
    let mut b = TwoDa::new(["label", "name"]);
    b.add_row("0", &[("label", "blaster"), ("name", "1")])
        .unwrap();
    b.add_row("1", &[("label", "sword"), ("name", "7")])
        .unwrap();
    b.add_row("2", &[("label", "none"), ("name", "-1")])
        .unwrap();
    write(&dir.join("Override/baseitems.2da"), &b.to_bytes().unwrap());
    dir
}

fn lint(game: &Path, paths: Option<&[String]>, db: Option<&Path>) -> Report {
    run(&Options {
        game,
        profile: Profile::K1,
        paths,
        db,
        known: None,
        threads: 2,
    })
    .unwrap()
}

fn brief(r: &Report) -> Vec<(&str, &str, &str, &str)> {
    r.findings
        .iter()
        .map(|f| (f.code, f.path.as_str(), f.site.as_str(), f.value.as_str()))
        .collect()
}

#[test]
fn dangling_references_and_strrefs_past_the_talk_table() {
    let game = install("rules");
    let r = lint(&game, None, None);
    assert_eq!(r.talk_table, Some(2));
    assert_eq!(
        brief(&r),
        [
            (
                "RW1901",
                "Override/appearance.2da",
                "row 1, column modela",
                "n_nobody"
            ),
            (
                "RW1902",
                "Override/baseitems.2da",
                "row 1, column name",
                "7"
            ),
            ("RW1902", "Override/carth.utc", "LastName", "2"),
            ("RW1901", "Override/carth.utc", "ScriptDeath", "k_gone"),
        ],
        "{:#?}",
        r.findings
    );
    assert_eq!(r.findings[3].want.as_deref(), Some("ncs"));
    assert_eq!((r.checked, r.unread), (4, 0));
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn a_tpc_beside_a_tga_and_names_differing_in_case() {
    let game = scratch("override");
    for f in [
        "a.tpc", "a.tga", "b.tpc", "c.TPC", "C.dds", "Foo.utc", "foo.utc",
    ] {
        write(&game.join("Override").join(f), b"");
    }
    let r = lint(&game, None, None);
    let got: Vec<(&str, &str, &str)> = r
        .findings
        .iter()
        .map(|f| (f.code, f.path.as_str(), f.value.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("RE1901", "Override/a.tpc", "`a.tga`"),
            ("RE1901", "Override/c.TPC", "`C.dds`"),
            ("RW1903", "Override/Foo.utc", "`foo.utc`"),
            ("RW1903", "Override/foo.utc", "`Foo.utc`"),
        ]
    );
    assert!(r.diagnostics()[0].is_error());
    assert!(!r.diagnostics()[2].is_error());

    // Only the listed paths, and the groups they lie in.
    let only = vec!["Override/FOO.utc".to_string(), "Override/c.tpc".to_string()];
    let r = lint(&game, Some(&only), None);
    let codes: Vec<&str> = r.findings.iter().map(|f| f.code).collect();
    assert_eq!(codes, ["RE1901", "RW1903", "RW1903"]);
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn known_findings_are_counted_not_listed() {
    let game = install("known");
    let first = lint(&game, None, None);
    let text = Known::render("accepted", &first.all[..2]);
    let known = Known::parse(&text).unwrap();
    let r = run(&Options {
        game: &game,
        profile: Profile::K1,
        paths: None,
        db: None,
        known: Some(&known),
        threads: 1,
    })
    .unwrap();
    assert_eq!(r.findings.len(), 2);
    assert_eq!(r.known.values().sum::<usize>(), 2);
    let _ = fs::remove_dir_all(&game);
}

#[test]
fn the_index_answers_as_the_walk_does_and_paths_narrow_both() {
    let game = install("index");
    write(&game.join("Override/broken.utc"), b"UTC V3.2");
    let db = game.with_extension("sqlite");
    let _ = fs::remove_file(&db);
    crate::index::build(&crate::index::Build {
        game: &game,
        db: &db,
        label: "t",
        parent: None,
        changed: None,
        threads: 1,
    })
    .unwrap();
    let walked = lint(&game, None, None);
    let indexed = lint(&game, None, Some(&db));
    assert_eq!(indexed.source, "index (tree t)");
    assert_eq!(walked.findings, indexed.findings);
    assert_eq!(walked.checked, indexed.checked);
    assert_eq!((walked.unread, indexed.unread), (1, 1));
    assert!(indexed.warnings.iter().any(|w| w.code.id == "RW1905"));

    let only = vec!["override/CARTH.utc".to_string()];
    let a = lint(&game, Some(&only), None);
    let b = lint(&game, Some(&only), Some(&db));
    assert_eq!(a.findings, b.findings);
    assert_eq!(a.findings.len(), 2);
    assert!(a.findings.iter().all(|f| f.path == "Override/carth.utc"));

    let other = scratch("index-other");
    let e = run(&Options {
        game: &other,
        profile: Profile::K1,
        paths: None,
        db: Some(&db),
        known: None,
        threads: 1,
    })
    .unwrap_err();
    assert_eq!(e.code.id, "RE1903");
    let _ = fs::remove_dir_all(&game);
    let _ = fs::remove_dir_all(&other);
    let _ = fs::remove_file(&db);
}

const NWSCRIPT: &str = "int OBJECT_TYPE_CREATURE = 1;\r\nint OBJECT_TYPE_ITEM = 2;\r\n\
void ExecuteScript(string sScript, object oTarget);\r\n\
object CreateObject(int nObjectType, string sTemplate);\r\n\
void SetCustomToken(int nCustomTokenNumber, string sTokenValue);\r\n";

fn compile(text: &str) -> Vec<u8> {
    let mut r = nwscript::MemResolver::new().with("nwscript.nss", NWSCRIPT);
    let mut map = SourceMap::new();
    let mut c = nwscript::Checker::new();
    let checked = c.check(&mut r, &mut map, "k_test.nss", text.as_bytes());
    let program =
        nwscript::compile::compile(&checked.analysis.unwrap().program, nwscript::Game::K1).unwrap();
    nwscript::ncs::encode(&program)
}

#[test]
fn what_a_script_passes_to_a_resref_parameter_resolves_by_the_parameter() {
    let game = scratch("script");
    write(&game.join("Override/nwscript.nss"), NWSCRIPT.as_bytes());
    write(
        &game.join("Override/k_there.ncs"),
        &compile("void main() {}"),
    );
    write(&game.join("Override/carth.utc"), b"");
    let script = compile(
        "void main() {\n\
         ExecuteScript(\"k_there\", OBJECT_SELF);\n\
         ExecuteScript(\"k_gone\", OBJECT_SELF);\n\
         ExecuteScript(\"\", OBJECT_SELF);\n\
         CreateObject(OBJECT_TYPE_CREATURE, \"carth\");\n\
         CreateObject(OBJECT_TYPE_ITEM, \"carth\");\n\
         SetCustomToken(1, \"not a resref\");\n\
         }",
    );
    write(&game.join("Override/k_test.ncs"), &script);
    let r = lint(&game, None, None);
    let got: Vec<(&str, &str, Option<&str>)> = r
        .findings
        .iter()
        .map(|f| {
            (
                f.value.as_str(),
                f.site.split(' ').nth(1).unwrap(),
                f.want.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            ("carth", "CreateObject.sTemplate", Some("uti")),
            ("k_gone", "ExecuteScript.sScript", Some("ncs")),
        ],
        "{:#?}",
        r.findings
    );
    assert!(r.findings.iter().all(|f| f.path == "Override/k_test.ncs"));
    let _ = fs::remove_dir_all(&game);
}
