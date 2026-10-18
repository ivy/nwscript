//! Recovery and verification on hand-assembled programs, against a small
//! engine table written for these tests.

use nwscript_decomp::{CompileError, Notes, Status, decompile};
use nwscript_diag::FileId;
use nwscript_ncs::{Program, asm, encode};
use nwscript_sema::EngineTable;

/// Routine numbers 0 to 8, as the fixtures call them.
const ENGINE: &str = r#"#define ENGINE_NUM_STRUCTURES 1
#define ENGINE_STRUCTURE_0 effect
int Random(int nMaxInteger);
void PrintString(string sString);
void PrintFloat(float fFloat, int nWidth = 18, int nDecimals = 9);
string FloatToString(float fFloat, int nWidth = 18, int nDecimals = 9);
void PrintInteger(int nInteger);
void PrintObject(object oObject);
void AssignCommand(object oActionSubject, action aActionToAssign);
void DelayCommand(float fSeconds, action aActionToDelay);
vector Vector(float x = 0.0f, float y = 0.0f, float z = 0.0f);
"#;

fn table() -> EngineTable {
    let (t, diags) = EngineTable::from_nwscript(FileId(0), ENGINE.as_bytes());
    assert!(diags.is_empty(), "{diags:?}");
    t
}

fn program(text: &str) -> Program {
    asm::parse(FileId(1), text.as_bytes()).expect("the fixture assembles")
}

fn source_of(d: &nwscript_decomp::Decompiled) -> String {
    let src = d.source();
    let parsed = nwscript_syntax::parse(FileId(2), &src);
    assert!(
        parsed.diagnostics().is_empty(),
        "the recovered source does not parse: {:?}\n{}",
        parsed.diagnostics(),
        String::from_utf8_lossy(&src)
    );
    String::from_utf8(src).unwrap()
}

const CONTROL: &str = include_str!("fixtures/control.asm");
const VALUES: &str = include_str!("fixtures/values.asm");

#[test]
fn control_flow_reads_back_as_statements() {
    let t = table();
    let d = decompile(&program(CONTROL), &t).unwrap();
    assert!(d.functions.iter().all(|f| f.status == Status::Unchecked));
    insta::assert_snapshot!(source_of(&d));
}

#[test]
fn values_types_and_deferred_actions() {
    let t = table();
    let d = decompile(&program(VALUES), &t).unwrap();
    assert!(d.functions.iter().all(|f| f.status == Status::Unchecked));
    insta::assert_snapshot!(source_of(&d));
}

#[test]
fn functions_are_named_and_ranged() {
    let t = table();
    let d = decompile(&program(CONTROL), &t).unwrap();
    let names: Vec<(&str, u32, u32)> = d
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.start, f.end))
        .collect();
    assert_eq!(
        names,
        [
            ("(globals)", 13, 61),
            ("main", 61, 638),
            ("sub_27e", 638, 840)
        ]
    );
}

#[test]
fn a_compiler_that_reproduces_the_bytes_verifies_every_function() {
    let t = table();
    let p = program(CONTROL);
    let mut d = decompile(&p, &t).unwrap();
    let mut compiler = |_: &[u8]| Ok(encode(&p));
    d.verify(&p, &mut compiler);
    assert_eq!(d.verified(), 3);
    let src = source_of(&d);
    assert!(src.starts_with("// Recovered from NCS bytecode: 3 of 3 functions verified"));
    assert_eq!(
        src.matches("// verified: recompiles to the original bytes")
            .count(),
        3
    );
}

#[test]
fn a_difference_is_reported_at_its_original_offset() {
    let t = table();
    let p = program(CONTROL);
    let mut d = decompile(&p, &t).unwrap();
    // The recompiled function at 638 returns 11 where the original returns 10.
    let changed = CONTROL.replacen(
        "    CONSTI       10\n    CPDOWNSP",
        "    CONSTI       11\n    CPDOWNSP",
        1,
    );
    let other = program(&changed);
    let mut compiler = |_: &[u8]| Ok(encode(&other));
    d.verify(&p, &mut compiler);
    let ins = p.instructions();
    let k = (0..ins.len() - 1)
        .find(|&k| {
            ins[k].to_string() == "CONSTI 10" && ins[k + 1].to_string().starts_with("CPDOWNSP")
        })
        .unwrap();
    let at = p.offsets()[k];
    assert_eq!(d.functions[0].status, Status::Verified);
    assert_eq!(d.functions[1].status, Status::Verified);
    assert_eq!(
        d.functions[2].status,
        Status::Unverified {
            at: Some(at),
            detail: String::new()
        }
    );
    assert!(source_of(&d).contains(&format!(
        "// unverified: recompiles differently from offset {at}\nint sub_27e("
    )));

    // With only the unverified marked and no header, the verified globals
    // and main carry no comment, and sub_27e its code.
    d.set_notes(Notes::Unverified);
    d.set_header(Vec::new());
    let quiet = source_of(&d);
    assert!(!quiet.contains("// verified"), "{quiet}");
    assert!(!quiet.starts_with('\n'), "{quiet}");
    assert!(quiet.contains(&format!(
        "// E0602: unverified: recompiles differently from offset {at}\nint sub_27e("
    )));
    assert_eq!(quiet.matches("//").count(), 1, "{quiet}");
}

#[test]
fn a_function_the_compiler_rejects_is_set_aside_and_the_rest_checked() {
    let t = table();
    let p = program(CONTROL);
    let mut d = decompile(&p, &t).unwrap();
    let mut calls = 0;
    let mut compiler = |src: &[u8]| {
        calls += 1;
        let text = String::from_utf8_lossy(src);
        if text.contains("placeholder") {
            return Ok(encode(&p));
        }
        let line = text.lines().position(|l| l.contains("return 10;")).unwrap() as u32 + 1;
        Err(CompileError {
            message: "Error: something".into(),
            line: Some(line),
        })
    };
    d.verify(&p, &mut compiler);
    assert_eq!(calls, 2);
    assert_eq!(d.verified(), 2);
    assert!(matches!(
        &d.functions[2].status,
        Status::Unverified { at: None, detail } if detail == "does not compile: Error: something"
    ));
    // The output keeps the recovered body, not the placeholder.
    let src = source_of(&d);
    assert!(src.contains("return 10;") && !src.contains("placeholder"));
}

#[test]
fn a_compile_error_without_a_line_leaves_everything_unverified() {
    let t = table();
    let p = program(VALUES);
    let mut d = decompile(&p, &t).unwrap();
    let mut compiler = |_: &[u8]| {
        Err(CompileError {
            message: "no".into(),
            line: None,
        })
    };
    d.verify(&p, &mut compiler);
    assert_eq!(d.verified(), 0);
    assert!(d.functions.iter().all(|f| matches!(
        &f.status,
        Status::Unverified { at: None, detail } if detail.ends_with("does not compile: no")
    )));
}

#[test]
fn an_unknown_shape_is_not_recovered_and_the_rest_still_prints() {
    let t = table();
    // `sub` pops two values at once, which no statement does.
    let p = program(
        "    JSR main\n    RETN\nmain:\n    JSR sub\n    RETN\nsub:\n    CONSTI 1\n    CONSTI 2\n    MOVSP -8\n    RETN\n",
    );
    let d = decompile(&p, &t).unwrap();
    assert_eq!(d.functions[0].status, Status::Unchecked);
    match &d.functions[1].status {
        Status::NotRecovered { reason } => {
            assert_eq!(
                reason,
                "a MOVSP inside an expression at offset 41 (MOVSP -8)"
            )
        }
        other => panic!("{other:?}"),
    }
    let src = source_of(&d);
    assert!(src.contains("// not recovered: "), "{src}");
}

#[test]
fn bytecode_without_an_entry_stub_is_an_error() {
    let t = table();
    let p = program("    CONSTI 1\n    RETN\n");
    assert!(decompile(&p, &t).is_err());
}

#[test]
fn starting_conditional_returns_its_value() {
    let t = table();
    // int StartingConditional() { return Random(2) == 1; }
    let p = program(
        "    RSADDI\n    JSR sc\n    RETN\nsc:\n    CONSTI 2\n    ACTION 0, 1\n    CONSTI 1\n    EQUALII\n    CPDOWNSP -8, 4\n    MOVSP -4\n    JMP end\n    MOVSP -4\nend:\n    RETN\n",
    );
    let d = decompile(&p, &t).unwrap();
    let src = source_of(&d);
    assert!(
        src.contains("int StartingConditional()\n{\n    return Random(2) == 1;\n}"),
        "{src}"
    );
}
