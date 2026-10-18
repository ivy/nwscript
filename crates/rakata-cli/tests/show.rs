//! `rakata show` and `rakata decompile` over an install built here, and the
//! acceptance checks over the user's vanilla install when `$RAKATA_GAME_K1`
//! names it. No game bytes are committed.

use assert_cmd::Command;
use rakata::gff::{Gff, Label, LocString, Struct, Value};
use rakata::res::Language;
use rakata::tlk::{Entry, Tlk};
use rakata::twoda::TwoDa;
use rakata_archive::Key;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn rakata() -> Command {
    Command::cargo_bin("rakata").unwrap()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn l(t: &str) -> Label {
    Label::new(t).unwrap()
}

fn table(columns: &[&str], rows: &[&[(&str, &str)]]) -> TwoDa {
    let mut t = TwoDa::new(columns.iter().copied());
    for (i, r) in rows.iter().enumerate() {
        t.add_row(i.to_string(), r).unwrap();
    }
    t
}

/// The strrefs the fixture's talk table holds, in order from 0.
const STRINGS: &[&str] = &[
    "Attack Bonus",   // 0
    "Damage Bonus",   // 1
    "Fire",           // 2
    "+5",             // 3
    "2",              // 4
    "Sword",          // 5
    "The Test Blade", // 6
    "Old Chest",      // 7
];

/// A temporary install: an empty `chitin.key`, the item-property 2DAs and
/// blueprints in `Override/`, and `dialog.tlk`.
struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let o = root.join("Override");
        std::fs::create_dir_all(&o).unwrap();
        std::fs::write(root.join("chitin.key"), Key::new().to_bytes().unwrap()).unwrap();
        let mut tlk = Tlk::new(Language::English);
        for t in STRINGS {
            tlk.append(Entry::new(*t));
        }
        tlk.save(root.join("dialog.tlk")).unwrap();
        let tables = [
            (
                "itempropdef",
                table(
                    &["name", "label", "subtyperesref", "costtableresref"],
                    &[
                        &[
                            ("name", "0"),
                            ("label", "AttackBonus"),
                            ("costtableresref", "1"),
                        ],
                        &[
                            ("name", "1"),
                            ("label", "Damage"),
                            ("subtyperesref", "IPRP_DAMAGETYPE"),
                            ("costtableresref", "2"),
                        ],
                    ],
                ),
            ),
            (
                "iprp_costtable",
                table(
                    &["name", "label"],
                    &[
                        &[("name", "IPRP_BASE1"), ("label", "Base1")],
                        &[("name", "IPRP_BONUSCOST"), ("label", "Bonus")],
                        &[("name", "IPRP_DAMAGECOST"), ("label", "Damage")],
                    ],
                ),
            ),
            ("iprp_base1", table(&["name", "label"], &[])),
            (
                "iprp_bonuscost",
                table(
                    &["name", "label"],
                    &[
                        &[("label", "0")],
                        &[],
                        &[],
                        &[],
                        &[],
                        &[("name", "3"), ("label", "5")],
                    ],
                ),
            ),
            (
                "iprp_damagecost",
                table(
                    &["name", "label"],
                    &[&[], &[], &[("name", "4"), ("label", "2")]],
                ),
            ),
            (
                "iprp_damagetype",
                table(&["name", "label"], &[&[("name", "2"), ("label", "Fire")]]),
            ),
            (
                "baseitems",
                table(&["name", "label"], &[&[("name", "5"), ("label", "Sword")]]),
            ),
        ];
        for (name, t) in tables {
            std::fs::write(o.join(format!("{name}.2da")), t.to_bytes().unwrap()).unwrap();
        }
        std::fs::write(o.join("test_blade.uti"), uti().to_bytes().unwrap()).unwrap();
        std::fs::write(o.join("test_chest.utp"), utp().to_bytes().unwrap()).unwrap();
        std::fs::write(o.join("test_area.git"), git().to_bytes().unwrap()).unwrap();
        Fixture { dir }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }
}

fn property(name: u16, subtype: u16, cost_table: u8, cost_value: u16) -> Struct {
    let mut p = Struct::new(0);
    p.push(l("PropertyName"), Value::Word(name))
        .push(l("Subtype"), Value::Word(subtype))
        .push(l("CostTable"), Value::Byte(cost_table))
        .push(l("CostValue"), Value::Word(cost_value))
        .push(l("Param1"), Value::Byte(255))
        .push(l("Param1Value"), Value::Byte(0));
    p
}

fn uti() -> Gff {
    let mut g = Gff::new(*b"UTI ");
    g.root
        .push(l("TemplateResRef"), Value::resref("test_blade").unwrap())
        .push(l("BaseItem"), Value::Int(0))
        .push(l("LocalizedName"), Value::LocString(LocString::strref(6)))
        .push(l("Tag"), Value::string("test_blade_tag"))
        .push(
            l("PropertiesList"),
            Value::List(vec![property(0, 0, 1, 5), property(1, 0, 2, 2)]),
        );
    g
}

fn utp() -> Gff {
    let mut g = Gff::new(*b"UTP ");
    let mut item = Struct::new(0);
    item.push(l("InventoryRes"), Value::resref("test_blade").unwrap());
    g.root
        .push(l("Tag"), Value::string("test_chest"))
        .push(l("LocName"), Value::LocString(LocString::strref(7)))
        .push(l("OnOpen"), Value::resref("k_open").unwrap())
        .push(l("OnUsed"), Value::resref("").unwrap())
        .push(l("ItemList"), Value::List(vec![item]));
    g
}

fn git() -> Gff {
    let mut g = Gff::new(*b"GIT ");
    let mut chest = Struct::new(9);
    chest.push(l("TemplateResRef"), Value::resref("test_chest").unwrap());
    let mut gone = Struct::new(9);
    gone.push(l("TemplateResRef"), Value::resref("test_gone").unwrap());
    g.root
        .push(l("Placeable List"), Value::List(vec![chest, gone]))
        .push(l("Creature List"), Value::List(vec![]));
    g
}

fn json(args: &[&str]) -> (Option<i32>, Json) {
    let out = rakata()
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code(),
        serde_json::from_slice(&out.stdout).unwrap(),
    )
}

#[test]
fn an_items_properties_resolve_through_the_measured_tables() {
    let f = Fixture::new();
    let out = rakata()
        .args(["show", "test_blade.uti", "--game", s(f.root())])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.starts_with("Override/test_blade.uti (UTI V3.2)\n"),
        "{text}"
    );
    for line in [
        "name: The Test Blade",
        "tag: test_blade_tag",
        "base item: Sword [baseitems 0]",
        "  Attack Bonus: +5 [itempropdef 0, iprp_bonuscost 5]",
        "  Damage Bonus (Fire): 2 [itempropdef 1, iprp_damagetype 0, iprp_damagecost 2]",
    ] {
        assert!(text.lines().any(|t| t == line), "no `{line}` in\n{text}");
    }

    let (code, doc) = json(&["show", "test_blade.uti", "--game", s(f.root())]);
    assert_eq!(code, Some(0));
    let shown = &doc["result"]["shown"];
    assert_eq!(shown["shown"], "item");
    assert_eq!(shown["properties"][1]["subtype"]["label"], "Fire");
    assert_eq!(shown["properties"][1]["value"]["table"], "iprp_damagecost");
}

#[test]
fn a_missing_table_is_a_warning_and_its_rows_show_by_number() {
    let f = Fixture::new();
    std::fs::remove_file(f.root().join("Override/iprp_damagecost.2da")).unwrap();
    let (code, doc) = json(&["show", "test_blade.uti", "--game", s(f.root())]);
    assert_eq!(code, Some(0));
    let codes: Vec<&str> = doc["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["RW2002"]);
    assert_eq!(doc["result"]["shown"]["properties"][1]["value"], Json::Null);
}

#[test]
fn a_placeable_shows_its_events_and_inventory_and_an_area_its_templates() {
    let f = Fixture::new();
    let out = rakata()
        .args(["show", "test_chest.utp", "--game", s(f.root())])
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    for line in [
        "name: Old Chest",
        "OnOpen: k_open",
        "inventory: 1",
        "  test_blade \"The Test Blade\" tag test_blade_tag",
    ] {
        assert!(text.lines().any(|t| t == line), "no `{line}` in\n{text}");
    }
    assert!(
        !text.contains("OnUsed"),
        "an empty script field is left out:\n{text}"
    );

    let out = rakata()
        .args(["show", "test_area.git", "--game", s(f.root())])
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    for line in [
        "placeables: 2",
        "  Placeable List[0] test_chest \"Old Chest\" tag test_chest",
        "    scripts: OnOpen k_open",
        "  Placeable List[1] test_gone (no layer holds the template)",
        "spawned by scripts: not listed; pass --db FILE, an index of the install",
    ] {
        assert!(text.lines().any(|t| t == line), "no `{line}` in\n{text}");
    }
    assert!(err.contains("RW2003"), "{err}");
}

#[test]
fn another_gff_type_prints_its_tree_and_a_wrong_script_name_is_refused() {
    let f = Fixture::new();
    let mut are = Gff::new(*b"ARE ");
    are.root.push(l("Tag"), Value::string("area_tag"));
    let path = f.root().join("x.are");
    std::fs::write(&path, are.to_bytes().unwrap()).unwrap();
    rakata()
        .args(["show", s(&path), "--game", s(f.root())])
        .assert()
        .code(0)
        .stdout(predicates::str::contains("Tag: CExoString = \"area_tag\""));

    let (code, doc) = json(&["decompile", "k_x.nss", "--game", s(f.root())]);
    assert_eq!(code, Some(1));
    assert_eq!(doc["diagnostics"][0]["code"], "RE2001");
    let (code, doc) = json(&["decompile", "k_x", "--game", s(f.root())]);
    assert_eq!(code, Some(1));
    assert_eq!(doc["diagnostics"][0]["code"], "RE0404");
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

fn text(args: &[&str]) -> String {
    let out = rakata().args(args).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn the_vanilla_uthar_condition_decompiles_verified() {
    let Some(game) = install("RAKATA_GAME_K1") else {
        return;
    };
    let t = text(&[
        "decompile",
        "k35_uth_hassword.ncs",
        "--module",
        "korr_m35aa",
        "--game",
        s(&game),
    ]);
    assert!(
        t.starts_with("// modules/korr_m35aa_s.rim:k35_uth_hassword.ncs (module korr_m35aa)\n"),
        "{t}"
    );
    assert!(
        t.contains("k35_uth_hassword [verified, 1 of 1 functions]"),
        "{t}"
    );
    assert!(
        t.contains("GetItemPossessedBy(oLocal2, \"k37_itm_freedont\")"),
        "{t}"
    );

    // A script whose include library folds, its module found unnamed.
    let t = text(&["decompile", "k37_sta_taketrue", "--game", s(&game)]);
    assert!(t.contains("(module korr_m37aa)"), "{t}");
    assert!(t.contains("#include \"k_inc_utility\""), "{t}");
}

#[test]
fn the_vanilla_sword_of_ajunta_pall() {
    let Some(game) = install("RAKATA_GAME_K1") else {
        return;
    };
    let t = text(&[
        "show",
        "k37_itm_ajunta.uti",
        "--module",
        "korr_m37aa",
        "--game",
        s(&game),
    ]);
    for line in [
        "name: Ajunta Pall's Blade",
        "tag: k37_itm_freedont",
        "base item: Double-Bladed Sword [baseitems 6]",
        "properties: 3",
        "  Attack Bonus: 5 [itempropdef 38, iprp_meleecost 5]",
        "  Damage Bonus (Fire): 2 [itempropdef 11, iprp_damagetype 8, iprp_damagecost 2]",
        "  Damage Bonus (Slashing): 3 [itempropdef 11, iprp_damagetype 2, iprp_damagecost 3]",
    ] {
        assert!(t.lines().any(|x| x == line), "no `{line}` in\n{t}");
    }
}

#[test]
fn the_vanilla_tomb_and_what_its_scripts_spawn() {
    let Some(game) = install("RAKATA_GAME_K1") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("idx.sqlite");
    text(&[
        "index",
        "build",
        "--game",
        s(&game),
        "--db",
        s(&db),
        "--tree",
        "vanilla",
    ]);
    let t = text(&[
        "show",
        "m37aa.git",
        "--module",
        "korr_m37aa",
        "--game",
        s(&game),
        "--db",
        s(&db),
    ]);
    assert!(t.lines().any(|x| x == "placeables: 28"), "{t}");
    assert!(
        t.lines()
            .any(|x| x == "  Placeable List[2] k37_statue \"Sith Statue\" tag k37_statue"),
        "{t}"
    );
    assert!(
        t.lines().any(
            |x| x.starts_with("  kor37_freedon (creature) ") && x.contains(" by k37_opensarc ")
        ),
        "{t}"
    );
}
