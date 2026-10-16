//! `parse` on arbitrary bytes: it never panics, the tree prints back to the
//! exact input whatever it was, every diagnostic's span lies within the
//! input, every node's byte span holds that node's text, and the typed
//! accessors and the debug dump walk any tree.

#![no_main]

use libfuzzer_sys::fuzz_target;
use nwscript_diag::FileId;
use nwscript_syntax::ast::AstNode;
use nwscript_syntax::text::latin1_bytes;
use nwscript_syntax::{ast, parse};

fuzz_target!(|data: &[u8]| {
    let p = parse(FileId(0), data);
    assert_eq!(p.to_bytes(), data);
    for d in p.diagnostics() {
        let s = d.primary_span().expect("a diagnostic without a span");
        assert!(s.start <= s.end && s.end <= data.len());
    }
    let root = p.syntax();
    for node in root.descendants() {
        let s = p.span_of(node.clone());
        assert_eq!(latin1_bytes(&node.to_string()), &data[s.start..s.end]);
        if let Some(e) = ast::Expr::cast(node.clone()) {
            walk_expr(&e);
        }
        if let Some(f) = ast::FunctionDecl::cast(node.clone()) {
            for param in f.params() {
                let _ = (param.ty().and_then(|t| t.kind()), param.default_value());
            }
        }
        if let Some(d) = ast::Define::cast(node) {
            let _ = (d.value(), d.engine_structure(), d.engine_num_structures());
        }
    }
    let _ = p.debug_dump();
});

fn walk_expr(e: &ast::Expr) {
    match e {
        ast::Expr::Literal(l) => {
            let _ = l.value();
        }
        ast::Expr::Binary(b) => {
            let _ = (b.op(), b.lhs(), b.rhs());
        }
        ast::Expr::Assign(a) => {
            let _ = (a.op(), a.target(), a.value());
        }
        ast::Expr::Prefix(p) => {
            let _ = p.op();
        }
        ast::Expr::Cond(c) => {
            let _ = (c.condition(), c.then_expr(), c.else_expr());
        }
        ast::Expr::Call(c) => {
            let _ = (c.name(), c.args().count());
        }
        _ => {}
    }
}
