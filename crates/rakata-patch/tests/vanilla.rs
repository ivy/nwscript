//! The executor against the user's own vanilla KotOR 1 install, read from
//! `$RAKATA_GAME_K1`; skipped, naming the variable, when it is unset. The
//! install is never written: the test's game folder holds copies of
//! `chitin.key` and `dialog.tlk`, links to the install's `data/` and to the
//! module's RIMs, and an empty `Override/`.

use rakata_2da::TwoDa;
use rakata_archive::Erf;
use rakata_gff::{Gff, Value};
use rakata_patch::exec::{Executor, Outcome};
use rakata_patch::{Plan, load};
use rakata_res::ResId;
use rakata_tlk::{Entry, Tlk};
use std::fs;
use std::path::{Path, PathBuf};

const GAME_VAR: &str = "RAKATA_GAME_K1";

fn vanilla() -> Option<PathBuf> {
    match std::env::var_os(GAME_VAR) {
        Some(v) if !v.is_empty() => Some(PathBuf::from(v)),
        _ => {
            eprintln!("skipped: ${GAME_VAR} is unset; set it to a vanilla KotOR 1 install");
            None
        }
    }
}

/// The entry of `dir` named `name`, ignoring case.
fn ci(dir: &Path, name: &str) -> PathBuf {
    fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name))
        })
        .unwrap_or_else(|| panic!("{} has no {name}", dir.display()))
}

#[cfg(unix)]
fn game(vanilla: &Path, dir: &Path) -> PathBuf {
    use std::os::unix::fs::symlink;
    let root = dir.join("game");
    fs::create_dir_all(root.join("Override")).unwrap();
    fs::create_dir_all(root.join("modules")).unwrap();
    fs::copy(ci(vanilla, "chitin.key"), root.join("chitin.key")).unwrap();
    fs::copy(ci(vanilla, "dialog.tlk"), root.join("dialog.tlk")).unwrap();
    symlink(ci(vanilla, "data"), root.join("data")).unwrap();
    let modules = ci(vanilla, "modules");
    for rim in ["danm13.rim", "danm13_s.rim"] {
        symlink(ci(&modules, rim), root.join("modules").join(rim)).unwrap();
    }
    root
}

#[cfg(unix)]
#[test]
fn a_hand_made_mod_installs_onto_the_games_own_files() {
    let Some(vanilla) = vanilla() else { return };
    let dir = tempfile::tempdir().unwrap();
    let root = game(&vanilla, dir.path());
    let data = dir.path().join("tslpatchdata");
    fs::create_dir_all(&data).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/exec/vanilla/changes.ini"),
        data.join("changes.ini"),
    )
    .unwrap();
    let mut append = Tlk::default();
    append.append(Entry::new("A line rakata's executor appended in a test."));
    fs::write(data.join("append.tlk"), append.to_bytes().unwrap()).unwrap();
    fs::write(
        data.join("k_rakata_tok.nss"),
        "#include \"k_inc_debug\"\r\nvoid main() { AurPostString(IntToString(#2DAMEMORY1#), 0, 0, 1.0); }\r\n",
    )
    .unwrap();
    let vanilla_tlk = Tlk::parse(&fs::read(root.join("dialog.tlk")).unwrap()).unwrap();

    let mut loaded = load(&data, None).unwrap();
    let plan = Plan::build(&loaded.changes, 0, &mut loaded.issues);
    let report = Executor::open(&root, &data, &loaded).unwrap().run(&plan);
    assert!(report.ok(), "{report}");
    assert!(report.records.iter().all(|r| r.outcome == Outcome::Written));

    // The line went to the end of the game's table.
    let strref = report.tokens.strref[&0];
    assert_eq!(strref as usize, vanilla_tlk.entries.len());

    // The table came from 2da.bif, the new row is its last.
    let table = &report.records[1];
    assert!(
        table
            .source
            .as_deref()
            .unwrap()
            .starts_with("data/2da.bif["),
        "{table:?}"
    );
    let t = TwoDa::parse(&fs::read(root.join("Override/appearance.2da")).unwrap()).unwrap();
    let row: usize = report.tokens.memory[&1].parse().unwrap();
    assert_eq!(row, t.rows.len() - 1);
    assert_eq!(t.get(row, "label").unwrap(), "Rakata_Test");

    // The creature went into a danm13.mod made from the RIMs, which holds
    // every member of both.
    assert_eq!(report.created, ["modules/danm13.mod"]);
    let m = fs::read(root.join("modules/danm13.mod")).unwrap();
    let m = Erf::parse(&m).unwrap();
    let utc = Gff::parse(m.get(ResId::parse("dan13_belaya.utc").unwrap()).unwrap()).unwrap();
    assert_eq!(
        utc.root.get("Appearance_Type"),
        Some(&Value::Word(row as u16))
    );
    let Some(Value::LocString(name)) = utc.root.get("FirstName") else {
        panic!("FirstName")
    };
    assert_eq!(name.strref, strref);
    let s_rim = fs::read(root.join("modules/danm13_s.rim")).unwrap();
    let s_rim = rakata_archive::Rim::parse(&s_rim).unwrap();
    assert!(s_rim.members().iter().all(|x| m.contains(x.id)));

    // The script compiled against the install's nwscript.nss and header.
    let ncs = fs::read(root.join("Override/k_rakata_tok.ncs")).unwrap();
    assert!(ncs.starts_with(b"NCS V1.0"));
}
