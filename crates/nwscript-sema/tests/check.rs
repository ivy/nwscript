//! Semantic analysis on hand-written snippets: the checked form's shape, and
//! every diagnostic's explanation, whose first example must raise it and
//! whose second must check clean.

use nwscript_diag::{Diagnostic, FileId, codes};
use nwscript_sema::hir::*;
use nwscript_sema::{Analysis, ConstValue, EngineTable, ParsedIncludes, Type, analyze};
use nwscript_syntax::parse;

/// A small `nwscript.nss` with the routines and constants the examples use.
const NWSCRIPT: &str = "\
#define ENGINE_NUM_STRUCTURES 4\r
#define ENGINE_STRUCTURE_0 effect\r
#define ENGINE_STRUCTURE_1 event\r
#define ENGINE_STRUCTURE_2 location\r
#define ENGINE_STRUCTURE_3 talent\r
int TRUE = 1;\r
int FALSE = 0;\r
int NPC_BASTILA = 0;\r
int FORCE_POWER_CHOKE = 7;\r
float HALF = 0.5;\r
int Random(int nMaxInteger);\r
void PrintString(string sString);\r
void PrintFloat(float fFloat, int nWidth=18, int nDecimals=9);\r
void AssignCommand(object oActionSubject,action aActionToAssign);\r
void DelayCommand(float fSeconds, action aActionToDelay);\r
object GetObjectByTag(string sTag, int nNth=0);\r
int GetGlobalNumber(string sIdentifier);\r
void SetGlobalNumber(string sIdentifier, int nValue);\r
string IntToString(int nInteger);\r
int GetIsObjectValid(object oObject);\r
object GetFirstPC();\r
int GetGold(object oTarget=OBJECT_SELF);\r
void GiveGoldToCreature(object oCreature, int nGP);\r
void SpeakString(string sStringToSpeak, int nTalkVolume=0);\r
void SetLocked(object oTarget, int bLocked);\r
vector GetPosition(object oTarget);\r
int AddPartyMember(int nNPC, object oCreature);\r
int RemovePartyMember(int nNPC);\r
effect EffectHeal(int nDamageToHeal);\r
void Typo(INT n);\r
vector Vector(float x=0.0, float y=0.0, float z=0.0);\r
";

fn table() -> EngineTable {
    EngineTable::from_nwscript(FileId(1000), NWSCRIPT.as_bytes()).0
}

/// Check `files[0]` as the script, the rest as includes by name.
fn check_files(files: &[(&str, String)]) -> Analysis {
    let t = table();
    let mut inc = ParsedIncludes::new();
    for (i, (name, src)) in files.iter().enumerate().skip(1) {
        let stem = name.trim_end_matches(".nss");
        inc = inc.with(stem, parse(FileId(i as u32), src.as_bytes()));
    }
    analyze(
        files[0].0,
        parse(FileId(0), files[0].1.as_bytes()),
        &t,
        &mut inc,
    )
}

fn check(src: &str) -> Analysis {
    check_files(&[("t.nss", src.to_string())])
}

fn codes_of(a: &Analysis) -> Vec<&'static str> {
    a.diagnostics.iter().map(|d| d.code.id).collect()
}

fn clean(src: &str) -> Program {
    let a = check(src);
    assert!(a.diagnostics.is_empty(), "{:#?}", a.diagnostics);
    a.program
}

fn show(ds: &[Diagnostic]) -> String {
    ds.iter()
        .map(|d| format!("{}: {}", d.code.id, d.message))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The body statements of the function named `name`.
fn body<'p>(p: &'p Program, name: &str) -> &'p [Stmt] {
    &p.function_by_name(name)
        .unwrap()
        .1
        .body
        .as_ref()
        .unwrap()
        .stmts
}

fn expr_of(s: &Stmt) -> &Expr {
    match &s.kind {
        StmtKind::Expr(e) => e,
        StmtKind::Return(Some(e)) => e,
        other => panic!("not an expression statement: {other:?}"),
    }
}

fn call_of(e: &Expr) -> &Call {
    match &e.kind {
        ExprKind::Call(c) => c,
        other => panic!("not a call: {other:?}"),
    }
}

// ---- the explanations ------------------------------------------------------

/// The ```c blocks of an explanation, each split into files at
/// `// file: NAME` lines; the first file is the script.
fn examples(md: &str) -> Vec<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut lines = md.lines();
    while let Some(l) = lines.next() {
        if l.trim() != "```c" {
            continue;
        }
        let mut files = vec![("t.nss".to_string(), String::new())];
        for l in lines.by_ref() {
            if l.trim() == "```" {
                break;
            }
            if let Some(name) = l.strip_prefix("// file: ") {
                files.push((name.trim().to_string(), String::new()));
                continue;
            }
            let f = &mut files.last_mut().unwrap().1;
            f.push_str(l);
            f.push_str("\r\n");
        }
        out.push(files);
    }
    out
}

#[test]
fn every_semantic_code_has_an_example_that_raises_it_and_one_that_fixes_it() {
    let mut checked = 0;
    for code in codes::ALL
        .iter()
        .filter(|c| c.id >= "E0303" && c.id < "E0400")
    {
        let ex = examples(code.explanation);
        assert!(ex.len() >= 2, "{} needs two ```c examples", code.id);
        let as_refs = |files: &[(String, String)]| -> Vec<(&'static str, String)> {
            files
                .iter()
                .map(|(n, s)| (&*n.clone().leak(), s.clone()))
                .collect()
        };
        let bad = check_files(&as_refs(&ex[0]));
        assert!(
            codes_of(&bad).contains(&code.id),
            "{}'s first example does not raise it:\n{}",
            code.id,
            show(&bad.diagnostics)
        );
        let good = check_files(&as_refs(&ex[1]));
        assert!(
            !good.has_errors(),
            "{}'s second example has errors:\n{}",
            code.id,
            show(&good.diagnostics)
        );
        checked += 1;
    }
    assert_eq!(checked, 24);
}

// ---- the checked form --------------------------------------------------------

#[test]
fn engine_calls_carry_every_argument_with_defaults_filled() {
    let p = clean("void main() { PrintFloat(1.5); SpeakString(\"hi\"); }");
    let c = call_of(expr_of(&body(&p, "main")[0]));
    let (n, _) = table().function("PrintFloat").unwrap();
    assert_eq!(c.callee, Callee::Engine(n));
    assert!(matches!(&c.args[0], Arg::Value(e) if e.ty == Type::Float));
    assert!(matches!(c.args[1], Arg::Default(ConstValue::Int(18))));
    assert!(matches!(c.args[2], Arg::Default(ConstValue::Int(9))));
    assert_eq!(expr_of(&body(&p, "main")[1]).ty, Type::Void);
}

#[test]
fn user_calls_take_the_prototypes_defaults() {
    let p = clean(
        "float F(float a = 0.0, float b = 0.75);\r\n\
         float F(float a = 0.4, float b = 1.1) { return a + b; }\r\n\
         void main() { F(); F(2.0); }",
    );
    let (id, f) = p.function_by_name("F").unwrap();
    assert_eq!(f.prototypes.len(), 1);
    assert!(f.definition.is_some());
    let c = call_of(expr_of(&body(&p, "main")[0]));
    assert_eq!(c.callee, Callee::User(id));
    assert!(matches!(c.args[0], Arg::Default(ConstValue::Float(x)) if x == 0.0));
    assert!(matches!(c.args[1], Arg::Default(ConstValue::Float(x)) if x == 0.75));
    let c = call_of(expr_of(&body(&p, "main")[1]));
    assert!(matches!(c.args[0], Arg::Value(_)));
    assert!(matches!(c.args[1], Arg::Default(_)));
}

#[test]
fn action_arguments_are_deferred_void_calls() {
    let p = clean(
        "void Later(string s) { SpeakString(s); }\r\n\
         void main() { DelayCommand(1.0, Later(\"x\")); AssignCommand(OBJECT_SELF, SpeakString(\"y\")); }",
    );
    let c = call_of(expr_of(&body(&p, "main")[0]));
    let Arg::Action(a) = &c.args[1] else {
        panic!("not an action: {:?}", c.args[1])
    };
    assert!(matches!(call_of(a).callee, Callee::User(_)));
    let a = check("void main() { DelayCommand(1.0, GetFirstPC()); }");
    assert_eq!(codes_of(&a), ["E0326"]);
    let a = check("void main() { int n; DelayCommand(1.0, n = 1); }");
    assert_eq!(codes_of(&a), ["E0326"]);
}

#[test]
fn names_resolve_to_locals_params_globals_and_constants() {
    let p = clean(
        "int G = 5;\r\n\
         int F(int a) { int b = a + G + TRUE; { int c = b; } return b; }\r\n\
         void main() { F(1); }",
    );
    let (_, f) = p.function_by_name("F").unwrap();
    assert_eq!(f.locals.len(), 2);
    assert_eq!(f.scopes.len(), 2);
    assert_eq!(f.scope(f.local(LocalId(1)).scope).parent, Some(ScopeId(0)));
    let StmtKind::Decl(d) = &body(&p, "F")[0].kind else {
        panic!()
    };
    let init = d[0].init.as_ref().unwrap();
    let ExprKind::Binary { lhs, rhs, .. } = &init.kind else {
        panic!()
    };
    assert!(
        matches!(&rhs.kind, ExprKind::Constant { name, value: ConstValue::Int(1) } if name == "TRUE")
    );
    let ExprKind::Binary { lhs: a, rhs: g, .. } = &lhs.kind else {
        panic!()
    };
    assert!(matches!(a.kind, ExprKind::Var(Var::Param(0))));
    assert!(matches!(g.kind, ExprKind::Var(Var::Global(GlobalId(0)))));
    assert!(p.globals[0].referenced);
}

#[test]
fn operators_type_by_the_bytecodes_rules() {
    let p = clean(
        "void main() { float f = 1 + 2.0; string s = \"a\" + \"b\"; vector v = GetPosition(OBJECT_SELF) * 2.0; f += 1; int b = s == \"ab\"; }",
    );
    let StmtKind::Decl(d) = &body(&p, "main")[0].kind else {
        panic!()
    };
    assert_eq!(d[0].init.as_ref().unwrap().ty, Type::Float);
    for (src, code) in [
        ("int n = 1 < 2.0;", "E0305"),
        ("float f = 1;", "E0305"),
        ("int n = 1; n += 1.5;", "E0305"),
        ("string s = 1 + \"a\";", "E0305"),
        ("float f = 1.0; f++;", "E0305"),
        ("if (\"x\") {}", "E0305"),
        ("GetFirstPC() = OBJECT_SELF;", "E0310"),
        ("TRUE = 2;", "E0310"),
    ] {
        let a = check(&format!("void main() {{ {src} }}"));
        assert_eq!(codes_of(&a), [code], "{src}");
    }
}

#[test]
fn switch_labels_fold_to_their_values() {
    let p = clean(
        "void main() { int n = 7; switch (n) { case FORCE_POWER_CHOKE: break; case -2: case (3): { } break; default: object o = OBJECT_SELF; } }",
    );
    let StmtKind::Switch { labels, body, .. } = &body(&p, "main")[1].kind else {
        panic!()
    };
    let values: Vec<Option<i32>> = labels.iter().map(|l| l.value).collect();
    assert_eq!(values, [Some(7), Some(-2), Some(3), None]);
    assert!(matches!(
        body.stmts[labels[3].index].kind,
        StmtKind::Default
    ));
    // The local declared after `default:` belongs to the switch body's scope.
    let (_, f) = p.function_by_name("main").unwrap();
    assert_eq!(f.local(LocalId(1)).scope, body.scope);
    for src in [
        "int G = 1; void main() { switch (1) { case G: break; } }",
        "void main() { switch (1) { case 1 + 2: break; } }",
        "void main() { switch (1) { case 1: case 1: break; } }",
        "void main() { switch (1) { default: default: break; } }",
        "void main() { switch (1) { case 1: { case 2: break; } } }",
        "void main() { switch (1) { case \"a\": break; } }",
    ] {
        assert_eq!(codes_of(&check(src)), ["E0315"], "{src}");
    }
}

#[test]
fn structs_and_vectors_have_members() {
    let p = clean(
        "struct S { int a; vector v; object o; };\r\n\
         struct S g;\r\n\
         void main() { g.a = 1; g.v.x = 2.0; float y = GetPosition(OBJECT_SELF).y; }",
    );
    let (id, s) = p.struct_by_name("S").unwrap();
    let offsets: Vec<u32> = s.fields.iter().map(|f| f.offset).collect();
    assert_eq!(offsets, [0, 4, 16]);
    assert_eq!(s.size, 20);
    assert_eq!(p.size_of(&Type::Struct("S".into())), 20);
    let ExprKind::Assign { target, .. } = &expr_of(&body(&p, "main")[1]).kind else {
        panic!()
    };
    let ExprKind::Member { base, member } = &target.kind else {
        panic!()
    };
    assert_eq!(*member, Member::Component(0));
    assert!(
        matches!(base.kind, ExprKind::Member { member: Member::Field { ty, index: 1 }, .. } if ty == id)
    );
    assert_eq!(
        codes_of(&check("void main() { vector v; float w = v.w; }")),
        ["E0321"]
    );
    assert_eq!(
        codes_of(&check("void main() { int n; int m = n.x; }")),
        ["E0321"]
    );
}

#[test]
fn includes_expand_once_ignoring_case() {
    let a = check_files(&[
        (
            "t.nss",
            "#include \"K_INC_A\"\r\n#include \"k_inc_b\"\r\nvoid main() { A(); }".into(),
        ),
        (
            "k_inc_a.nss",
            "int GA = 1;\r\nvoid A() { }\r\nvoid Unused() { }".into(),
        ),
        ("k_inc_b.nss", "#include \"k_inc_a\"\r\nint GB = GA;".into()),
    ]);
    assert!(a.diagnostics.is_empty(), "{}", show(&a.diagnostics));
    let p = &a.program;
    assert_eq!(p.files.len(), 3);
    let names: Vec<&str> = p.globals.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["GA", "GB"]);
    assert!(p.function_by_name("A").unwrap().1.reachable);
    assert!(!p.function_by_name("Unused").unwrap().1.reachable);
    assert_eq!(
        p.entry,
        Some(Entry {
            kind: EntryKind::Main,
            function: p.function_by_name("main").unwrap().0
        })
    );
}

#[test]
fn reachability_follows_calls_actions_and_global_initialisers() {
    let p = clean(
        "int Init() { return 1; }\r\n\
         int G = Init();\r\n\
         void Deferred() { }\r\n\
         void Helper() { DelayCommand(1.0, Deferred()); }\r\n\
         void Orphan() { Helper(); }\r\n\
         int StartingConditional() { Helper(); return TRUE; }",
    );
    let reach: Vec<&str> = p
        .reachable_functions()
        .iter()
        .map(|&id| p.function(id).name.as_str())
        .collect();
    assert_eq!(reach, ["Init", "Deferred", "Helper", "StartingConditional"]);
    assert_eq!(p.entry.unwrap().kind, EntryKind::StartingConditional);
}

#[test]
fn a_library_has_no_entry_and_nothing_reachable() {
    let p = clean("void Unused(); int F() { return 0; }");
    assert!(p.entry.is_none());
    assert!(p.functions.iter().all(|f| !f.reachable));
}

#[test]
fn declarations_are_visible_after_they_appear() {
    let a = check("void main() { F(); }\r\nvoid F() { }");
    assert_eq!(codes_of(&a), ["E0307"]);
    let a = check("int A = B;\r\nint B = 1;\r\nvoid main() { }");
    assert_eq!(codes_of(&a), ["E0307"]);
    // Recursion and a later definition after a prototype are fine.
    clean(
        "int F(int n);\r\nvoid main() { F(3); }\r\nint F(int n) { if (n) return F(n - 1); return 0; }",
    );
}

#[test]
fn a_local_may_hide_a_parameter() {
    // k_inc_gensupport's GN_GetTargetedForcePower declares a local with its
    // parameter's name; BioWare's compiler accepted it.
    let p = clean("int F(int n) { int n; return n; }\r\nvoid main() { F(1); }");
    let StmtKind::Return(Some(e)) = &body(&p, "F")[1].kind else {
        panic!()
    };
    assert!(matches!(e.kind, ExprKind::Var(Var::Local(_))));
}

#[test]
fn bioware_accepted_errors_have_their_own_codes() {
    let a = check("void main() { AddPartyMember(GetObjectByTag(\"bastila\")); }");
    assert_eq!(codes_of(&a), ["E0309", "E0308"]);
    // The call keeps what was written.
    let c = call_of(expr_of(&body(&a.program, "main")[0]));
    assert_eq!(c.args.len(), 1);
    let a = check("void F();\r\nstring F() { return \"\"; }");
    assert_eq!(codes_of(&a), ["E0311"]);
    assert_eq!(a.program.functions[0].ret, Type::String);
    for d in &a.diagnostics {
        assert!(codes::BIOWARE_ACCEPTED.contains(&d.code));
    }
}

#[test]
fn calling_a_routine_whose_declaration_did_not_read_reports_it() {
    let a = check("void main() { Typo(1); }");
    assert_eq!(codes_of(&a), ["E0301"]);
    let a = check("void main() { Typo(1); Typo(2); }");
    assert_eq!(codes_of(&a), ["E0301"]);
}

#[test]
fn return_paths_follow_biowares_rule() {
    clean("int F(int n) { if (n) return 1; else return 2; }");
    clean("int F(int n) { { return 1; } n = 2; }");
    for src in [
        "int F(int n) { if (n) return 1; }",
        "int F(int n) { while (n) { return 1; } }",
        "int F(int n) { switch (n) { default: return 1; } }",
    ] {
        assert_eq!(codes_of(&check(src)), ["E0316"], "{src}");
    }
}

#[test]
fn undeclared_names_suggest_the_nearest() {
    let a = check("void main() { int nGold = 1; nGlod = 2; GetFirstPc(); }");
    assert_eq!(codes_of(&a), ["E0303", "E0304"]);
    assert_eq!(
        a.diagnostics[0].help.as_deref(),
        Some("did you mean `nGold`?")
    );
    assert_eq!(
        a.diagnostics[1].help.as_deref(),
        Some("did you mean `GetFirstPC`?")
    );
}

#[test]
fn spans_point_into_the_file_they_come_from() {
    let a = check_files(&[
        ("t.nss", "#include \"k_inc_x\"\r\nvoid main() { }".into()),
        ("k_inc_x.nss", "void X() { Nope(); }".into()),
    ]);
    assert_eq!(codes_of(&a), ["E0304"]);
    let s = a.diagnostics[0].primary_span().unwrap();
    assert_eq!(s.file, FileId(1));
    assert_eq!(&"void X() { Nope(); }"[s.start..s.end], "Nope");
}

#[test]
fn engine_structures_are_types() {
    let p = clean(
        "effect F() { return EffectHeal(1); }\r\nvoid main() { effect e = F(); location l; }",
    );
    let (_, f) = p.function_by_name("F").unwrap();
    assert_eq!(f.ret, Type::Engine(0));
    assert_eq!(codes_of(&check("void main() { Effect e; }")), ["E0301"]);
    assert_eq!(
        check("void main() { Effect e; }").diagnostics[0]
            .help
            .as_deref(),
        Some("did you mean `effect`? Type names are case-sensitive")
    );
}
