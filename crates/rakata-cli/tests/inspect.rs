//! `rakata cat`, `gff`, `2da`, `tlk` and `lip` over fixtures built here, and
//! the acceptance checks over the user's installs when `$RAKATA_GAME_K1` and
//! `$RAKATA_BUILD` name them.

use assert_cmd::Command;
use predicates::prelude::*;
use rakata::gff::{Gff, Label, LocString, Value};
use rakata::lip::{Lip, Shape};
use rakata::res::{Language, ResId};
use rakata::tlk::{Entry, Tlk};
use rakata::twoda::TwoDa;
use rakata_archive::{Erf, ErfKind, Key};
use serde_json::Value as Json;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// A creature blueprint with a field of most kinds.
fn utc() -> Gff {
    let mut g = Gff::new(*b"UTC ");
    let l = |t: &str| Label::new(t).unwrap();
    let mut first = LocString::none();
    first.set(0, b"Juhani".to_vec());
    let mut class = rakata::gff::Struct::new(2);
    class.push(l("Class"), Value::Int(3));
    class.push(l("ClassLevel"), Value::Short(9));
    g.root
        .push(l("Tag"), Value::string("dan14_juhani"))
        .push(l("TemplateResRef"), Value::resref("dan14_juhani").unwrap())
        .push(l("FirstName"), Value::LocString(first))
        .push(l("Plot"), Value::Byte(0))
        .push(l("ChallengeRating"), Value::Float(1.5))
        .push(l("ClassList"), Value::List(vec![class]));
    g
}

fn table() -> TwoDa {
    let mut t = TwoDa::new(["name", "type"]);
    t.add_row("0", &[("name", "K_SWG_HELENA"), ("type", "Number")])
        .unwrap();
    t.add_row("1", &[("name", "K_SWG_DUSTIL"), ("type", "")])
        .unwrap();
    t
}

/// A temporary install: an empty `chitin.key`, `Override/p_juhani.utc`,
/// `modules/testm.mod` holding `globalcat.2da` and a lip, and
/// `dialog.tlk`.
struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Override")).unwrap();
        std::fs::create_dir_all(root.join("modules")).unwrap();
        std::fs::write(root.join("chitin.key"), Key::new().to_bytes().unwrap()).unwrap();
        std::fs::write(
            root.join("Override/p_juhani.utc"),
            utc().to_bytes().unwrap(),
        )
        .unwrap();
        let mut m = Erf::new(ErfKind::Mod);
        m.add(
            ResId::parse("globalcat.2da").unwrap(),
            table().to_bytes().unwrap(),
        )
        .unwrap();
        let lip = Lip::from_cues([(0.0, Shape::Rest), (0.25, Shape::Oh)], 0.5);
        m.add(ResId::parse("line.lip").unwrap(), lip.to_bytes().unwrap())
            .unwrap();
        std::fs::write(root.join("modules/testm.mod"), m.to_bytes().unwrap()).unwrap();
        let mut tlk = Tlk::new(Language::English);
        tlk.append(Entry::new("Bad Pazaak"));
        tlk.append(Entry::new("You must gather your party."));
        tlk.save(root.join("dialog.tlk")).unwrap();
        Fixture { dir }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn path(&self, p: &str) -> PathBuf {
        self.dir.path().join(p)
    }
}

fn json(cmd: &mut Command) -> (Option<i32>, Json) {
    let out = cmd.output().unwrap();
    let mut docs = serde_json::Deserializer::from_slice(&out.stdout).into_iter::<Json>();
    let doc = docs
        .next()
        .unwrap_or_else(|| panic!("no JSON: {}", String::from_utf8_lossy(&out.stderr)))
        .unwrap();
    assert!(docs.next().is_none(), "more than one JSON document");
    (out.status.code(), doc)
}

#[test]
fn every_inspection_help_runs_and_has_an_example() {
    for args in [
        &["cat", "--help"][..],
        &["gff", "--help"],
        &["gff", "show", "--help"],
        &["gff", "set", "--help"],
        &["gff", "diff", "--help"],
        &["2da", "--help"],
        &["2da", "show", "--help"],
        &["2da", "diff", "--help"],
        &["tlk", "--help"],
        &["tlk", "show", "--help"],
        &["tlk", "append", "--help"],
        &["lip", "--help"],
        &["lip", "show", "--help"],
    ] {
        let out = rakata().args(args).output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains("Example"), "{args:?} --help has no example");
    }
}

#[test]
fn a_name_resolves_through_the_install_and_a_path_reads_the_file() {
    let f = Fixture::new();
    let bytes = std::fs::read(f.path("Override/p_juhani.utc")).unwrap();
    rakata()
        .args(["--game", s(f.root()), "cat", "p_juhani.utc"])
        .assert()
        .code(0)
        .stdout(bytes.clone());
    rakata()
        .args(["cat", s(&f.path("Override/p_juhani.utc"))])
        .assert()
        .code(0)
        .stdout(bytes);
}

#[test]
fn a_module_member_resolves_with_module_and_by_archive_member() {
    let f = Fixture::new();
    let expected = table().to_bytes().unwrap();
    rakata()
        .args(["--game", s(f.root()), "cat", "globalcat.2da", "-m", "testm"])
        .assert()
        .code(0)
        .stdout(expected.clone());
    rakata()
        .args([
            "--game",
            s(f.root()),
            "cat",
            "modules/TESTM.mod!globalcat.2da",
        ])
        .assert()
        .code(0)
        .stdout(expected.clone());
    rakata()
        .args([
            "cat",
            "globalcat.2da",
            "--from",
            s(&f.path("modules/testm.mod")),
        ])
        .assert()
        .code(0)
        .stdout(expected);
}

#[test]
fn a_missing_member_is_re1301_and_a_missing_resource_re0404() {
    let f = Fixture::new();
    rakata()
        .args(["--game", s(f.root()), "cat", "modules/testm.mod!nope.2da"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE1301"));
    rakata()
        .args(["--game", s(f.root()), "cat", "nope.2da"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE0404"));
    rakata()
        .args(["cat", "nope.2da"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE0001").and(predicate::str::contains("--game")));
}

#[test]
fn cat_json_records_the_input_and_writes_the_output() {
    let f = Fixture::new();
    let out = f.path("copy.utc");
    let (code, doc) = json(rakata().args([
        "--format",
        "json",
        "--game",
        s(f.root()),
        "cat",
        "p_juhani.utc",
        "-o",
        s(&out),
    ]));
    assert_eq!(code, Some(0));
    assert_eq!(doc["command"], "cat");
    assert_eq!(doc["inputs"][0]["path"], "Override/p_juhani.utc");
    assert_eq!(doc["outputs"][0]["path"], s(&out));
    assert_eq!(
        std::fs::read(out).unwrap(),
        std::fs::read(f.path("Override/p_juhani.utc")).unwrap()
    );
}

#[test]
fn gff_show_prints_the_tree_and_a_path_prints_the_value() {
    let f = Fixture::new();
    let file = f.path("Override/p_juhani.utc");
    rakata()
        .args(["gff", "show", s(&file)])
        .assert()
        .code(0)
        .stdout(
            predicate::str::contains("UTC V3.2")
                .and(predicate::str::contains(
                    "Tag: CExoString = \"dan14_juhani\"",
                ))
                .and(predicate::str::contains(
                    "FirstName: CExoLocString = strref -1, 0=\"Juhani\"",
                ))
                .and(predicate::str::contains("  [0] struct 2"))
                .and(predicate::str::contains("    ClassLevel: SHORT = 9")),
        );
    rakata()
        .args([
            "--game",
            s(f.root()),
            "gff",
            "show",
            "p_juhani.utc",
            "--path",
            "Tag",
        ])
        .assert()
        .code(0)
        .stdout("dan14_juhani\n");
    rakata()
        .args(["gff", "show", s(&file), "--path", "ClassList[0].Class"])
        .assert()
        .code(0)
        .stdout("3\n");
    rakata()
        .args(["gff", "show", s(&file), "--path", "Nope"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE0607"));
}

#[test]
fn gff_show_json_types_every_value() {
    let f = Fixture::new();
    let (code, doc) = json(rakata().args([
        "--format",
        "json",
        "gff",
        "show",
        s(&f.path("Override/p_juhani.utc")),
    ]));
    assert_eq!(code, Some(0));
    assert_eq!(doc["result"]["file_type"], "UTC");
    let fields = doc["result"]["root"]["fields"].as_array().unwrap();
    assert_eq!(fields[0]["label"], "Tag");
    assert_eq!(fields[0]["type"], "CExoString");
    assert_eq!(fields[0]["value"], "dan14_juhani");
    assert_eq!(fields[2]["value"]["strref"], Json::Null);
    assert_eq!(fields[2]["value"]["substrings"][0]["text"], "Juhani");
    assert_eq!(fields[5]["value"][0]["fields"][1]["value"], 9);
}

#[test]
fn gff_set_round_trips_and_leaves_the_rest_identical() {
    let f = Fixture::new();
    let input = f.path("Override/p_juhani.utc");
    let out = f.path("out.utc");
    rakata()
        .args(["gff", "set", s(&input), "Tag=juhani2", "-o", s(&out)])
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "Tag: \"dan14_juhani\" -> \"juhani2\"",
        ));
    rakata()
        .args(["gff", "show", s(&out), "--path", "Tag"])
        .assert()
        .code(0)
        .stdout("juhani2\n");
    let (code, doc) = json(rakata().args(["--format", "json", "gff", "diff", s(&input), s(&out)]));
    assert_eq!(code, Some(3));
    let changes = doc["result"]["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(changes[0]["path"], "Tag");
    assert_eq!(changes[0]["after"]["value"], "juhani2");

    // Setting the value back writes the input's bytes.
    let back = f.path("back.utc");
    rakata()
        .args(["gff", "set", s(&out), "Tag=dan14_juhani", "-o", s(&back)])
        .assert()
        .code(0);
    assert_eq!(
        std::fs::read(&back).unwrap(),
        std::fs::read(&input).unwrap()
    );
    rakata()
        .args(["gff", "diff", s(&input), s(&back)])
        .assert()
        .code(0)
        .stdout("no differences\n");
}

#[test]
fn gff_set_reads_types_and_refuses_an_untyped_new_field() {
    let f = Fixture::new();
    let input = f.path("Override/p_juhani.utc");
    let out = f.path("out.utc");
    rakata()
        .args([
            "gff",
            "set",
            s(&input),
            "FirstName=locstring:0=Jo",
            "ChallengeRating=2.25",
            "ClassList[0].ClassLevel=10",
            "Plot=byte:1",
            "NewField=int:-5",
            "-o",
            s(&out),
        ])
        .assert()
        .code(0);
    for (path, want) in [
        ("FirstName", "strref -1, 0=\"Jo\"\n"),
        ("ChallengeRating", "2.25\n"),
        ("ClassList[0].ClassLevel", "10\n"),
        ("Plot", "1\n"),
        ("NewField", "-5\n"),
    ] {
        rakata()
            .args(["gff", "show", s(&out), "--path", path])
            .assert()
            .code(0)
            .stdout(want);
    }
    rakata()
        .args(["gff", "set", s(&input), "Missing=1", "-o", s(&out)])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE1303"));
    rakata()
        .args(["gff", "set", s(&input), "Plot=300", "-o", s(&out)])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE1302"));
}

#[test]
fn twoda_show_prints_the_table_a_row_a_column_and_a_cell() {
    let f = Fixture::new();
    let game = s(f.root());
    let base = [
        "--game",
        game,
        "2da",
        "show",
        "globalcat.2da",
        "-m",
        "testm",
    ];
    rakata()
        .args(base)
        .assert()
        .code(0)
        .stdout("\tname\ttype\n0\tK_SWG_HELENA\tNumber\n1\tK_SWG_DUSTIL\t****\n");
    rakata()
        .args(base)
        .args(["--row", "1"])
        .assert()
        .code(0)
        .stdout("name\tK_SWG_DUSTIL\ntype\t****\n");
    rakata()
        .args(base)
        .args(["--column", "name"])
        .assert()
        .code(0)
        .stdout("0\tK_SWG_HELENA\n1\tK_SWG_DUSTIL\n");
    rakata()
        .args(base)
        .args(["--row", "0", "--column", "type"])
        .assert()
        .code(0)
        .stdout("Number\n");
    rakata()
        .args(base)
        .args(["--column", "Name"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE0706"));
}

#[test]
fn twoda_diff_lists_added_rows_and_changed_cells_and_exits_3() {
    let f = Fixture::new();
    let mut b = table();
    b.set("0", "type", "Boolean").unwrap();
    b.add_row("2", &[("name", "OAJ_SPECIAL"), ("type", "Boolean")])
        .unwrap();
    let bp = f.path("b.2da");
    std::fs::write(&bp, b.to_bytes().unwrap()).unwrap();
    rakata()
        .args([
            "--game",
            s(f.root()),
            "2da",
            "diff",
            "globalcat.2da",
            s(&bp),
            "--module-a",
            "testm",
        ])
        .assert()
        .code(3)
        .stdout(
            "row `0`, column `type`: Number -> Boolean\nrow `2` added: name=OAJ_SPECIAL type=Boolean\n",
        );
    rakata()
        .args(["2da", "diff", s(&bp), s(&bp)])
        .assert()
        .code(0)
        .stdout("no differences\n");
}

#[test]
fn tlk_show_by_strref_and_text_and_append_prints_the_strref() {
    let f = Fixture::new();
    rakata()
        .args(["--game", s(f.root()), "tlk", "show", "1"])
        .assert()
        .code(0)
        .stdout("You must gather your party.\n");
    rakata()
        .args(["--game", s(f.root()), "tlk", "show", "--text", "PAZAAK"])
        .assert()
        .code(0)
        .stdout("0\tBad Pazaak\n");
    rakata()
        .args(["--game", s(f.root()), "tlk", "show", "7"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("RE1304"));
    rakata()
        .args(["tlk", "show", "1"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--game"));
    let out = f.path("out.tlk");
    rakata()
        .args([
            "tlk",
            "append",
            s(&f.path("dialog.tlk")),
            "--text",
            "Hello there.",
            "--sound",
            "n_juhani_01",
            "-o",
            s(&out),
        ])
        .assert()
        .code(0)
        .stdout("2\n");
    let (code, doc) =
        json(rakata().args(["--format", "json", "tlk", "show", "2", "--from", s(&out)]));
    assert_eq!(code, Some(0));
    assert_eq!(doc["result"]["text"], "Hello there.");
    assert_eq!(doc["result"]["sound"], "n_juhani_01");
}

#[test]
fn lip_show_prints_the_keyframes() {
    let f = Fixture::new();
    rakata()
        .args([
            "--game",
            s(f.root()),
            "lip",
            "show",
            "modules/testm.mod!line.lip",
        ])
        .assert()
        .code(0)
        .stdout("length 0.5, 3 keyframes\n0\trest\n0.25\toh\n0.5\trest\n");
    let (code, doc) = json(rakata().args([
        "--format",
        "json",
        "--game",
        s(f.root()),
        "lip",
        "show",
        "line.lip",
        "-m",
        "testm",
    ]));
    assert_eq!(code, Some(0));
    assert_eq!(doc["result"]["keys"][1]["label"], "oh");
    assert_eq!(doc["result"]["keys"][1]["shape"], 4);
}

/// The install a variable names, or `None` after saying which is unset.
fn install(var: &str) -> Option<PathBuf> {
    match std::env::var_os(var) {
        Some(v) => Some(PathBuf::from(v)),
        None => {
            eprintln!("skipped: ${var} is not set; it names a KotOR 1 install");
            None
        }
    }
}

#[test]
fn the_build_juhanis_tag() {
    let Some(build) = install("RAKATA_BUILD") else {
        return;
    };
    rakata()
        .args(["gff", "show", "dan14_juhani.utc", "--game", s(&build)])
        .args(["--module", "danm14ac", "--path", "Tag"])
        .assert()
        .code(0)
        .stdout("dan14_juhani\n");
}

#[test]
fn the_build_adds_the_juhani_globals() {
    let (Some(vanilla), Some(build)) = (install("RAKATA_GAME_K1"), install("RAKATA_BUILD")) else {
        return;
    };
    let out = rakata()
        .args(["2da", "diff", "globalcat.2da", "globalcat.2da"])
        .args(["--game-a", s(&vanilla), "--game-b", s(&build)])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let text = String::from_utf8(out.stdout).unwrap();
    for name in ["OAJ_SPECIAL", "OAJ_DONE"] {
        assert!(
            text.lines()
                .any(|l| l.contains("added") && l.contains(&format!("name={name} "))),
            "{name} not added: {text}"
        );
    }
}

#[test]
fn the_build_talk_table_strref() {
    let Some(build) = install("RAKATA_BUILD") else {
        return;
    };
    let out = rakata()
        .args(["tlk", "show", "42724", "--game", s(&build)])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(!text.trim().is_empty());
    let (code, doc) = json(rakata().args([
        "--format",
        "json",
        "tlk",
        "show",
        "42724",
        "--game",
        s(&build),
    ]));
    assert_eq!(code, Some(0));
    assert_eq!(doc["command"], "tlk show");
    assert_eq!(doc["result"]["strref"], 42724);
    assert_eq!(doc["result"]["text"], text.trim_end_matches('\n'));
}
