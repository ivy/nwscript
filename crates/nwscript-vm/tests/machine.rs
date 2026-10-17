//! The machine instruction by instruction, on programs written in NCS assembly.

use nwscript_diag::FileId;
use nwscript_vm::{Engine, FaultKind, Limits, Recording, Reply, Routine, Ty, Value, run};

fn routines() -> Vec<Routine> {
    let r = |name: &str, params: &[Ty], ret: Ty| Routine {
        name: name.into(),
        params: params.to_vec(),
        ret,
    };
    vec![
        r("PrintString", &[Ty::String], Ty::Void),
        r("DelayCommand", &[Ty::Float, Ty::Action], Ty::Void),
        r("GetDistanceBetween", &[Ty::Object, Ty::Object], Ty::Float),
        r("GetGlobalNumber", &[Ty::String], Ty::Int),
        r("GetPosition", &[Ty::Object], Ty::Vector),
        r("PrintVector", &[Ty::Vector, Ty::Int], Ty::Void),
    ]
}

fn assemble(text: &str) -> nwscript_ncs::Program {
    nwscript_ncs::asm::parse(FileId(0), text.as_bytes()).unwrap_or_else(|e| panic!("{e:?}\n{text}"))
}

/// A `StartingConditional` whose body leaves one int on the stack.
fn cond(body: &str) -> nwscript_vm::Run {
    let text = format!(
        "    RSADDI\n    JSR sc\n    RETN\nsc:\n{body}\n    CPDOWNSP -8, 4\n    MOVSP -4\n    RETN\n"
    );
    run(
        &assemble(&text),
        &routines(),
        &mut Recording,
        Limits::default(),
    )
}

fn int(body: &str) -> i32 {
    let r = cond(body);
    assert_eq!(r.fault, None, "{body}");
    match r.result {
        Some(Value::Int(i)) => i,
        other => panic!("{other:?}"),
    }
}

#[test]
fn integer_operations() {
    let bin = |a: i32, b: i32, op: &str| int(&format!("    CONSTI {a}\n    CONSTI {b}\n    {op}"));
    assert_eq!(bin(7, 3, "ADDII"), 10);
    assert_eq!(bin(7, 3, "SUBII"), 4);
    assert_eq!(bin(7, 3, "MULII"), 21);
    assert_eq!(bin(7, 3, "DIVII"), 2);
    assert_eq!(bin(-7, 3, "MODII"), -1);
    assert_eq!(bin(6, 3, "BOOLANDII"), 2);
    assert_eq!(bin(6, 3, "INCORII"), 7);
    assert_eq!(bin(6, 3, "EXCORII"), 5);
    assert_eq!(bin(1, 4, "SHLEFTII"), 16);
    assert_eq!(bin(-16, 2, "SHRIGHTII"), -4);
    assert_eq!(bin(-16, 28, "USHRIGHTII"), 15);
    assert_eq!(bin(2, 0, "LOGANDII"), 0);
    assert_eq!(bin(2, 0, "LOGORII"), 1);
    assert_eq!(bin(2, 3, "LTII"), 1);
    assert_eq!(bin(3, 3, "GEQII"), 1);
    assert_eq!(bin(3, 3, "NEQUALII"), 0);
    assert_eq!(int("    CONSTI 5\n    NEGI"), -5);
    assert_eq!(int("    CONSTI 0\n    COMPI"), -1);
    assert_eq!(int("    CONSTI 0\n    NOTI"), 1);
}

#[test]
fn floats_strings_objects_and_vectors() {
    assert_eq!(
        int("    CONSTI 1\n    CONSTF 1.5\n    ADDIF\n    CONSTF 2.5\n    EQUALFF"),
        1
    );
    assert_eq!(int("    CONSTF 1.0\n    CONSTF 2.0\n    GTFF\n    NOTI"), 1);
    assert_eq!(
        int("    CONSTS \"ab\"\n    CONSTS \"c\"\n    ADDSS\n    CONSTS \"abc\"\n    EQUALSS"),
        1
    );
    assert_eq!(int("    CONSTO 0\n    CONSTO 1\n    NEQUALOO"), 1);
    let v =
        |x: f32, y: f32, z: f32| format!("    CONSTF {x:?}\n    CONSTF {y:?}\n    CONSTF {z:?}\n");
    let body = format!(
        "{}{}    ADDVV\n{}    EQUALTT 12",
        v(1.0, 2.0, 3.0),
        v(1.0, 1.0, 1.0),
        v(2.0, 3.0, 4.0)
    );
    assert_eq!(int(&body), 1);
    let body = format!(
        "{}    CONSTF 2.0\n    MULVF\n{}    EQUALTT 12",
        v(1.0, 2.0, 3.0),
        v(2.0, 4.0, 6.0)
    );
    assert_eq!(int(&body), 1);
}

#[test]
fn stack_copies_destruct_and_increments() {
    // Three values; copy the bottom one to the top; DESTRUCT keeps only the middle.
    assert_eq!(
        int("    CONSTI 1\n    CONSTI 2\n    CONSTI 3\n    CPTOPSP -12, 4\n    DESTRUCT 16, 4, 4"),
        2
    );
    assert_eq!(
        int("    CONSTI 1\n    CONSTI 9\n    CPDOWNSP -8, 4\n    MOVSP -4"),
        9
    );
    assert_eq!(
        int("    CONSTI 5\n    INCISP -4\n    INCISP -4\n    DECISP -4"),
        6
    );
}

#[test]
fn globals_through_bp() {
    // Two globals, then SAVEBP and a call that reads and bumps the first through BP.
    let text = "    RSADDI\n    JSR glob\n    RETN\nglob:\n    CONSTI 40\n    CONSTI 2\n    SAVEBP\n    JSR main\n    RESTOREBP\n    CPTOPSP -8, 4\n    CPDOWNSP -16, 4\n    MOVSP -12\n    RETN\nmain:\n    INCIBP -8\n    CPTOPBP -8, 4\n    CPTOPBP -4, 4\n    ADDII\n    CPDOWNBP -8, 4\n    MOVSP -4\n    RETN\n";
    let r = run(
        &assemble(text),
        &routines(),
        &mut Recording,
        Limits::default(),
    );
    assert_eq!(r.fault, None, "{:?}", r.history);
    assert_eq!(r.result, Some(Value::Int(43)));
}

#[test]
fn conditional_jumps() {
    assert_eq!(
        int("    CONSTI 0\n    JZ yes\n    CONSTI 1\n    JMP done\nyes:\n    CONSTI 2\ndone:"),
        2
    );
    assert_eq!(
        int("    CONSTI 0\n    JNZ yes\n    CONSTI 1\n    JMP done\nyes:\n    CONSTI 2\ndone:"),
        1
    );
}

#[test]
fn engine_arguments_first_parameter_on_top() {
    // GetDistanceBetween(OBJECT_INVALID, OBJECT_SELF): the last argument is pushed first.
    let r = cond("    CONSTO 0\n    CONSTO 1\n    ACTION 2, 2\n    MOVSP -4\n    CONSTI 0");
    assert_eq!(r.fault, None);
    assert_eq!(r.trace[0].args, vec![Value::Object(1), Value::Object(0)]);
    assert_eq!(r.trace[0].ret, Value::Float(0.0));
    assert!(!r.trace[0].modelled);
    assert_eq!(r.unmodelled, vec!["GetDistanceBetween".to_string()]);
}

#[test]
fn vectors_cross_the_engine_boundary() {
    // PrintVector(GetPosition(OBJECT_SELF), 7)
    let r = cond("    CONSTI 7\n    CONSTO 0\n    ACTION 4, 1\n    ACTION 5, 2\n    CONSTI 0");
    assert_eq!(r.fault, None, "{:?}", r.history);
    assert_eq!(
        r.trace[1].args,
        vec![Value::Vector([0.0; 3]), Value::Int(7)]
    );
}

struct Fixed;
impl Engine for Fixed {
    fn call(&mut self, routine: u16, sig: &Routine, _args: &[Value]) -> Reply {
        match routine {
            3 => Reply {
                value: Value::Int(4),
                modelled: true,
            },
            _ => Reply {
                value: Value::default_of(&sig.ret),
                modelled: false,
            },
        }
    }
}

#[test]
fn an_engine_supplies_values() {
    let text = "    RSADDI\n    JSR sc\n    RETN\nsc:\n    CONSTS \"K_X\"\n    ACTION 3, 1\n    CONSTI 4\n    EQUALII\n    CPDOWNSP -8, 4\n    MOVSP -4\n    RETN\n";
    let r = run(&assemble(text), &routines(), &mut Fixed, Limits::default());
    assert_eq!(r.result, Some(Value::Int(1)));
    assert!(r.unmodelled.is_empty());
}

#[test]
fn deferred_actions_run_after_the_body_with_their_saved_stack() {
    // DelayCommand(1.0, PrintString(s)) where s is a local: STORE_STATE saves it.
    let text = "    JSR main\n    RETN\nmain:\n    CONSTS \"later\"\n    STORE_STATE 0, 4\n    JMP after\n    CPTOPSP -4, 4\n    ACTION 0, 1\n    MOVSP -4\n    RETN\nafter:\n    CONSTF 1.0\n    ACTION 1, 2\n    CONSTS \"now\"\n    ACTION 0, 1\n    MOVSP -4\n    RETN\n";
    let p = assemble(text);
    let r = run(&p, &routines(), &mut Recording, Limits::default());
    assert_eq!(r.fault, None, "{:?}", r.history);
    let shown: Vec<String> = r.trace.iter().map(|c| c.to_string()).collect();
    assert_eq!(
        shown,
        [
            "DelayCommand(1.0, <deferred 0>) (unmodelled)",
            "PrintString(\"now\") (unmodelled)",
            "[deferred 0] PrintString(\"later\") (unmodelled)"
        ]
    );
}

#[test]
fn faults() {
    assert_eq!(
        cond("    CONSTI 1\n    CONSTI 0\n    DIVII")
            .fault
            .unwrap()
            .kind,
        FaultKind::DivisionByZero
    );
    assert_eq!(
        cond("loop:\n    JMP loop").fault.unwrap().kind,
        FaultKind::Budget
    );
    assert!(matches!(
        cond("    CONSTI 1\n    ACTION 0, 1\n    CONSTI 0")
            .fault
            .unwrap()
            .kind,
        FaultKind::ArgumentMismatch { routine: 0, .. }
    ));
    assert_eq!(
        cond("    ACTION 99, 0\n    CONSTI 0").fault.unwrap().kind,
        FaultKind::UnknownRoutine(99)
    );
    assert!(matches!(
        cond("    MOVSP -4").fault.unwrap().kind,
        FaultKind::BadStackOffset(_)
    ));
}

#[test]
fn remaining_forms() {
    // Floats and vectors through every arithmetic form.
    assert_eq!(
        int("    CONSTF 2.0\n    NEGF\n    CONSTF -2.0\n    EQUALFF"),
        1
    );
    assert_eq!(
        int("    CONSTF 1.5\n    CONSTF 2.5\n    ADDFF\n    CONSTF 4.0\n    EQUALFF"),
        1
    );
    assert_eq!(
        int("    CONSTF 5.0\n    CONSTI 2\n    SUBFI\n    CONSTF 3.0\n    EQUALFF"),
        1
    );
    assert_eq!(
        int("    CONSTI 5\n    CONSTF 2.0\n    MULIF\n    CONSTF 10.0\n    EQUALFF"),
        1
    );
    assert_eq!(
        int("    CONSTF 1.0\n    CONSTF 4.0\n    DIVFF\n    CONSTF 0.25\n    EQUALFF"),
        1
    );
    assert_eq!(
        int(
            "    CONSTF 1.0\n    CONSTF 2.0\n    LTFF\n    CONSTF 2.0\n    CONSTF 2.0\n    LEQFF\n    LOGANDII"
        ),
        1
    );
    assert_eq!(
        int(
            "    CONSTF 3.0\n    CONSTF 2.0\n    GEQFF\n    CONSTF 2.0\n    CONSTF 2.0\n    NEQUALFF\n    NOTI\n    BOOLANDII"
        ),
        1
    );
    assert_eq!(
        int(
            "    CONSTI 3\n    CONSTI 2\n    GTII\n    CONSTI 2\n    CONSTI 3\n    LEQII\n    LOGANDII"
        ),
        1
    );
    assert_eq!(int("    CONSTI 2\n    CONSTI 3\n    LTII"), 1);
    let v =
        |x: f32, y: f32, z: f32| format!("    CONSTF {x:?}\n    CONSTF {y:?}\n    CONSTF {z:?}\n");
    assert_eq!(
        int(&format!(
            "{}{}    SUBVV\n{}    EQUALTT 12",
            v(3.0, 3.0, 3.0),
            v(1.0, 2.0, 3.0),
            v(2.0, 1.0, 0.0)
        )),
        1
    );
    assert_eq!(
        int(&format!(
            "    CONSTF 2.0\n{}    MULFV\n{}    EQUALTT 12",
            v(1.0, 2.0, 3.0),
            v(2.0, 4.0, 6.0)
        )),
        1
    );
    assert_eq!(
        int(&format!(
            "{}    CONSTF 2.0\n    DIVVF\n{}    NEQUALTT 12\n    NOTI",
            v(2.0, 4.0, 6.0),
            v(1.0, 2.0, 3.0)
        )),
        1
    );
    // Every reservation type, objects and engine structures.
    assert_eq!(
        int(
            "    RSADDF\n    RSADDS\n    RSADDO\n    RSADDE2\n    CPTOPSP -4, 4\n    EQUALEE2\n    CPDOWNSP -16, 4\n    MOVSP -12"
        ),
        1
    );
    assert_eq!(int("    RSADDE0\n    RSADDE0\n    NEQUALEE0\n    NOTI"), 1);
    assert_eq!(int("    CONSTO 0\n    CONSTO 0\n    EQUALOO"), 1);
    assert_eq!(int("    CONSTS \"a\"\n    CONSTS \"b\"\n    NEQUALSS"), 1);
    assert_eq!(int("    NOP\n    CONSTI 1"), 1);
}

#[test]
fn bp_relative_decrement() {
    let text = "    RSADDI\n    JSR glob\n    RETN\nglob:\n    CONSTI 10\n    SAVEBP\n    JSR main\n    RESTOREBP\n    CPTOPSP -4, 4\n    CPDOWNSP -12, 4\n    MOVSP -8\n    RETN\nmain:\n    DECIBP -4\n    DECIBP -4\n    RETN\n";
    let r = run(
        &assemble(text),
        &routines(),
        &mut Recording,
        Limits::default(),
    );
    assert_eq!(r.fault, None, "{:?}", r.history);
    assert_eq!(r.result, Some(Value::Int(8)));
}

#[test]
fn store_state_all_is_reported_unsupported() {
    let r = cond("    STORE_STATEALL\n    CONSTI 0");
    assert_eq!(
        r.fault.unwrap().kind,
        FaultKind::Unsupported("STORE_STATEALL")
    );
}

/// Every opcode appears in a program in this file.
#[test]
fn every_opcode_is_exercised() {
    let src = include_str!("machine.rs");
    let missing: Vec<&str> = nwscript_ncs::Opcode::ALL
        .iter()
        .map(|o| o.name())
        .filter(|n| !src.contains(&format!("    {n}")) && !src.contains(&format!("\"{n}")))
        .collect();
    assert!(missing.is_empty(), "no test runs {missing:?}");
}
