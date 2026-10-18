//! A dialogue whose loaded copy is in `Override/` and whose name a
//! module's capsules also hold: shown with that module, so its scripts
//! resolve, and refused with RE1105 when several modules hold it. Over a
//! hand-made install, and over `$RAKATA_BUILD` (skipped, naming the
//! variable, when it is unset). No game bytes are committed.

use std::fs;
use std::path::PathBuf;

use rakata::dialog::Session;
use rakata::dialog::lint::Linter;
use rakata::game::Profile;
use rakata_archive::{Key, Rim};
use rakata_gff::Gff;
use rakata_res::ResId;

/// An install with `x.dlg` in `Override/` and in each of `modules`' `.rim`.
fn make(name: &str, modules: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "rakata-dialog-override-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Override")).unwrap();
    fs::create_dir_all(root.join("modules")).unwrap();
    fs::write(root.join("chitin.key"), Key::new().to_bytes().unwrap()).unwrap();
    let dlg = Gff::new(*b"DLG ").to_bytes().unwrap();
    fs::write(root.join("Override/x.dlg"), &dlg).unwrap();
    for m in modules {
        let mut rim = Rim::new();
        rim.add(ResId::parse("x.dlg").unwrap(), dlg.clone())
            .unwrap();
        fs::write(
            root.join("modules").join(format!("{m}.rim")),
            rim.to_bytes().unwrap(),
        )
        .unwrap();
    }
    root
}

#[test]
fn an_override_dialogue_one_module_holds_is_shown_with_that_module() {
    let root = make("one", &["danm14ac"]);
    let mut session = Session::open(&root, Profile::K1).unwrap();
    let loaded = session.load("x.dlg").unwrap();
    assert_eq!(loaded.location, "Override/x.dlg");
    assert_eq!(loaded.module.as_deref(), Some("danm14ac"));
    assert_eq!(
        session.module().map(ToString::to_string).as_deref(),
        Some("danm14ac")
    );
}

#[test]
fn an_override_dialogue_no_module_holds_stays_global() {
    let root = make("none", &[]);
    let mut session = Session::open(&root, Profile::K1).unwrap();
    let loaded = session.load("x.dlg").unwrap();
    assert_eq!(loaded.location, "Override/x.dlg");
    assert_eq!(loaded.module, None);
}

#[test]
fn an_override_dialogue_several_modules_hold_asks_for_a_module() {
    let root = make("several", &["danm13", "danm14aa"]);
    let mut session = Session::open(&root, Profile::K1).unwrap();
    let e = session.load("x.dlg").unwrap_err();
    assert_eq!(e.code(), "RE1105");
    assert_eq!(
        e.message(),
        "x.dlg is in Override/ and in 2 modules: danm13, danm14aa; name one with --module"
    );
    // Named, the module is the one searched.
    session.set_module(Some(rakata::game::ModuleName::new("danm13")));
    assert_eq!(
        session.load("x.dlg").unwrap().module.as_deref(),
        Some("danm13")
    );
    // The lint of a named dialogue still lints it as a global one.
    let mut linter = Linter::open(&root, Profile::K1).unwrap();
    let linted = linter.lint_named("x.dlg", None).unwrap();
    assert_eq!(linted.loaded.module, None);
}

#[test]
fn juhanis_override_dialogue_in_the_build_resolves_its_module_scripts() {
    let Ok(root) = std::env::var("RAKATA_BUILD") else {
        eprintln!("skipped: $RAKATA_BUILD is unset; set it to a modded KotOR 1 build");
        return;
    };
    let mut session = Session::open(&PathBuf::from(root), Profile::K1).unwrap();
    let loaded = session.load("dan14_juhani.dlg").unwrap();
    assert_eq!(loaded.location, "Override/dan14_juhani.dlg");
    assert_eq!(loaded.module.as_deref(), Some("danm14ac"));
    let t = session.transcript(
        &loaded.dialog,
        rakata::dialog::TranscriptOptions { scripts: false },
    );
    assert!(!t.text().contains("[missing]"), "{}", t.text());
}
