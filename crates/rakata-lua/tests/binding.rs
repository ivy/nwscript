//! The binding without an install: the sandbox, errors carrying codes, and
//! the formats and authoring operations over values a script builds.

use std::fs;
use std::path::Path;

use rakata::game::Profile;
use rakata_lua::{Failure, Options, Outcome, Print, run};

/// Write `script` as `main.lua` in a fresh directory and run it.
fn run_in(
    dir: &Path,
    script: &str,
    args: &[String],
    game: Option<&Path>,
) -> Result<Outcome, Failure> {
    let path = dir.join("main.lua");
    fs::write(&path, script).unwrap();
    run(&Options {
        script: path,
        args: args.to_vec(),
        game: game.map(Path::to_path_buf),
        profile: Profile::K1,
        print: Print::Capture,
    })
}

fn lua(script: &str) -> Result<Outcome, Failure> {
    let dir = tempfile::tempdir().unwrap();
    run_in(dir.path(), script, &[], None)
}

fn ok(script: &str) -> String {
    match lua(script) {
        Ok(o) => o.output,
        Err(f) => panic!("{}: {}\n{:?}", f.code, f.message, f.traceback),
    }
}

#[test]
fn the_state_has_no_io_os_package_or_file_loaders() {
    let out = ok(r#"
        print(io, os, package, debug, dofile, loadfile)
        print(type(string.format), type(table.concat), type(math.floor), type(utf8.char))
    "#);
    assert_eq!(
        out,
        "nil\tnil\tnil\tnil\tnil\tnil\nfunction\tfunction\tfunction\tfunction\n"
    );
}

#[test]
fn a_path_outside_the_inputs_is_refused_with_its_code() {
    let out = ok(r#"
        local ok, e = pcall(rakata.fs.read, "/etc/hostname")
        print(ok, e.code, rakata.Error.is(e))
        local ok2, e2 = pcall(rakata.fs.write, "/tmp/rakata-lua-escape", "x")
        print(ok2, e2.code)
        local ok3, e3 = pcall(rakata.fs.read, "../outside.txt")
        print(ok3, e3.code)
    "#);
    assert_eq!(out, "false\tRE1801\ttrue\nfalse\tRE1801\nfalse\tRE1801\n");
    assert!(!Path::new("/tmp/rakata-lua-escape").exists());
}

#[test]
fn an_argument_names_a_directory_the_script_may_write_and_read() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let arg = out_dir.path().display().to_string();
    let o = run_in(
        dir.path(),
        r#"
        local out = rakata.args[1]
        rakata.fs.write(rakata.fs.join(out, "sub", "a.bin"), "\0\1\2")
        local b = rakata.fs.read(rakata.fs.join(out, "sub", "a.bin"))
        print(#b, rakata.fs.exists(rakata.fs.join(out, "sub")), table.concat(rakata.fs.list(out), ","))
        local ok, e = pcall(rakata.fs.write, rakata.fs.join(out, "..", "beside"), "x")
        print(ok, e.code)
    "#,
        &[arg],
        None,
    )
    .unwrap();
    assert_eq!(o.output, "3\ttrue\tsub\nfalse\tRE1801\n");
    assert_eq!(
        fs::read(out_dir.path().join("sub/a.bin")).unwrap(),
        b"\0\x01\x02"
    );
}

#[test]
fn a_symbolic_link_does_not_lead_out_of_an_input() {
    let dir = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("/etc", dir.path().join("etc")).unwrap();
    let o = run_in(
        dir.path(),
        r#"
        local ok, e = pcall(rakata.fs.read, rakata.fs.join(rakata.script, "..", "etc", "hostname"))
        print(ok, e.code)
    "#,
        &[],
        None,
    )
    .unwrap();
    assert_eq!(o.output, "false\tRE1801\n");
}

#[test]
fn require_loads_modules_from_the_scripts_directory_only() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("lib")).unwrap();
    fs::write(
        dir.path().join("lib/util.lua"),
        "local M = {} M.loads = (M.loads or 0) + 1 function M.twice(x) return 2 * x end return M",
    )
    .unwrap();
    let o = run_in(
        dir.path(),
        r#"
        local u = require "lib.util"
        local again = require "lib.util"
        print(u.twice(21), u == again, require "rakata" == rakata)
        local ok, e = pcall(require, "..secret")
        print(ok, e.code)
        local ok2, e2 = pcall(require, "missing")
        print(ok2, e2.code)
    "#,
        &[],
        None,
    )
    .unwrap();
    assert_eq!(o.output, "42\ttrue\ttrue\nfalse\tRE1802\nfalse\tRE1802\n");
}

#[test]
fn an_authoring_error_is_a_lua_error_carrying_the_diagnostic_code() {
    let out = ok(r#"
        local cat = rakata.twoda.new({"name", "type"})
        print(rakata.author.globals.declare(cat, "OAJ_DONE", "Boolean"))
        local ok, e = pcall(rakata.author.globals.declare, cat, "oaj_done", "Number")
        print(ok, e.code, tostring(e):sub(1, 7), #cat)
        print(e.message)
    "#);
    assert_eq!(
        out,
        "0\nfalse\tRE1404\tRE1404:\t1\nglobalcat.2da already declares oaj_done, at row 0\n"
    );

    // Uncaught, and raised again with error(e), the code reaches the run's
    // failure.
    for script in [
        r#"local cat = rakata.twoda.new({"name", "type"})
           rakata.author.globals.declare(cat, "A", "Boolean")
           rakata.author.globals.declare(cat, "A", "Boolean")"#,
        r#"local cat = rakata.twoda.new({"name", "type"})
           rakata.author.globals.declare(cat, "A", "Boolean")
           local ok, e = pcall(rakata.author.globals.declare, cat, "A", "Boolean")
           error(e)"#,
    ] {
        let f = lua(script).unwrap_err();
        assert_eq!(f.code, "RE1404", "{}", f.message);
        assert!(f.message.contains("already declares A"), "{}", f.message);
    }

    // A script's own error has the binding's code.
    let f = lua("local t = nil; return t.x").unwrap_err();
    assert_eq!(f.code, "RE1804");
    let f = lua("error('mine')").unwrap_err();
    assert_eq!(f.code, "RE1804");
    assert!(f.message.contains("mine"), "{}", f.message);
}

#[test]
fn a_blueprint_field_keeps_its_type() {
    let out = ok(r#"
        local g = rakata.gff.new("UTC")
        local gff = rakata.gff
        local ok, e = pcall(rakata.author.blueprint.set, g, "Str", 9)
        print(ok, e.code)
        rakata.author.blueprint.set(g, "Str", gff.byte(14))
        print(rakata.author.blueprint.get(g, "Str"), g:type("Str"))
        print(rakata.author.blueprint.set(g, "Str", 7))
        local ok2, e2 = pcall(rakata.author.blueprint.set, g, "Str", gff.word(5))
        print(ok2, e2.code)
        local ok3, e3 = pcall(rakata.author.blueprint.set, g, "Str", 300)
        print(ok3, e3.code)
        rakata.author.blueprint.set(g, "TemplateResRef", gff.resref("dan14_juhani"))
        local c = rakata.author.blueprint.clone(g, "oaj_juhani1")
        print(c:get("TemplateResRef"), g:get("TemplateResRef"))
        local back = rakata.gff.parse(c:to_bytes())
        print(back:file_type(), back:get("Str"), table.concat(back:fields(), ","))
    "#);
    assert_eq!(
        out,
        "false\tRE1805\n14\tBYTE\n14\nfalse\tRE1406\nfalse\tRE1805\noaj_juhani1\tdan14_juhani\nUTC\t7\tStr,TemplateResRef\n"
    );
}

#[test]
fn a_table_row_copied_with_a_changed_label_is_written_as_the_rust_api_writes_it() {
    let mut t = rakata::twoda::TwoDa::new(["label", "race", "modeltype"]);
    t.add_row(
        "0",
        &[
            ("label", "Creature_Brith"),
            ("race", "C_Brith"),
            ("modeltype", "S"),
        ],
    )
    .unwrap();
    t.add_row(
        "1",
        &[("label", "Other"), ("race", "X"), ("modeltype", "F")],
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.2da");
    fs::write(&input, t.to_bytes().unwrap()).unwrap();
    let out = dir.path().join("out");
    let o = run_in(
        dir.path(),
        r#"
        local t = rakata.twoda.parse(rakata.fs.read(rakata.args[1]))
        local i = rakata.author.twoda.add_row_copying(t, "Creature_Brith", "Hidden", {race = "PLC_Invis"})
        print(i, t:label(i), t:get(i, "label"), t:get(i, "modeltype"))
        local ok, e = pcall(rakata.author.twoda.add_row_copying, t, "Creature_Brith", "Hidden")
        print(ok, e.code)
        rakata.fs.write(rakata.fs.join(rakata.args[2], "appearance.2da"), t:to_bytes())
    "#,
        &[input.display().to_string(), out.display().to_string()],
        None,
    )
    .unwrap();
    assert_eq!(o.output, "2\t2\tHidden\tS\nfalse\tRE1405\n");
    let from = rakata::author::twoda::RowKey::Label("Creature_Brith".into());
    rakata::author::twoda::add_row_copying(
        &mut t,
        &from,
        "Hidden",
        &[("race".into(), "PLC_Invis".into())],
    )
    .unwrap();
    assert_eq!(
        fs::read(out.join("appearance.2da")).unwrap(),
        t.to_bytes().unwrap()
    );
}

#[test]
fn the_install_needs_game() {
    let f = lua("rakata.Install.open()").unwrap_err();
    assert_eq!(f.code, "RE1803");
    let f = lua("rakata.dialog.Session.open()").unwrap_err();
    assert_eq!(f.code, "RE1803");
}

#[test]
fn an_option_the_function_does_not_take_is_refused() {
    let f = lua(r#"rakata.Install.open{profle = "k2"}"#).unwrap_err();
    assert_eq!(f.code, "RE1805");
    assert!(f.message.contains("profle"), "{}", f.message);
}

#[test]
fn lip_sync_from_rhubarbs_cues() {
    let out = ok(r#"
        local A = rakata.author.lip.rhubarb_shape("A")
        local lip = rakata.author.lip.from_cues({{0.0, A}, {0.25, "ah"}, {time = 0.5, shape = 4}}, 1.0)
        local back = rakata.lip.parse(lip:to_bytes())
        local shapes = {}
        for _, k in ipairs(back:keys()) do shapes[#shapes + 1] = k.shape end
        print(back.length, table.concat(shapes, " "), rakata.author.lip.rhubarb_shape("Q"))
    "#);
    assert_eq!(out, "1.0\tm ah oh rest\tnil\n");
}

#[test]
fn print_is_captured_and_explain_answers() {
    let out = ok(r#"
        local c = rakata.explain("re1404")
        print(c.code, c.summary)
        print(rakata.explain("XX9999"))
        print(rakata.res.hash(""), rakata.res.id("APPEARANCE.2DA"))
    "#);
    assert_eq!(
        out,
        "RE1404\ta global globalcat.2da already declares\nnil\ne3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\tAPPEARANCE.2da\n"
    );
}

#[test]
fn every_binding_code_is_registered() {
    for c in [
        rakata_lua::codes::NOT_AN_INPUT,
        rakata_lua::codes::NO_SUCH_MODULE,
        rakata_lua::codes::NO_GAME,
        rakata_lua::codes::SCRIPT_FAILED,
        rakata_lua::codes::BAD_ARGUMENT,
        rakata_lua::codes::WRITE_FAILED,
        rakata_lua::codes::READ_FAILED,
    ] {
        assert!(rakata::codes::lookup(c).is_some(), "{c}");
    }
}
