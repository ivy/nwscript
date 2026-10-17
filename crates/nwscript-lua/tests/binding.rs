//! The binding against hand-assembled scripts and a tiny engine table.

use nwscript_diag::FileId;
use nwscript_lua::{Host, run_test_file};
use nwscript_sema::EngineTable;
use std::collections::HashMap;
use std::rc::Rc;

const NWSCRIPT: &str = "int NPC_PEBBLE = 7;\r\nint GetGold(object o);\r\nobject GetFirstPC();\r\nvoid PrintString(string s);\r\nint GetGlobalNumber(string s);\r\n";

fn host(scripts: &[(&str, &str)]) -> Host {
    let (table, _) = EngineTable::from_nwscript(FileId(0), NWSCRIPT.as_bytes());
    let routines = table.functions.iter().map(|f| {
        let ty = |t: &nwscript_sema::Type| match t {
            nwscript_sema::Type::Int => nwscript_vm::Ty::Int,
            nwscript_sema::Type::String => nwscript_vm::Ty::String,
            nwscript_sema::Type::Object => nwscript_vm::Ty::Object,
            _ => nwscript_vm::Ty::Void,
        };
        nwscript_vm::Routine {
            name: f.name.clone(),
            params: f.params.iter().map(|p| ty(&p.ty)).collect(),
            ret: ty(&f.ret),
        }
    });
    let compiled: HashMap<String, Vec<u8>> = scripts
        .iter()
        .map(|(n, asm)| {
            (
                n.to_string(),
                nwscript_ncs::encode(&nwscript_ncs::asm::parse(FileId(0), asm.as_bytes()).unwrap()),
            )
        })
        .collect();
    Host {
        game: "k1".into(),
        routines: Rc::new(routines.collect()),
        table: Rc::new(table),
        find: Rc::new(move |n| compiled.get(n).map(|b| (n.to_string(), b.clone()))),
    }
}

/// int StartingConditional() { PrintString("x"); return GetGold(GetFirstPC()) < 500; }
const RICH: &str = "    RSADDI\n    JSR sc\n    RETN\nsc:\n    CONSTS \"x\"\n    ACTION 2, 1\n    ACTION 1, 0\n    ACTION 0, 1\n    CONSTI 500\n    LTII\n    CPDOWNSP -8, 4\n    MOVSP -4\n    RETN\n";

fn run_lua(test: &str) -> Vec<nwscript_lua::TestOutcome> {
    let dir = std::env::temp_dir().join(format!("nws-lua-{}-{}", std::process::id(), test.len()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("x_test.lua");
    std::fs::write(&f, test).unwrap();
    run_test_file(&host(&[("rich", RICH)]), &f, None).unwrap()
}

#[test]
fn overrides_answer_and_count_as_modelled() {
    let out = run_lua(
        r#"
        it("works", function()
          local w = nws.world()
          local pc = w:create("creature", "pc", { is_pc = true })
          w.party = { pc }
          w.engine.GetGold = function(o) assert(o == pc, "the PC is passed") return 100 end
          w.engine.PrintString = function() end
          local run = nws.script("rich"):conditional(w)
          assert.is_true(run.result)
          assert.modelled(run)
          assert.called(run, "GetGold", pc)
          assert.called(run, "PrintString", nws.any)
          assert.called_before(run, "PrintString", "GetGold")
          assert.equal(7, nws.const.NPC_PEBBLE)
        end)
    "#,
    );
    assert!(out.iter().all(|o| o.passed), "{out:?}");
}

#[test]
fn unmodelled_routines_are_reported() {
    let out = run_lua(
        r#"
        it("lists GetGold", function()
          local run = nws.script("rich"):conditional(nws.world())
          assert.equal("GetGold", run.unmodelled[1])
          assert.is_true(run.result)
          assert.not_called(run, "GetGlobalNumber")
        end)
    "#,
    );
    assert!(out.iter().all(|o| o.passed), "{out:?}");
}

#[test]
fn failures_carry_the_message_and_the_trace() {
    let out = run_lua(
        r#"
        describe("d", function()
          before_each(function() SEEN = (SEEN or 0) + 1 end)
          it("fails", function()
            nws.script("rich"):conditional(nws.world())
            assert.called(run_missing or nws.script("rich"):conditional(nws.world()), "GetGlobalNumber")
          end)
          it("counts before_each", function() assert.equal(2, SEEN) end)
        end)
    "#,
    );
    assert_eq!(out.len(), 2);
    assert!(!out[0].passed);
    assert_eq!(out[0].name, "d > fails");
    assert!(
        out[0]
            .message
            .as_deref()
            .unwrap()
            .contains("expected a call to GetGlobalNumber"),
        "{:?}",
        out[0].message
    );
    assert!(out[0].trace.contains("GetGold"), "{}", out[0].trace);
    assert!(out[1].passed, "{:?}", out[1].message);
}

#[test]
fn globals_and_objects_round_trip() {
    let out = run_lua(
        r#"
        it("holds state", function()
          local w = nws.world()
          w.globals.A = 3
          w.globals.B = true
          w.globals.C = "x"
          assert.equal(3, w.globals.A)
          assert.equal(true, w.globals.B)
          assert.equal("x", w.globals.C)
          local o = w:create("creature", "pebble", { hp = 4, position = { x = 1, y = 2, z = 3 } })
          o.max_hp = 9
          o:set_local(2, true)
          local item = o:give("card")
          assert.equal(9, o.max_hp)
          assert.equal(2, o.position.y)
          assert.equal(item, o.inventory[1])
          assert.equal(o, w:object("PEBBLE"))
          assert.is_true(o:valid())
        end)
    "#,
    );
    assert!(out.iter().all(|o| o.passed), "{out:?}");
}
