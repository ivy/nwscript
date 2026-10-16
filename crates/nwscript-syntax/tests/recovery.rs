//! Error recovery: one injected mistake yields exactly one diagnostic, the
//! file still prints back byte-identical, and the declarations after the
//! mistake still parse.

mod common;
use common::*;

/// A script with one mistake in `main`, followed by a later function.
fn with_mistake(body: &str) -> nwscript_syntax::Parse {
    parse_ok(&format!(
        "int G = 1;\r\nvoid main()\r\n{{\r\n    int n = 0;\r\n{body}\r\n    n++;\r\n}}\r\nvoid later()\r\n{{\r\n    G = 2;\r\n}}\r\n"
    ))
}

fn one_error(body: &str, code: &str) {
    let p = with_mistake(body);
    assert_eq!(codes(&p), [code], "{body:?}\n{}", p.debug_dump());
    assert_eq!(function_names(&p), ["main", "later"], "{body:?}");
    let later = p.tree().functions().nth(1).unwrap();
    assert_eq!(later.body().unwrap().statements().count(), 1);
}

#[test]
fn missing_semicolon() {
    one_error("    n = 1", "E0205");
    one_error("    int m = 2", "E0205");
    one_error("    f()", "E0205");
}

#[test]
fn missing_close_paren() {
    one_error("    f(n, 2;", "E0205");
    one_error("    if (n == 1 { f(); }", "E0205");
    one_error("    while (n < 3 n++;", "E0205");
    one_error("    n = (1 + 2;", "E0205");
}

#[test]
fn stray_token() {
    one_error("    n = 1 );", "E0205");
    one_error("    ) n = 1;", "E0205");
    one_error("    n = * 2;", "E0205");
    one_error("    n = 1 @;", "E0201");
    one_error("    else n = 1;", "E0205");
}

#[test]
fn unterminated_block_comment() {
    one_error("    n = 1; /* never closed", "E0202");
}

#[test]
fn unterminated_string() {
    one_error("    f(\"never closed);", "E0203");
}

#[test]
fn invalid_number() {
    one_error("    n = 1.;", "E0204");
}

#[test]
fn missing_close_brace_before_next_function() {
    let p = parse_ok(
        "void main()\r\n{\r\n    if (x) {\r\n        f();\r\n}\r\nvoid later() { g(); }\r\n",
    );
    assert_eq!(codes(&p), ["E0205"], "{}", p.debug_dump());
    assert_eq!(function_names(&p), ["main", "later"]);
}

#[test]
fn missing_close_brace_at_end_of_file() {
    let p = parse_ok("void main() { f();");
    assert_eq!(codes(&p), ["E0205"]);
}

#[test]
fn top_level_mistakes() {
    for src in [
        "int A = 1\r\nvoid later() {}",
        "} void later() {}",
        "int A = ;\r\nvoid later() {}",
        "void f(int a,) {}\r\nvoid later() {}",
        "void f()\r\nvoid later() {}",
        "struct S { int a; int b }; void later() {}",
    ] {
        let p = parse_ok(src);
        assert_eq!(codes(&p), ["E0205"], "{src:?}\n{}", p.debug_dump());
        assert!(function_names(&p).contains(&"later".to_string()), "{src:?}");
    }
}

#[test]
fn directive_mistakes() {
    for src in [
        "#pragma once\r\nvoid later() {}",
        "#include <k_inc>\r\nvoid later() {}",
        "#include \"a\" x\r\nvoid later() {}",
        "int A; #include \"a\"\r\nvoid later() {}",
        "void f() {\r\n#include \"a\"\r\n}\r\nvoid later() {}",
    ] {
        let p = parse_ok(src);
        assert_eq!(p.diagnostics().len(), 1, "{src:?}\n{}", p.debug_dump());
        assert!(function_names(&p).contains(&"later".to_string()), "{src:?}");
    }
}

#[test]
fn messages_name_what_was_expected() {
    let p = with_mistake("    n = 1");
    let d = &p.diagnostics()[0];
    assert_eq!(d.message, "expected `;`, found identifier `n`");
}

#[test]
fn hostile_inputs_print_back() {
    for src in [
        "",
        "(",
        ")",
        "{",
        "}",
        "[",
        "#",
        "#define",
        "#include",
        "\"",
        "/*",
        "void",
        "void f(",
        "void f() { if",
        "int a = b ? c",
        "x y z",
        "struct",
        "struct S {",
        "for (;;",
        "case 1:",
        "void f() { case ; default }",
        "a.b.c.()",
        "f(,,)",
        "[[[",
        "}}}}",
        "\r\r\n\n",
    ] {
        let p = parse_ok(src);
        let _ = p.debug_dump();
    }
}

/// Nesting past the parser's limit is one diagnostic, not a stack overflow,
/// and the next declaration still parses.
#[test]
fn nesting_limit() {
    for src in [
        format!(
            "void main() {{ x = {}1; }}\r\nvoid later() {{}}",
            "(".repeat(100_000)
        ),
        format!(
            "void main() {{ x = {}; }}\r\nvoid later() {{}}",
            "-".repeat(100_000)
        ),
        format!(
            "void main() {} \r\nvoid later() {{}}",
            "{ if (x) ".repeat(50_000)
        ),
    ] {
        let p = parse_ok(&src);
        assert_eq!(codes(&p), ["E0207"]);
        assert_eq!(function_names(&p), ["main", "later"]);
    }
    // Within the limit, nothing is reported.
    parse_clean(&format!(
        "void main() {{ x = {}1{}; }}",
        "(".repeat(100),
        ")".repeat(100)
    ));
}
