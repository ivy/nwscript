//! Helpers shared by the parser tests.

#![allow(dead_code)]

use nwscript_diag::FileId;
pub use nwscript_syntax::ast::*;
use nwscript_syntax::{Parse, parse};

/// Parse, and check the tree prints back to the input.
pub fn parse_ok(src: &str) -> Parse {
    let p = parse(FileId(0), src.as_bytes());
    assert_eq!(p.to_bytes(), src.as_bytes(), "print-back differs");
    p
}

/// Parse and require no diagnostics.
pub fn parse_clean(src: &str) -> Parse {
    let p = parse_ok(src);
    assert!(
        p.diagnostics().is_empty(),
        "unexpected diagnostics for {src:?}:\n{}",
        p.debug_dump()
    );
    p
}

/// The codes of the diagnostics, in order.
pub fn codes(p: &Parse) -> Vec<&'static str> {
    p.diagnostics().iter().map(|d| d.code.id).collect()
}

/// The statements of `void main() { body }`.
pub fn statements(body: &str) -> Vec<Stmt> {
    let p = parse_clean(&format!("void main() {{ {body} }}"));
    let f = p.tree().functions().next().unwrap();
    f.body().unwrap().statements().collect()
}

/// The expression statement `src;` in a function, as an s-expression.
pub fn expr(src: &str) -> String {
    let stmts = statements(&format!("{src};"));
    match &stmts[..] {
        [Stmt::Expr(e)] => sexp(&e.expr().unwrap()),
        other => panic!("not one expression statement: {other:?}"),
    }
}

/// A compact form of an expression tree, built from the typed accessors.
pub fn sexp(e: &Expr) -> String {
    let s = |e: Option<Expr>| e.as_ref().map_or("?".to_string(), sexp);
    match e {
        Expr::Literal(l) => match l.value() {
            Some(LiteralValue::String(b)) => format!("{:?}", String::from_utf8_lossy(&b)),
            Some(LiteralValue::ObjectSelf) => "OBJECT_SELF".into(),
            Some(LiteralValue::ObjectInvalid) => "OBJECT_INVALID".into(),
            Some(LiteralValue::Int(n)) => n.to_string(),
            Some(LiteralValue::Float(f)) => format!("{f:?}"),
            None => "?lit".into(),
        },
        Expr::Name(n) => n.name(),
        Expr::Paren(p) => format!("(paren {})", s(p.expr())),
        Expr::Call(c) => {
            let mut out = format!("(call {}", c.name().unwrap().text());
            for a in c.args() {
                out.push(' ');
                out.push_str(&sexp(&a));
            }
            out + ")"
        }
        Expr::Member(m) => format!("(. {} {})", s(m.base()), m.field().unwrap().text()),
        Expr::Postfix(p) => format!("(post{} {})", p.op_token().unwrap().text(), s(p.operand())),
        Expr::Prefix(p) => format!("({} {})", p.op_token().unwrap().text(), s(p.operand())),
        Expr::Binary(b) => format!(
            "({} {} {})",
            b.op_token().unwrap().text(),
            s(b.lhs()),
            s(b.rhs())
        ),
        Expr::Assign(a) => format!(
            "({} {} {})",
            a.op_token().unwrap().text(),
            s(a.target()),
            s(a.value())
        ),
        Expr::Cond(c) => format!(
            "(? {} {} {})",
            s(c.condition()),
            s(c.then_expr()),
            s(c.else_expr())
        ),
        Expr::Vector(v) => {
            let parts: Vec<String> = v.elements().map(|e| sexp(&e)).collect();
            format!("[{}]", parts.join(" "))
        }
    }
}

/// The names of the file's functions, in order.
pub fn function_names(p: &Parse) -> Vec<String> {
    p.tree()
        .functions()
        .filter_map(|f| f.name())
        .map(|t| t.text().to_string())
        .collect()
}

pub fn cast<N: AstNode>(node: &impl AstNode) -> N {
    N::cast(node.syntax().clone()).unwrap()
}
