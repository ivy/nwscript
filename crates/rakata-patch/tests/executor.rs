//! The executor over a hand-made install: a KEY and one BIF, a talk table,
//! a module as a `.mod` and as a pair of RIMs, and Override. Each test
//! copies a fixture's changes file into a fresh `tslpatchdata`, writes the
//! binary files it names there, installs, and reads the tree back with the
//! format crates. No game bytes; every file is built here.

use rakata_2da::TwoDa;
use rakata_archive::{Bif, BifMember, BifRef, Erf, ErfKind, Key, KeyEntry, Rim};
use rakata_gff::{Gff, Label, LocString, Path as GffPath, Struct, Value};
use rakata_patch::exec::{Executor, Outcome, Report, Target};
use rakata_patch::ssf::Ssf;
use rakata_patch::{Plan, codes, load};
use rakata_res::{ContentHash, ResId, ResType};
use rakata_tlk::{Entry, Tlk};
use std::fs;
use std::path::{Path, PathBuf};

const NWSCRIPT: &str =
    "int TRUE = 1;\r\nvoid PrintInteger(int n);\r\nvoid PrintString(string s);\r\n";

fn id(text: &str) -> ResId {
    ResId::parse(text).unwrap()
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/exec")
}

fn appearance() -> TwoDa {
    let mut t = TwoDa::new(["label", "name", "race"]);
    t.add_row("0", &[("label", "Carth"), ("name", "100"), ("race", "3")])
        .unwrap();
    t.add_row("1", &[("label", "Bastila"), ("name", "101"), ("race", "5")])
        .unwrap();
    t
}

fn creature() -> Gff {
    let mut g = Gff::new(*b"UTC ");
    g.root
        .push(Label::new("Tag").unwrap(), Value::string("p_test"))
        .push(Label::new("Appearance_Type").unwrap(), Value::Word(1))
        .push(
            Label::new("FirstName").unwrap(),
            Value::LocString(LocString::strref(5)),
        )
        .push(Label::new("ItemList").unwrap(), Value::List(Vec::new()));
    g
}

fn soundset() -> Vec<u8> {
    Ssf::new().to_bytes()
}

/// The hand-made install, under `dir`.
fn game(dir: &Path) -> PathBuf {
    let root = dir.join("game");
    for sub in ["Override", "modules", "data"] {
        fs::create_dir_all(root.join(sub)).unwrap();
    }
    let mut bif = Bif::new();
    let members: Vec<(&str, Vec<u8>)> = vec![
        ("appearance.2da", appearance().to_bytes().unwrap()),
        ("nwscript.nss", NWSCRIPT.as_bytes().to_vec()),
        (
            "k_inc_test.nss",
            b"int TestValue() { return 7; }\r\n".to_vec(),
        ),
        ("p_test.utc", creature().to_bytes().unwrap()),
    ];
    for (i, (name, bytes)) in members.iter().enumerate() {
        bif.push(BifMember::owned(i as u32, id(name).ty, bytes.clone()));
    }
    let bif_bytes = bif.to_bytes().unwrap();
    fs::write(root.join("data/templates.bif"), &bif_bytes).unwrap();
    let mut key = Key::new();
    key.add_bif(BifRef {
        size: bif_bytes.len() as u32,
        name: b"data\\templates.bif".to_vec(),
        drives: 1,
    });
    for (i, (name, _)) in members.iter().enumerate() {
        key.add(KeyEntry {
            id: id(name),
            bif: 0,
            index: i as u32,
        })
        .unwrap();
    }
    fs::write(root.join("chitin.key"), key.to_bytes().unwrap()).unwrap();

    let mut tlk = Tlk::default();
    for i in 0..3 {
        tlk.append(Entry::new(format!("vanilla line {i}")));
    }
    fs::write(root.join("dialog.tlk"), tlk.to_bytes().unwrap()).unwrap();

    let mut m = Erf::new(ErfKind::Mod);
    m.add(id("k_test.ncs"), b"old bytecode".to_vec()).unwrap();
    m.add(id("m_npc.utc"), creature().to_bytes().unwrap())
        .unwrap();
    fs::write(root.join("modules/testmod.mod"), m.to_bytes().unwrap()).unwrap();
    let mut rim = Rim::new();
    rim.add(id("other.are"), b"area".to_vec()).unwrap();
    fs::write(root.join("modules/other.rim"), rim.to_bytes().unwrap()).unwrap();
    let mut s_rim = Rim::new();
    s_rim
        .add(id("k_other.ncs"), b"other script".to_vec())
        .unwrap();
    s_rim
        .add(id("other.are"), b"shadowed area".to_vec())
        .unwrap();
    fs::write(root.join("modules/other_s.rim"), s_rim.to_bytes().unwrap()).unwrap();
    root
}

/// A fixture's folder copied to `dir/tslpatchdata`, with the installer's
/// binary files written beside its changes file.
fn tslpatchdata(dir: &Path, fixture: &str) -> PathBuf {
    let to = dir.join("tslpatchdata");
    copy_dir(&fixtures().join(fixture), &to);
    let data = if to.join("opt").is_dir() {
        to.join("opt")
    } else {
        to.clone()
    };
    let mut append = Tlk::default();
    append.append(Entry::new("appended zero"));
    append.append(Entry::new("appended one"));
    fs::write(data.join("append.tlk"), append.to_bytes().unwrap()).unwrap();
    fs::write(data.join("k_test.ncs"), b"new bytecode").unwrap();
    fs::write(data.join("k_new.ncs"), b"a script for other").unwrap();
    fs::write(data.join("m_npc.utc"), b"not what the .mod holds").unwrap();
    fs::write(data.join("k_copy.ncs"), b"copied bytecode").unwrap();
    fs::write(data.join("k_hack.ncs"), vec![0u8; 16]).unwrap();
    fs::write(data.join("p_test.ssf"), soundset()).unwrap();
    fs::write(
        data.join("k_test.nss"),
        "#include \"k_inc_test\"\r\nvoid main() { PrintInteger(TestValue()); }\r\n",
    )
    .unwrap();
    fs::write(
        data.join("k_tok.nss"),
        "void main() { PrintInteger(#2DAMEMORY1#); }\r\n",
    )
    .unwrap();
    fs::write(data.join("k_kept.nss"), "void main() { }\r\n").unwrap();
    to
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dest = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &dest);
        } else {
            fs::copy(e.path(), dest).unwrap();
        }
    }
}

fn install_with(
    game: &Path,
    data: &Path,
    option: Option<u32>,
    hook: Option<&mut dyn FnMut(&Target)>,
) -> Report {
    let mut loaded = load(data, option).unwrap();
    let file = loaded.sources.len() - 1;
    let plan = Plan::build(&loaded.changes, file, &mut loaded.issues);
    assert!(
        loaded.issues.iter().all(|i| !i.is_error()),
        "{:?}",
        loaded.issues
    );
    let mut exec = Executor::open(game, data, &loaded).unwrap();
    if let Some(h) = hook {
        exec.after_write(h);
    }
    exec.run(&plan)
}

fn install(game: &Path, data: &Path) -> Report {
    install_with(game, data, None, None)
}

fn member(capsule: &Path, name: &str) -> Option<Vec<u8>> {
    let bytes = fs::read(capsule).unwrap();
    if bytes.starts_with(b"RIM ") {
        Rim::parse(&bytes)
            .unwrap()
            .get(id(name))
            .map(<[u8]>::to_vec)
    } else {
        Erf::parse(&bytes)
            .unwrap()
            .get(id(name))
            .map(<[u8]>::to_vec)
    }
}

fn hash(b: &[u8]) -> String {
    ContentHash::of(b).to_hex()
}

#[test]
fn replace_into_a_mod_that_holds_the_member_leaves_the_new_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "replace-mod");
    let report = install(&game, &data);
    assert!(report.ok(), "{report}");

    let m = game.join("modules/testmod.mod");
    assert_eq!(member(&m, "k_test.ncs").unwrap(), b"new bytecode");
    let r = &report.records[0];
    assert_eq!(r.target, "modules/testmod.mod:k_test.ncs");
    assert_eq!(r.outcome, Outcome::Written);
    assert_eq!(r.found.as_deref(), Some(hash(b"new bytecode").as_str()));
    assert_eq!(r.found, r.written);
    // The other member's bytes are untouched.
    assert_eq!(
        member(&m, "m_npc.utc").unwrap(),
        creature().to_bytes().unwrap()
    );

    // other.mod is made from the RIMs, the .rim's copy of a name first.
    let other = game.join("modules/other.mod");
    assert_eq!(report.created, ["modules/other.mod"]);
    assert_eq!(member(&other, "k_new.ncs").unwrap(), b"a script for other");
    assert_eq!(member(&other, "other.are").unwrap(), b"area");
    assert_eq!(member(&other, "k_other.ncs").unwrap(), b"other script");

    // File# does not overwrite a member the .mod holds.
    let skip = &report.records[2];
    assert_eq!(skip.outcome, Outcome::Skipped);
    assert_eq!(skip.issues[0].code, codes::SKIPPED_EXISTS);
    assert_eq!(
        member(&m, "m_npc.utc").unwrap(),
        creature().to_bytes().unwrap()
    );
}

#[test]
fn the_report_is_the_tree_a_copy_changed_before_read_back_fails_with_both_hashes() {
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "readback");
    let mut other_process = |t: &Target| {
        if let Target::File(p) = t
            && p.file_name().is_some_and(|n| n == "k_copy.ncs")
        {
            fs::write(p, b"another process wrote this").unwrap();
        }
    };
    let report = install_with(&game, &data, None, Some(&mut other_process));
    assert!(!report.ok(), "{report}");

    let copy = report
        .records
        .iter()
        .find(|r| r.operation.contains("k_copy.ncs"))
        .unwrap();
    assert_eq!(copy.outcome, Outcome::Failed);
    assert_eq!(copy.issues[0].code, codes::READBACK_MISMATCH);
    assert_eq!(
        copy.written.as_deref(),
        Some(hash(b"copied bytecode").as_str())
    );
    assert_eq!(
        copy.found.as_deref(),
        Some(hash(b"another process wrote this").as_str())
    );
    assert!(
        copy.issues[0]
            .message
            .contains(copy.written.as_deref().unwrap())
    );
    assert!(
        copy.issues[0]
            .message
            .contains(copy.found.as_deref().unwrap())
    );
    assert!(
        copy.issues[0].message.contains("[InstallList] line"),
        "{}",
        copy.issues[0].message
    );

    let edit = report
        .records
        .iter()
        .find(|r| r.operation.contains("p_test.utc"))
        .unwrap();
    assert_eq!(edit.outcome, Outcome::Written, "{report}");
    // The GFF came from the BIF and went to Override.
    assert_eq!(edit.source.as_deref(), Some("data/templates.bif[3]"));
    assert_eq!(edit.target, "Override/p_test.utc");
    let g = Gff::parse(&fs::read(game.join("Override/p_test.utc")).unwrap()).unwrap();
    assert_eq!(g.root.get("Tag"), Some(&Value::string("Edited")));
}

#[cfg(unix)]
#[test]
fn a_target_made_unwritable_after_planning_is_an_error_naming_the_operation() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "readback");
    let mut loaded = load(&data, None).unwrap();
    let plan = Plan::build(&loaded.changes, 0, &mut loaded.issues);

    let locked = game.join("Override/k_copy.ncs");
    fs::write(&locked, b"locked").unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o444)).unwrap();
    if fs::OpenOptions::new().write(true).open(&locked).is_ok() {
        eprintln!("skipped: this user can write a read-only file (running as root?)");
        return;
    }
    let report = Executor::open(&game, &data, &loaded).unwrap().run(&plan);
    assert!(!report.ok());
    let failed: Vec<_> = report.failed().collect();
    assert_eq!(failed.len(), 1, "{report}");
    assert_eq!(failed[0].issues[0].code, codes::WRITE_FAILED);
    assert!(
        failed[0].name().starts_with("[InstallList] line 10"),
        "{}",
        failed[0].name()
    );
    assert!(failed[0].issues[0].message.contains("Override/k_copy.ncs"));
    assert_eq!(fs::read(&locked).unwrap(), b"locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
}

#[test]
fn tokens_set_by_one_list_resolve_in_the_files_another_writes() {
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "tokens");
    let report = install(&game, &data);
    assert!(report.ok(), "{report}");

    // TLK: the two entries appended after the three vanilla lines.
    let tlk = Tlk::parse(&fs::read(game.join("dialog.tlk")).unwrap()).unwrap();
    assert_eq!(tlk.entries.len(), 5);
    assert_eq!(tlk.entries[3].text, "appended zero");
    assert_eq!(report.tokens.strref[&0], 3);
    assert_eq!(report.tokens.strref[&1], 4);

    // 2DA: the new row's name cell holds StrRef1, its race high() of the
    // column; row 1's race is blank; the new column holds StrRef0 in row 0.
    let t = TwoDa::parse(&fs::read(game.join("Override/appearance.2da")).unwrap()).unwrap();
    assert_eq!(t.rows.len(), 3);
    assert_eq!(t.rows[2].label, "2");
    assert_eq!(t.get(2, "label").unwrap(), "NewOne");
    assert_eq!(t.get(2, "name").unwrap(), "4");
    assert_eq!(t.get(2, "race").unwrap(), "6");
    assert_eq!(t.get(1, "race").unwrap(), "");
    assert_eq!(t.get(0, "tokened").unwrap(), "3");
    assert_eq!(t.get(1, "tokened").unwrap(), "x");
    assert_eq!(t.get(2, "tokened").unwrap(), "");
    assert_eq!(report.tokens.memory[&1], "2");
    assert_eq!(report.tokens.memory[&2], "NewOne");
    assert_eq!(report.tokens.memory[&3], "1");
    assert_eq!(report.tokens.memory[&6], "3");
    // One table, three operations, one file: every record has its hash.
    let table: Vec<_> = report
        .records
        .iter()
        .filter(|r| r.target == "Override/appearance.2da")
        .collect();
    assert_eq!(table.len(), 3);
    let on_disk = hash(&fs::read(game.join("Override/appearance.2da")).unwrap());
    assert!(
        table
            .iter()
            .all(|r| r.found.as_deref() == Some(on_disk.as_str()))
    );

    // GFF: the 2DA row index, the appended strref, a struct added to a list
    // with a child resref from a 2DA cell, and a later edit through the
    // child's stored path.
    let g = Gff::parse(&fs::read(game.join("Override/p_test.utc")).unwrap()).unwrap();
    assert_eq!(g.root.get("Appearance_Type"), Some(&Value::Word(2)));
    let Some(Value::LocString(name)) = g.root.get("FirstName") else {
        panic!("FirstName")
    };
    assert_eq!(name.strref, 3);
    assert_eq!(name.get(0), Some(&b"Senni"[..]));
    assert_eq!(g.root.get("Tag"), Some(&Value::string("1")));
    let item = g
        .struct_at(&GffPath::parse("ItemList[0]").unwrap())
        .unwrap();
    assert_eq!(item.id, 7);
    assert_eq!(
        item.get("InventoryRes"),
        Some(&Value::resref("NewOne").unwrap())
    );
    assert_eq!(item.get("Dropable"), Some(&Value::Byte(1)));
    assert_eq!(report.tokens.memory[&4], "0");
    assert_eq!(report.tokens.memory[&7], "ItemList\\0\\Dropable");

    // SSF: the strrefs in the named slots.
    let s = Ssf::parse(&fs::read(game.join("Override/p_test.ssf")).unwrap()).unwrap();
    assert_eq!(s.strrefs[0], 3);
    assert_eq!(s.strrefs[rakata_patch::ssf::slot("Death").unwrap()], 4);
}

#[test]
fn a_compile_list_compiles_through_nwscript_and_never_runs_the_shipped_compiler() {
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "compile");
    let marker = dir.path().join("nwnnsscomp-ran");
    let exe = data.join("nwnnsscomp.exe");
    fs::write(
        &exe,
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(game.join("Override/k_kept.ncs"), b"kept").unwrap();

    let report = install(&game, &data);
    assert!(report.ok(), "{report}");
    assert!(!marker.exists(), "the shipped nwnnsscomp.exe was run");

    let ncs = fs::read(game.join("Override/k_test.ncs")).unwrap();
    assert!(ncs.starts_with(b"NCS V1.0"), "{ncs:?}");
    assert_eq!(ncs, expected_compile(&game, &data, "k_test.nss"));

    // The token filled in before compiling: the new row is row 2.
    let tok = fs::read(game.join("Override/k_tok.ncs")).unwrap();
    assert_eq!(
        tok,
        library_compile(&game, &data, b"void main() { PrintInteger(2); }\r\n")
    );

    let kept = report
        .records
        .iter()
        .find(|r| r.operation == "compile k_kept.nss")
        .unwrap();
    assert_eq!(kept.outcome, Outcome::Skipped);
    assert_eq!(fs::read(game.join("Override/k_kept.ncs")).unwrap(), b"kept");
}

/// What `nws compile` writes for the script: the `nws` binary of this
/// build when it is there (`cargo test --workspace` builds it), else the
/// same library path `nws compile` takes.
fn expected_compile(game: &Path, data: &Path, script: &str) -> Vec<u8> {
    let exe = std::env::current_exe().unwrap();
    let nws = exe
        .parent()
        .and_then(Path::parent)
        .map(|d| d.join(format!("nws{}", std::env::consts::EXE_SUFFIX)));
    if let Some(nws) = nws.filter(|p| p.is_file()) {
        let out = data.parent().unwrap().join("nws-out.ncs");
        let status = std::process::Command::new(&nws)
            .args(["--game", "k1", "--game-dir"])
            .arg(game)
            .arg("compile")
            .arg(data.join(script))
            .arg("-o")
            .arg(&out)
            .status()
            .unwrap();
        assert!(status.success(), "nws compile failed");
        return fs::read(out).unwrap();
    }
    eprintln!("note: no nws binary beside the test; comparing with nwscript's library path");
    library_compile(game, data, &fs::read(data.join(script)).unwrap())
}

fn library_compile(game: &Path, data: &Path, source: &[u8]) -> Vec<u8> {
    use nwscript::Resolver;
    struct Both(nwscript::FsResolver, rakata_game::Install);
    impl Resolver for Both {
        fn include(&mut self, n: &str) -> std::io::Result<Option<nwscript::Resolved>> {
            match self.0.include(n)? {
                Some(r) => Ok(Some(r)),
                None => self.1.include(n),
            }
        }
        fn nwscript(&mut self) -> std::io::Result<Option<nwscript::Resolved>> {
            match self.0.nwscript()? {
                Some(r) => Ok(Some(r)),
                None => self.1.nwscript(),
            }
        }
        fn inputs(&self) -> &[nwscript::InputRecord] {
            &[]
        }
    }
    let install = rakata_game::Install::open(game, rakata_game::Profile::K1).unwrap();
    let mut r = Both(
        nwscript::FsResolver::new(vec![data.to_path_buf()], None),
        install,
    );
    let mut map = nwscript_diag::SourceMap::new();
    let out =
        nwscript::Checker::with_declarations(nwscript::Declarations::Shipped, nwscript::Game::K1)
            .check(&mut r, &mut map, "t.nss", source);
    assert!(
        out.diagnostics.iter().all(|d| !d.is_error()),
        "{:?}",
        out.diagnostics
    );
    let program =
        nwscript::compile::compile(&out.analysis.unwrap().program, nwscript::Game::K1).unwrap();
    nwscript::ncs::encode(&program)
}

#[test]
fn a_missing_required_file_installs_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "required");
    let report = install(&game, &data);
    assert!(!report.ok());
    let f = report.failure.as_ref().unwrap();
    assert_eq!(f.code, codes::REQUIRED_MISSING);
    assert!(
        f.message
            .contains("Install the mod this one builds on first.")
    );
    assert!(report.records.is_empty());
    assert!(!game.join("Override/k_copy.ncs").exists());
}

#[test]
fn a_lone_required_msg_and_an_empty_required_install() {
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "required-lone");
    let report = install(&game, &data);
    assert!(report.ok(), "{report}");
    assert_eq!(
        fs::read(game.join("Override/k_copy.ncs")).unwrap(),
        b"copied bytecode"
    );
}

#[test]
fn an_option_without_ini_name_installs_from_its_data_path() {
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "namespaces");
    let report = install_with(&game, &data, Some(1), None);
    assert!(report.ok(), "{report}");
    assert_eq!(
        report.records[0].source.as_deref(),
        Some("tslpatchdata/opt/k_copy.ncs")
    );
    assert_eq!(
        fs::read(game.join("Override/k_copy.ncs")).unwrap(),
        b"copied bytecode"
    );
}

#[test]
fn hack_list_writes_big_endian_32_bit_values_and_refuses_an_offset_past_the_end() {
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "hack");
    let report = install(&game, &data);
    let hack = report
        .records
        .iter()
        .find(|r| r.operation.contains("k_hack.ncs"))
        .unwrap();
    assert_eq!(hack.outcome, Outcome::Failed);
    assert_eq!(hack.issues.len(), 1);
    assert_eq!(hack.issues[0].code, codes::CANNOT_APPLY);
    assert!(hack.issues[0].message.contains("offset 64"));
    let b = fs::read(game.join("Override/k_hack.ncs")).unwrap();
    assert_eq!(&b[4..12], &[0, 0, 0x01, 0x2c, 0, 0, 0, 2][..]);
    assert_eq!(hack.found.as_deref(), Some(hash(&b).as_str()));
}

#[test]
fn a_report_serializes_with_every_operation_target_and_hash() {
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "replace-mod");
    let report = install(&game, &data);
    let doc = serde_json::to_value(&report).unwrap();
    let rec = &doc["records"][0];
    assert_eq!(rec["phase"], "install");
    assert_eq!(rec["outcome"], "written");
    assert_eq!(rec["target"], "modules/testmod.mod:k_test.ncs");
    assert_eq!(rec["found"].as_str().unwrap().len(), 64);
    let _ = Struct::new(0);
    let _ = ResType::Utc;
}

#[test]
fn a_member_name_longer_than_a_resref_goes_in_under_its_first_sixteen_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let game = game(dir.path());
    let data = tslpatchdata(dir.path(), "long-name");
    fs::write(data.join("k_test_long_named.ncs"), b"long-named bytecode").unwrap();
    let report = install(&game, &data);
    assert!(report.ok(), "{report}");

    let r = &report.records[0];
    assert_eq!(r.target, "modules/testmod.mod:k_test_long_name.ncs");
    assert_eq!(r.outcome, Outcome::Written);
    assert_eq!(r.issues[0].code, codes::NAME_CUT);
    let m = game.join("modules/testmod.mod");
    assert_eq!(
        member(&m, "k_test_long_name.ncs").unwrap(),
        b"long-named bytecode"
    );
    assert_eq!(member(&m, "k_test.ncs").unwrap(), b"old bytecode");
}
