//! Top level, types, defaults, statements and directives: `docs/grammar.md`'s
//! syntactic productions, each as a hand-written snippet. The tree dumps are
//! insta snapshots; the assertions check the typed accessors.

mod common;
use common::*;
use insta::assert_snapshot;

fn dump(src: &str) -> String {
    parse_clean(src).debug_dump()
}

// ---- top level -----------------------------------------------------------

#[test]
fn function_definition_and_prototype() {
    let src = "int Add(int a, int b);\r\nint Add(int a, int b)\r\n{\r\n    return a + b;\r\n}\r\n";
    assert_snapshot!(dump(src));
    let fs: Vec<FunctionDecl> = parse_clean(src).tree().functions().collect();
    assert!(fs[0].is_prototype());
    assert!(!fs[1].is_prototype());
    assert_eq!(fs[1].ret_type().unwrap().kind(), Some(TypeKind::Int));
    let params: Vec<String> = fs[1]
        .params()
        .map(|p| p.name().unwrap().text().to_string())
        .collect();
    assert_eq!(params, ["a", "b"]);
}

#[test]
fn empty_and_void_parameter_lists() {
    let p = parse_clean("void main() {}\r\nvoid f(void);\r\n");
    let fs: Vec<FunctionDecl> = p.tree().functions().collect();
    assert_eq!(fs[0].params().count(), 0);
    assert!(!fs[0].param_list().unwrap().is_void());
    assert!(fs[1].param_list().unwrap().is_void());
    assert_eq!(fs[1].params().count(), 0);
    assert!(fs[0].body().unwrap().statements().next().is_none());
}

#[test]
fn parameter_defaults() {
    let src = "void f(int a = 1, int b = -1, float c = 0.5, string d = \"\", int e = TRUE, \
               object o = OBJECT_SELF, object p = OBJECT_INVALID, vector v = [0.0,0.0,0.0]);";
    assert_snapshot!(dump(src));
    let p = parse_clean(src);
    let f = p.tree().functions().next().unwrap();
    let defaults: Vec<String> = f
        .params()
        .map(|p| sexp(&p.default_value().unwrap()))
        .collect();
    assert_eq!(
        defaults,
        [
            "1",
            "(- 1)",
            "0.5",
            "\"\"",
            "TRUE",
            "OBJECT_SELF",
            "OBJECT_INVALID",
            "[0.0 0.0 0.0]"
        ]
    );
}

#[test]
fn globals() {
    let src = "int A = 1;\r\nfloat B;\r\nstring C = GetStringByStrRef(5);\r\nint D = A, E;\r\nconst int F = 2;\r\n";
    assert_snapshot!(dump(src));
    let p = parse_clean(src);
    let gs: Vec<GlobalDecl> = p.tree().globals().collect();
    assert_eq!(gs.len(), 5);
    assert!(gs[1].declarators().next().unwrap().init().is_none());
    assert_eq!(gs[3].declarators().count(), 2);
    assert!(gs[4].is_const());
    assert!(!gs[0].is_const());
}

#[test]
fn struct_declaration_and_use() {
    let src = "struct tLevel\r\n{\r\n    int nLevel;\r\n    object oTarget;\r\n};\r\nstruct tLevel gLevel;\r\n";
    assert_snapshot!(dump(src));
    let p = parse_clean(src);
    let s = p.tree().structs().next().unwrap();
    assert_eq!(s.name().unwrap().text(), "tLevel");
    let fields: Vec<(String, String)> = s
        .fields()
        .map(|f| {
            (
                f.ty().unwrap().text(),
                f.declarators()
                    .next()
                    .unwrap()
                    .name()
                    .unwrap()
                    .text()
                    .to_string(),
            )
        })
        .collect();
    assert_eq!(
        fields,
        [
            ("int".into(), "nLevel".into()),
            ("object".into(), "oTarget".into())
        ]
    );
    let g = p.tree().globals().next().unwrap();
    match g.ty().unwrap().kind() {
        Some(TypeKind::Struct(Some(n))) => assert_eq!(n.text(), "tLevel"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn empty_declaration() {
    let p = parse_clean(";\r\nint x;;\r\n");
    let kinds: Vec<bool> = p
        .tree()
        .items()
        .map(|i| matches!(i, Item::Empty(_)))
        .collect();
    assert_eq!(kinds, [true, false, true]);
}

#[test]
fn global_after_function_definition() {
    let p = parse_clean("void f() {}\r\nint X = 1;\r\nvoid main() { f(); }\r\n");
    assert_eq!(function_names(&p), ["f", "main"]);
    assert_eq!(p.tree().globals().count(), 1);
}

// ---- types ---------------------------------------------------------------

#[test]
fn every_type() {
    let src = "void a(); int b(); float c(); string d(); object e(); vector f(); \
               effect g(); event h(); location i(); talent j(); itemproperty k(); \
               void l(action x); struct S m();";
    let p = parse_clean(src);
    let kinds: Vec<String> = p
        .tree()
        .functions()
        .map(|f| match f.ret_type().unwrap().kind().unwrap() {
            TypeKind::Named(t) => format!("named {}", t.text()),
            TypeKind::Struct(n) => format!("struct {}", n.unwrap().text()),
            k => format!("{k:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "Void",
            "Int",
            "Float",
            "String",
            "Object",
            "Vector",
            "named effect",
            "named event",
            "named location",
            "named talent",
            "named itemproperty",
            "Void",
            "struct S"
        ]
    );
    let l = p.tree().functions().nth(11).unwrap();
    assert_eq!(
        l.params().next().unwrap().ty().unwrap().kind(),
        Some(TypeKind::Action)
    );
}

/// `nwscript.nss` line 5145 has `INT` and `OBJECT_ID` for `int` and
/// `object`. Under the declaration rule an unknown name in type position is
/// a named type, so the line parses; name resolution reports the types.
#[test]
fn unknown_type_names_parse() {
    let p = parse_clean("void SetAvailableNPCId(INT nNPC, OBJECT_ID oidNPC);");
    let f = p.tree().functions().next().unwrap();
    let types: Vec<String> = f.params().map(|p| p.ty().unwrap().text()).collect();
    assert_eq!(types, ["INT", "OBJECT_ID"]);
}

/// Keywords are matched exactly: `Vector` and `Location` are function names.
#[test]
fn keywords_are_case_sensitive() {
    let p = parse_clean(
        "vector Vector(float x=0.0f, float y=0.0f, float z=0.0f);\r\nlocation Location(object oArea, vector vPosition, float fOrientation);",
    );
    assert_eq!(function_names(&p), ["Vector", "Location"]);
}

// ---- statements ----------------------------------------------------------

#[test]
fn blocks_and_empty_statements() {
    assert_snapshot!(dump("void main() { {} { ; } ;; }"));
}

#[test]
fn local_declarations() {
    let src = "void main() { int a; int b = 1, c, d = b; f(); string s = \"x\"; effect e = EffectHeal(1); struct S t; const int k = 3; }";
    assert_snapshot!(dump(src));
    let stmts = statements("int b = 1, c, d = b;");
    let Stmt::Local(d) = &stmts[0] else { panic!() };
    let names: Vec<String> = d
        .declarators()
        .map(|d| d.name().unwrap().text().to_string())
        .collect();
    assert_eq!(names, ["b", "c", "d"]);
}

/// An identifier followed by an identifier begins a declaration; an
/// identifier followed by anything else begins an expression.
#[test]
fn declaration_rule() {
    let stmts = statements("talent t; t = x; location l = GetLocation(o); l;");
    let kinds: Vec<&str> = stmts
        .iter()
        .map(|s| match s {
            Stmt::Local(_) => "decl",
            Stmt::Expr(_) => "expr",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["decl", "expr", "decl", "expr"]);
}

#[test]
fn if_else() {
    let src = "void main() { if (a) b(); else if (c) { d(); } else e(); }";
    assert_snapshot!(dump(src));
    let stmts = statements("if (a) b(); else if (c) { d(); } else e();");
    let Stmt::If(i) = &stmts[0] else { panic!() };
    assert_eq!(sexp(&i.condition().unwrap()), "a");
    assert!(matches!(i.then_branch(), Some(Stmt::Expr(_))));
    let Some(Stmt::If(inner)) = i.else_branch() else {
        panic!()
    };
    assert!(matches!(inner.then_branch(), Some(Stmt::Block(_))));
    assert!(matches!(inner.else_branch(), Some(Stmt::Expr(_))));
}

/// An `else` binds to the nearest `if`.
#[test]
fn dangling_else() {
    let stmts = statements("if (a) if (b) x(); else y();");
    let Stmt::If(outer) = &stmts[0] else { panic!() };
    assert!(outer.else_branch().is_none());
    let Some(Stmt::If(inner)) = outer.then_branch() else {
        panic!()
    };
    assert!(inner.else_branch().is_some());
}

#[test]
fn loops() {
    let src = "void main() { while (i < 3) { i++; } do { i--; } while (i > 0); for (i = 0; i < 3; i++) { f(i); } }";
    assert_snapshot!(dump(src));
    let stmts = statements("while (a) b(); do c(); while (d); for (i = 0; i < 3; i++) x();");
    let Stmt::While(w) = &stmts[0] else { panic!() };
    assert_eq!(sexp(&w.condition().unwrap()), "a");
    assert!(w.body().is_some());
    let Stmt::Do(d) = &stmts[1] else { panic!() };
    assert_eq!(sexp(&d.condition().unwrap()), "d");
    assert!(d.body().is_some());
    let Stmt::For(f) = &stmts[2] else { panic!() };
    let init: Vec<String> = f.init().unwrap().exprs().map(|e| sexp(&e)).collect();
    assert_eq!(init, ["(= i 0)"]);
    assert_eq!(sexp(&f.condition().unwrap()), "(< i 3)");
    let step: Vec<String> = f.step().iter().map(sexp).collect();
    assert_eq!(step, ["(post++ i)"]);
}

#[test]
fn for_variants() {
    let stmts = statements(
        "for (;;) break; for (int i = 0, j = 1; i < j; i++, j--) {} for (i = 0, j = 1; ; ) continue;",
    );
    let Stmt::For(a) = &stmts[0] else { panic!() };
    assert!(a.init().is_none() && a.condition().is_none() && a.step().is_empty());
    assert!(matches!(a.body(), Some(Stmt::Break(_))));
    let Stmt::For(b) = &stmts[1] else { panic!() };
    let decl = b.init().unwrap().decl().unwrap();
    assert_eq!(decl.declarators().count(), 2);
    assert_eq!(b.step().len(), 2);
    let Stmt::For(c) = &stmts[2] else { panic!() };
    assert_eq!(c.init().unwrap().exprs().count(), 2);
    assert!(c.condition().is_none());
    assert!(matches!(c.body(), Some(Stmt::Continue(_))));
}

#[test]
fn switch() {
    let src = "void main() { switch (n) { case 0: case FORCE_POWER_CHOKE: int x = 1; break; case 2: { f(); } break; default: g(); } }";
    assert_snapshot!(dump(src));
    let stmts = statements("switch (n) { case 1: break; default: break; }");
    let Stmt::Switch(s) = &stmts[0] else { panic!() };
    assert_eq!(sexp(&s.value().unwrap()), "n");
    let Some(Stmt::Block(body)) = s.body() else {
        panic!()
    };
    let labels: Vec<String> = body
        .statements()
        .map(|s| match s {
            Stmt::Case(c) => format!("case {}", sexp(&c.value().unwrap())),
            Stmt::Default(_) => "default".into(),
            Stmt::Break(_) => "break".into(),
            _ => "other".into(),
        })
        .collect();
    assert_eq!(labels, ["case 1", "break", "default", "break"]);
}

#[test]
fn jumps() {
    assert_snapshot!(dump(
        "int f() { while (1) { break; continue; } return(1); return 2; return; }"
    ));
    let stmts = statements("return; return (x);");
    let Stmt::Return(a) = &stmts[0] else { panic!() };
    assert!(a.value().is_none());
    let Stmt::Return(b) = &stmts[1] else { panic!() };
    assert_eq!(sexp(&b.value().unwrap()), "(paren x)");
}

#[test]
fn expression_statements() {
    let stmts = statements("f(); a = 1; a += 2; i++; --i; n == 1;");
    let forms: Vec<String> = stmts
        .iter()
        .map(|s| match s {
            Stmt::Expr(e) => sexp(&e.expr().unwrap()),
            _ => panic!(),
        })
        .collect();
    assert_eq!(
        forms,
        [
            "(call f)",
            "(= a 1)",
            "(+= a 2)",
            "(post++ i)",
            "(-- i)",
            "(== n 1)"
        ]
    );
}

#[test]
fn struct_inside_function() {
    let stmts = statements("struct P { int x; }; struct P p;");
    assert!(matches!(stmts[0], Stmt::Struct(_)));
    assert!(matches!(stmts[1], Stmt::Local(_)));
}

// ---- directives ----------------------------------------------------------

#[test]
fn include() {
    let src = "#include \"k_inc_debug\"\r\n#include \"K_inc_utility\"\r\nvoid main() {}\r\n";
    assert_snapshot!(dump(src));
    let p = parse_clean(src);
    let names: Vec<String> = p.tree().includes().filter_map(|i| i.path()).collect();
    assert_eq!(names, ["k_inc_debug", "K_inc_utility"]);
}

#[test]
fn define() {
    let src = "#define ENGINE_NUM_STRUCTURES 2\r\n#define ENGINE_STRUCTURE_0 effect\r\n#define ENGINE_STRUCTURE_1   event\r\nint X = 1;";
    assert_snapshot!(dump(src));
    let p = parse_clean(src);
    let ds: Vec<Define> = p.tree().defines().collect();
    assert_eq!(ds[0].engine_num_structures(), Some(2));
    assert_eq!(ds[0].value(), "2");
    assert_eq!(ds[1].engine_structure(), Some((0, "effect".into())));
    assert_eq!(ds[2].engine_structure(), Some((1, "event".into())));
    assert_eq!(ds[2].name().unwrap().text(), "ENGINE_STRUCTURE_1");
}

/// The body ends at the line end, also at end of file without one.
#[test]
fn define_body_ends_at_line_end() {
    let p = parse_clean("#define A 1 + 2 // c\r\nint B;\r\n#define C D");
    let ds: Vec<Define> = p.tree().defines().collect();
    assert_eq!(ds[0].value(), "1 + 2");
    assert_eq!(ds[0].value_tokens().len(), 3);
    assert_eq!(ds[1].value(), "D");
    assert_eq!(p.tree().globals().count(), 1);
    let empty = parse_clean("#define E\r\nint F;");
    let d = empty.tree().defines().next().unwrap();
    assert_eq!(d.value(), "");
    assert_eq!(d.engine_structure(), None);
}

#[test]
fn space_after_hash_and_leading_whitespace() {
    let p = parse_clean("  # include \"a\"\r\n\t#define X 1\r\n");
    assert_eq!(p.tree().includes().count(), 1);
    assert_eq!(p.tree().defines().count(), 1);
}

/// `include` and `define` are identifiers outside directives.
#[test]
fn directive_words_are_identifiers_elsewhere() {
    let p = parse_clean("int include = 1; void define() {}");
    assert_eq!(function_names(&p), ["define"]);
}
