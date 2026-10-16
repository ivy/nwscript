//! The world model: its routines, its generated documentation, and its
//! coverage of the corpus's engine calls.

use nwscript_diag::FileId;
use nwscript_vm::world::{Kind, MODELS, Object, model_doc};
use nwscript_vm::{Engine, Limits, Routine, Ty, Value, World, run};

fn sig(name: &str, params: &[Ty], ret: Ty) -> Routine {
    Routine {
        name: name.into(),
        params: params.to_vec(),
        ret,
    }
}

fn call(w: &mut World, name: &str, ret: Ty, args: &[Value]) -> Value {
    let r = w.call(0, &sig(name, &[], ret), args);
    assert!(r.modelled, "{name} is not modelled");
    r.value
}

#[test]
fn docs_are_generated_from_the_models() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/engine-model.md");
    let doc = model_doc();
    if std::env::var_os("UPDATE_DOCS").is_some() {
        std::fs::write(&path, &doc).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap_or_default(),
        doc,
        "docs/engine-model.md is stale: rerun with UPDATE_DOCS=1"
    );
}

#[test]
fn every_listed_routine_answers() {
    // With no arguments every modelled routine still answers as modelled,
    // except the ones whose criteria it declines (none with defaults).
    let mut w = World::new(1);
    for (name, _, _) in MODELS {
        let r = w.call(0, &sig(name, &[], Ty::Int), &[]);
        assert!(r.modelled || *name == "GetNearestCreature", "{name}");
    }
    assert!(MODELS.iter().filter(|m| m.1 == Kind::Pure).count() >= 10);
}

#[test]
fn strings_and_numbers() {
    let mut w = World::new(0);
    let s = |x: &str| Value::string(x);
    assert_eq!(
        call(&mut w, "StringToInt", Ty::Int, &[s("  -42abc")]),
        Value::Int(-42)
    );
    assert_eq!(
        call(&mut w, "StringToInt", Ty::Int, &[s("x")]),
        Value::Int(0)
    );
    assert_eq!(
        call(&mut w, "IntToString", Ty::String, &[Value::Int(-7)]),
        s("-7")
    );
    assert_eq!(
        call(&mut w, "FloatToString", Ty::String, &[Value::Float(1.5)]),
        s("       1.500000000")
    );
    assert_eq!(
        call(
            &mut w,
            "FloatToString",
            Ty::String,
            &[Value::Float(1.5), Value::Int(0), Value::Int(1)]
        ),
        s("1.5")
    );
    assert_eq!(
        call(
            &mut w,
            "GetSubString",
            Ty::String,
            &[s("abcdef"), Value::Int(2), Value::Int(10)]
        ),
        s("cdef")
    );
    assert_eq!(
        call(&mut w, "FindSubString", Ty::Int, &[s("abcdef"), s("cd")]),
        Value::Int(2)
    );
    assert_eq!(
        call(&mut w, "FloatToInt", Ty::Int, &[Value::Float(-2.7)]),
        Value::Int(-2)
    );
}

#[test]
fn the_generator_is_seeded() {
    let rolls = |seed| {
        let mut w = World::new(seed);
        (0..20)
            .map(|_| call(&mut w, "d6", Ty::Int, &[Value::Int(2)]))
            .collect::<Vec<_>>()
    };
    assert_eq!(rolls(7), rolls(7));
    assert_ne!(rolls(7), rolls(8));
    assert!(rolls(7).iter().all(|v| matches!(v, Value::Int(2..=12))));
}

#[test]
fn objects_party_and_variables() {
    let mut w = World::new(0);
    let pc = w.create(Object {
        kind: 1,
        tag: "pc".into(),
        is_pc: true,
        area: 9,
        ..Object::default()
    });
    let pebble = w.create(Object {
        kind: 1,
        tag: "Pebble".into(),
        area: 9,
        position: [3.0, 4.0, 0.0],
        hp: 5,
        max_hp: 10,
        ..Object::default()
    });
    w.party = vec![pc, pebble];
    assert_eq!(
        call(
            &mut w,
            "GetObjectByTag",
            Ty::Object,
            &[Value::string("pebble")]
        ),
        Value::Object(pebble)
    );
    assert_eq!(
        call(
            &mut w,
            "GetPartyMemberByIndex",
            Ty::Object,
            &[Value::Int(1)]
        ),
        Value::Object(pebble)
    );
    assert_eq!(
        call(
            &mut w,
            "GetDistanceBetween",
            Ty::Float,
            &[Value::Object(pc), Value::Object(pebble)]
        ),
        Value::Float(5.0)
    );
    call(
        &mut w,
        "SetGlobalNumber",
        Ty::Void,
        &[Value::string("K_X"), Value::Int(3)],
    );
    assert_eq!(
        call(&mut w, "GetGlobalNumber", Ty::Int, &[Value::string("K_X")]),
        Value::Int(3)
    );
    call(
        &mut w,
        "SetLocalBoolean",
        Ty::Void,
        &[Value::Object(pebble), Value::Int(5), Value::Int(1)],
    );
    assert_eq!(
        call(
            &mut w,
            "GetLocalBoolean",
            Ty::Int,
            &[Value::Object(pebble), Value::Int(5)]
        ),
        Value::Int(1)
    );
    let card = call(
        &mut w,
        "CreateItemOnObject",
        Ty::Object,
        &[Value::string("tar_vaultcard"), Value::Object(pc)],
    );
    assert_eq!(
        call(
            &mut w,
            "GetItemPossessedBy",
            Ty::Object,
            &[Value::Object(pc), Value::string("tar_vaultcard")]
        ),
        card
    );
    call(&mut w, "DestroyObject", Ty::Void, std::slice::from_ref(&card));
    assert_eq!(
        call(&mut w, "GetIsObjectValid", Ty::Int, &[card]),
        Value::Int(0)
    );
}

#[test]
fn assigned_actions_run_as_their_owner() {
    // AssignCommand(GetObjectByTag("npc"), PrintString(GetTag(OBJECT_SELF)))
    let text = "    JSR main\n    RETN\nmain:\n    STORE_STATE 0, 0\n    JMP after\n    CONSTO 0\n    ACTION 1, 1\n    ACTION 2, 1\n    RETN\nafter:\n    CONSTI 0\n    CONSTS \"npc\"\n    ACTION 3, 2\n    ACTION 0, 2\n    RETN\n";
    let p = nwscript_ncs::asm::parse(FileId(0), text.as_bytes()).unwrap();
    let table = vec![
        sig("AssignCommand", &[Ty::Object, Ty::Action], Ty::Void),
        sig("GetTag", &[Ty::Object], Ty::String),
        sig("PrintString", &[Ty::String], Ty::Void),
        sig("GetObjectByTag", &[Ty::String, Ty::Int], Ty::Object),
    ];
    let mut w = World::new(0);
    let me = w.create(Object {
        kind: 1,
        tag: "me".into(),
        ..Object::default()
    });
    w.create(Object {
        kind: 1,
        tag: "npc".into(),
        ..Object::default()
    });
    w.context.caller = me;
    let r = run(&p, &table, &mut w, Limits::default());
    assert_eq!(r.fault, None, "{:?}", r.history);
    let last = r.trace.last().unwrap();
    assert_eq!(last.name, "PrintString");
    assert_eq!(last.args, vec![Value::string("npc")]);
    assert!(r.unmodelled.is_empty(), "{:?}", r.unmodelled);
}
