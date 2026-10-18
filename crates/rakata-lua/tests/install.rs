//! The binding over the user's own KotOR 1 install: the same operations in
//! Lua and through the Rust API give the same bytes.
//!
//! Reads `$RAKATA_GAME_K1` (a vanilla KotOR 1 install) and skips, naming the
//! variable, when it is unset. Writes go to temporary directories and to a
//! mirror of the install whose `Override/` is the test's own; nothing is
//! written to the install, and no game bytes are committed.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use rakata::author::dialog::{self as dlg, Text};
use rakata::author::twoda::{self, RowKey};
use rakata::author::{Mod, codes};
use rakata::game::{Install, Profile};
use rakata::twoda::TwoDa;
use rakata_dialog::{Dialog, Kind, NewNode, NodeKey};
use rakata_lua::{Failure, Options, Outcome, Print, run};

fn game() -> Option<PathBuf> {
    match std::env::var("RAKATA_GAME_K1") {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: $RAKATA_GAME_K1 is unset; set it to a vanilla KotOR 1 install");
            None
        }
    }
}

fn run_in(dir: &Path, script: &str, args: &[String], game: &Path) -> Result<Outcome, Failure> {
    let path = dir.join("main.lua");
    fs::write(&path, script).unwrap();
    run(&Options {
        script: path,
        args: args.to_vec(),
        game: Some(game.to_path_buf()),
        profile: Profile::K1,
        print: Print::Capture,
    })
}

fn id(text: &str) -> rakata::res::ResId {
    rakata::res::ResId::parse(text).unwrap()
}

/// A mirror of `game`: every entry linked, except `Override/` and
/// `modules/`, whose files are linked one by one into folders of the
/// mirror's own, so a write replaces a link or adds a file and never
/// touches the install.
fn mirror(game: &Path, root: &Path) {
    for e in fs::read_dir(game).unwrap() {
        let e = e.unwrap();
        let n = e.file_name();
        let own = ["override", "modules"]
            .iter()
            .any(|d| n.to_string_lossy().eq_ignore_ascii_case(d));
        if own {
            let dir = root.join(&n);
            fs::create_dir(&dir).unwrap();
            for f in fs::read_dir(e.path()).unwrap() {
                let f = f.unwrap();
                symlink(f.path(), dir.join(f.file_name())).unwrap();
            }
        } else {
            symlink(e.path(), root.join(&n)).unwrap();
        }
    }
}

#[test]
fn a_globalcat_row_copied_with_a_changed_label_is_the_bytes_the_rust_api_writes() {
    let Some(game) = game() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let o = run_in(
        dir.path(),
        r#"
        local install = rakata.Install.open()
        local m = rakata.Mod.open(install)
        local loaded = m:loaded("globalcat.2da")
        local cat = rakata.twoda.parse(loaded.bytes)
        local row = rakata.author.twoda.add_row_copying(cat, 0, "OAJ_TEST", {name = "OAJ_TEST"})
        rakata.fs.write(rakata.fs.join(rakata.args[1], "globalcat.2da"), cat:to_bytes())
        print(row, cat:label(row), cat:get(row, "name"), loaded.written)
    "#,
        &[out.path().display().to_string()],
        &game,
    )
    .unwrap_or_else(|f| panic!("{}: {}", f.code, f.message));

    let install = Install::open(&game, Profile::K1).unwrap();
    let m = Mod::open(install, None).unwrap();
    let mut cat = TwoDa::parse(&m.loaded(id("globalcat.2da")).unwrap().bytes).unwrap();
    let row = twoda::add_row_copying(
        &mut cat,
        &RowKey::Index(0),
        "OAJ_TEST",
        &[("name".into(), "OAJ_TEST".into())],
    )
    .unwrap();
    assert_eq!(o.output, format!("{row}\tOAJ_TEST\tOAJ_TEST\tfalse\n"));
    assert_eq!(
        fs::read(out.path().join("globalcat.2da")).unwrap(),
        cat.to_bytes().unwrap()
    );
}

#[test]
fn the_dialogue_operations_write_what_the_rust_api_writes_and_refuse_what_it_refuses() {
    let Some(game) = game() else {
        return;
    };
    let lua_tree = tempfile::tempdir().unwrap();
    let rust_tree = tempfile::tempdir().unwrap();
    mirror(&game, lua_tree.path());
    mirror(&game, rust_tree.path());
    let dir = tempfile::tempdir().unwrap();
    let o = run_in(
        dir.path(),
        r#"
        local d = rakata.author.dialog
        local m = rakata.Mod.open(rakata.Install.open(), "danm14ac")
        local banter = rakata.dialog.parse("banter.dlg", m:loaded("banter.dlg").bytes)
        local ours = d.empty_like(banter, "oaj_banter.dlg")
        local e0 = d.add_entry(ours, {text = "Hey, Carth?", speaker = "Mission", script = "k_oaj_beat"})
        d.start(ours, e0, {condition = "k_oaj_c_done"})
        local ok, e = pcall(m.write_override, m, "oaj_banter.dlg", ours:to_bytes())
        print(ok, e.code, tostring(e0))

        local name = "k_hcar_dialog.dlg"
        local carth = rakata.dialog.parse(name, m:loaded(name).bytes)
        local before = #m:check_dialog(carth)
        local r = d.add_reply(carth, {text = "About what happened on Dantooine..."})
        d.link(carth, "E0", r)
        local w = m:write_override(name, carth:to_bytes())
        print(#w.warnings == before, m:loaded(name).written, r.kind, r.index)
        local key = rakata.dialog.NodeKey.of(carth, r)
        print(tostring(key), key:resolve(carth) == r)
    "#,
        &[],
        lua_tree.path(),
    )
    .unwrap_or_else(|f| panic!("{}: {}", f.code, f.message));

    let install = Install::open(rust_tree.path(), Profile::K1).unwrap();
    let mut m = Mod::open(install, Some("danm14ac")).unwrap();
    let banter = Dialog::parse("banter.dlg", &m.loaded(id("banter.dlg")).unwrap().bytes).unwrap();
    let mut ours = dlg::empty_like(&banter, "oaj_banter.dlg");
    let e0 = dlg::add_entry(
        &mut ours,
        &NewNode {
            text: Text::English("Hey, Carth?".into()),
            speaker: "Mission".into(),
            script: Some(rakata::res::ResRef::new("k_oaj_beat").unwrap()),
            ..NewNode::default()
        },
    )
    .unwrap();
    dlg::start(&mut ours, &e0.into(), Some("k_oaj_c_done"), None).unwrap();
    let e = m
        .write_override("oaj_banter.dlg", &ours.to_bytes().unwrap())
        .unwrap_err();
    assert_eq!(e.code(), codes::DIALOG_REFUSED);

    let name = "k_hcar_dialog.dlg";
    let mut carth = Dialog::parse(name, &m.loaded(id(name)).unwrap().bytes).unwrap();
    let r = dlg::add_reply(
        &mut carth,
        &NewNode {
            text: Text::English("About what happened on Dantooine...".into()),
            ..NewNode::default()
        },
    )
    .unwrap();
    dlg::link(
        &mut carth,
        &NodeKey::Index(Kind::Entry, 0),
        &r.into(),
        None,
        None,
    )
    .unwrap();
    m.write_override(name, &carth.to_bytes().unwrap()).unwrap();

    assert_eq!(
        o.output,
        format!(
            "false\tRE1407\tE{}\ntrue\ttrue\treply\t{}\ntext:About what happened on Dantooine...\ttrue\n",
            e0.index, r.index
        )
    );
    assert!(!lua_tree.path().join("Override/oaj_banter.dlg").exists());
    assert_eq!(
        fs::read(lua_tree.path().join("Override").join(name)).unwrap(),
        fs::read(rust_tree.path().join("Override").join(name)).unwrap()
    );
}

#[test]
fn a_session_loads_a_dialogue_by_name_and_shows_its_transcript() {
    let Some(game) = game() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let o = run_in(
        dir.path(),
        r#"
        local s = rakata.dialog.Session.open()
        local l = s:load("k_hcar_dialog.dlg")
        local t = s:transcript(l.dialog)
        print(l.dialog:name(), t.dialog, t.entries == l.dialog:len("entry"), #t.text > 0)
        local ok, e = pcall(s.load, s, "/etc/x.dlg")
        print(ok, e.code)
        print(rakata.Install.open():read("no_such_thing.dlg"))
    "#,
        &[],
        &game,
    )
    .unwrap_or_else(|f| panic!("{}: {}", f.code, f.message));
    assert_eq!(
        o.output,
        "k_hcar_dialog.dlg\tk_hcar_dialog.dlg\ttrue\ttrue\nfalse\tRE1801\nnil\n"
    );
}

#[test]
fn a_script_name_over_sixteen_bytes_is_refused_before_anything_compiles() {
    let Some(game) = game() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let o = run_in(
        dir.path(),
        r#"
        local m = rakata.Mod.open(rakata.Install.open())
        local ok, e = pcall(rakata.author.scripts.compile, m, {k_oaj_seventeen01 = "void main() { }"})
        print(ok, e.code, e.message)
        ok, e = pcall(rakata.author.scripts.compile, m, {{name = "k_oaj_seventeen02", text = "not nwscript"}})
        print(ok, e.code)
    "#,
        &[],
        &game,
    )
    .unwrap_or_else(|f| panic!("{}: {}", f.code, f.message));
    assert_eq!(
        o.output,
        "false\tRE0101\tresource name `k_oaj_seventeen01` is 17 bytes; a resref holds at most 16\nfalse\tRE0101\n"
    );
}

#[test]
fn a_dialogue_written_into_one_modules_mod_is_the_bytes_the_rust_api_writes() {
    let Some(game) = game() else {
        return;
    };
    let lua_tree = tempfile::tempdir().unwrap();
    let rust_tree = tempfile::tempdir().unwrap();
    mirror(&game, lua_tree.path());
    mirror(&game, rust_tree.path());
    let dir = tempfile::tempdir().unwrap();
    let o = run_in(
        dir.path(),
        r#"
        local d = rakata.author.dialog
        local m = rakata.Mod.open(rakata.Install.open(), "tar_m02af")
        local name = "tar02_carbast.dlg"
        local ok, e = pcall(m.guard_other_modules, m, name)
        print(ok, e.code)
        local carbast = rakata.dialog.parse(name, m:loaded(name).bytes)
        local r = d.add_reply(carbast, {text = "A reply of the test's own."})
        d.link(carbast, "voice:nm02aacarb01002_", r, {at = 0})
        local w = m:write_module(name, carbast:to_bytes())
        print(m:loaded(name).location, w.len > 0)
        m:set_module(nil)
        ok, e = pcall(m.write_module, m, name, carbast:to_bytes())
        print(ok, e.code)
    "#,
        &[],
        lua_tree.path(),
    )
    .unwrap_or_else(|f| panic!("{}: {}", f.code, f.message));
    assert_eq!(
        o.output,
        "false\tRE1415\nmodules/tar_m02af.mod:tar02_carbast.dlg\ttrue\nfalse\tRE1419\n"
    );

    let install = Install::open(rust_tree.path(), Profile::K1).unwrap();
    let mut m = Mod::open(install, Some("tar_m02af")).unwrap();
    let name = "tar02_carbast.dlg";
    let mut carbast = Dialog::parse(name, &m.loaded(id(name)).unwrap().bytes).unwrap();
    let r = dlg::add_reply(
        &mut carbast,
        &NewNode {
            text: Text::English("A reply of the test's own.".into()),
            ..NewNode::default()
        },
    )
    .unwrap();
    let anchor = NodeKey::Voice(rakata::res::ResRef::new("nm02aacarb01002_").unwrap());
    dlg::link(&mut carbast, &anchor, &r.into(), None, Some(0)).unwrap();
    m.write_module(name, &carbast.to_bytes().unwrap()).unwrap();
    assert_eq!(
        fs::read(lua_tree.path().join("modules/tar_m02af.mod")).unwrap(),
        fs::read(rust_tree.path().join("modules/tar_m02af.mod")).unwrap()
    );
    assert!(!game.join("modules/tar_m02af.mod").exists());
}
