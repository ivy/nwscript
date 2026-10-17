//! Lints: warnings about programs the language accepts. They read the
//! checked program and report; nothing here changes it, and code generation
//! never sees them. Which lints fail a build is decided by the caller's
//! levels (strict, `--compat`, `--allow`, `--deny`, comment directives).
//!
//! Only functions defined in the script itself are linted: an include is a
//! library, linted when it is checked on its own.

use crate::hir::{Arg, Block, ExprKind, FuncId, Program, Stmt, StmtKind, Var};
use crate::types::Type;
use crate::visit::{block_exprs, expr_exprs};
use nwscript_diag::{Diagnostic, Span, codes};
use nwscript_syntax::ast::{AssignOp, BinaryOp};
use std::collections::HashMap;

/// Every lint for `p`.
pub fn lint(p: &Program) -> Vec<Diagnostic> {
    let main_file = p.main().parse.file();
    let mut out = Vec::new();
    for (i, f) in p.functions.iter().enumerate() {
        let Some(body) = &f.body else { continue };
        if f.definition.is_none_or(|s| s.file != main_file) {
            continue;
        }
        unused_locals(p, FuncId(i as u32), body, &mut out);
        shadowing(p, FuncId(i as u32), &mut out);
        unreachable(body, &mut out);
        block_exprs(body, &mut |e| expression_lints(e, &mut out));
        statement_lints(body, &mut out);
        if p.entry.is_some() && !f.reachable {
            out.push(
                Diagnostic::new(
                    &codes::W0003,
                    format!("function `{}` is never called", f.name),
                )
                .primary(f.definition.unwrap(), "defined here")
                .help("remove it, or call it from the entry point"),
            );
        }
    }
    out.sort_by_key(|d| d.primary_span().map(|s| (s.file, s.start)));
    out
}

fn unused_locals(p: &Program, f: FuncId, body: &Block, out: &mut Vec<Diagnostic>) {
    let func = p.function(f);
    let mut reads: HashMap<u32, usize> = HashMap::new();
    let mut plain_writes: HashMap<u32, usize> = HashMap::new();
    block_exprs(body, &mut |e| match &e.kind {
        ExprKind::Var(Var::Local(l)) => *reads.entry(l.0).or_default() += 1,
        ExprKind::Assign {
            op: AssignOp::Assign,
            target,
            ..
        } => {
            if let ExprKind::Var(Var::Local(l)) = target.kind {
                *plain_writes.entry(l.0).or_default() += 1;
            }
        }
        _ => {}
    });
    for (i, local) in func.locals.iter().enumerate() {
        let i = i as u32;
        let used = reads.get(&i).copied().unwrap_or(0) > plain_writes.get(&i).copied().unwrap_or(0);
        if !used {
            out.push(
                Diagnostic::new(&codes::W0001, format!("`{}` is never read", local.name))
                    .primary(local.span, "declared here")
                    .help("remove it, or allow W0001 on its line if it is kept on purpose"),
            );
        }
    }
}

fn shadowing(p: &Program, f: FuncId, out: &mut Vec<Diagnostic>) {
    let func = p.function(f);
    for local in &func.locals {
        if let Some(param) = func.params.iter().find(|x| x.name == local.name) {
            out.push(
                Diagnostic::new(
                    &codes::W0004,
                    format!(
                        "local `{}` hides the parameter of the same name",
                        local.name
                    ),
                )
                .primary(local.span, "this local")
                .secondary(param.span, "the parameter")
                .help("rename one of them; after this declaration the name means the local"),
            );
            continue;
        }
        let mut scope = func.scopes[local.scope.0 as usize].parent;
        while let Some(s) = scope {
            let sc = &func.scopes[s.0 as usize];
            if let Some(outer) = sc
                .locals
                .iter()
                .map(|l| &func.locals[l.0 as usize])
                .find(|o| o.name == local.name && o.span != local.span)
            {
                out.push(
                    Diagnostic::new(
                        &codes::W0004,
                        format!(
                            "local `{}` hides an outer local of the same name",
                            local.name
                        ),
                    )
                    .primary(local.span, "this local")
                    .secondary(outer.span, "the outer one"),
                );
                break;
            }
            scope = sc.parent;
        }
        if let Some(g) = p.globals.iter().find(|g| g.name == local.name) {
            out.push(
                Diagnostic::new(
                    &codes::W0004,
                    format!("local `{}` hides the global of the same name", local.name),
                )
                .primary(local.span, "this local")
                .secondary(g.span, "the global"),
            );
        }
    }
}

fn ends_flow(s: &Stmt) -> bool {
    matches!(
        s.kind,
        StmtKind::Return(_) | StmtKind::Break | StmtKind::Continue
    )
}

fn unreachable(block: &Block, out: &mut Vec<Diagnostic>) {
    let mut ended = false;
    for s in &block.stmts {
        match s.kind {
            // A label is reached by its jump whatever precedes it.
            StmtKind::Case(_) | StmtKind::Default => ended = false,
            StmtKind::Empty => {}
            _ if ended => {
                out.push(
                    Diagnostic::new(&codes::W0005, "this statement is never reached")
                        .primary(s.span, "after a `return`, `break` or `continue`")
                        .help("remove it"),
                );
                // One report per run of dead statements.
                ended = false;
            }
            _ => ended = ends_flow(s),
        }
        nested_blocks(s, &mut |b| unreachable(b, out));
    }
}

fn nested_blocks(s: &Stmt, f: &mut dyn FnMut(&Block)) {
    match &s.kind {
        StmtKind::Block(b) => f(b),
        StmtKind::If {
            then, otherwise, ..
        } => {
            nested_blocks(then, f);
            if let Some(o) = otherwise {
                nested_blocks(o, f);
            }
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. } => nested_blocks(body, f),
        StmtKind::Switch { body, .. } => f(body),
        _ => {}
    }
}

fn has_effect(e: &crate::hir::Expr) -> bool {
    let mut effect = false;
    expr_exprs(e, &mut |x| {
        if matches!(
            x.kind,
            ExprKind::Call(_) | ExprKind::Assign { .. } | ExprKind::IncDec { .. } | ExprKind::Error
        ) {
            effect = true;
        }
    });
    effect
}

fn statement_lints(block: &Block, out: &mut Vec<Diagnostic>) {
    let mut each = |s: &Stmt| {
        if let StmtKind::Expr(e) = &s.kind
            && !has_effect(e)
        {
            out.push(
                Diagnostic::new(
                    &codes::W0007,
                    "this statement computes a value and discards it",
                )
                .primary(e.span, "no effect")
                .help("an `==` meant as `=`? Otherwise remove it"),
            );
        }
    };
    fn walk(b: &Block, each: &mut dyn FnMut(&Stmt)) {
        for s in &b.stmts {
            each(s);
            nested_blocks(s, &mut |inner| walk(inner, each));
            if let StmtKind::If {
                then, otherwise, ..
            } = &s.kind
            {
                each(then);
                if let Some(o) = otherwise {
                    each(o);
                }
            }
        }
    }
    walk(block, &mut each);
}

/// Variables an expression writes and reads.
fn writes_and_reads(e: &crate::hir::Expr) -> (Vec<Var>, Vec<Var>) {
    let (mut w, mut r) = (Vec::new(), Vec::new());
    expr_exprs(e, &mut |x| match &x.kind {
        ExprKind::Assign { target, .. } | ExprKind::IncDec { target, .. } => {
            if let ExprKind::Var(v) = target.kind {
                w.push(v);
            }
        }
        ExprKind::Var(v) => r.push(*v),
        _ => {}
    });
    (w, r)
}

fn expression_lints(e: &crate::hir::Expr, out: &mut Vec<Diagnostic>) {
    match &e.kind {
        ExprKind::Binary { op, lhs, rhs } if !matches!(op, BinaryOp::LogAnd | BinaryOp::LogOr) => {
            let mixed = matches!(
                (&lhs.ty, &rhs.ty),
                (Type::Int, Type::Float) | (Type::Float, Type::Int)
            );
            if mixed {
                let int_side = if lhs.ty == Type::Int { lhs } else { rhs };
                out.push(Diagnostic::new(&codes::W0008, "an `int` is converted to `float` here").primary(int_side.span, "this `int`").help("write the conversion with `IntToFloat(...)`, or use a float literal such as `2.0`"));
            }
        }
        ExprKind::Call(c) => {
            let args: Vec<&crate::hir::Expr> = c
                .args
                .iter()
                .filter_map(|a| if let Arg::Value(x) = a { Some(x) } else { None })
                .collect();
            for (i, a) in args.iter().enumerate() {
                let (writes, _) = writes_and_reads(a);
                for (j, b) in args.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    let (_, reads) = writes_and_reads(b);
                    if writes.iter().any(|v| reads.contains(v)) {
                        out.push(Diagnostic::new(&codes::W0006, "one argument changes a variable another argument reads").primary(a.span, "changes it").secondary(b.span, "reads it").help("arguments are evaluated last to first; compute the change in a statement before the call"));
                        return;
                    }
                }
            }
        }
        _ => {}
    }
}

/// The spans of every comment directive `// nwscript: allow(CODE, …)` in a
/// file's text, and the line each covers: its own line when it follows code,
/// the next line when it stands alone.
pub fn directives(text: &[u8]) -> Vec<(usize, Vec<String>)> {
    let mut out = Vec::new();
    for (n, line) in text.split(|&b| b == b'\n').enumerate() {
        let line = String::from_utf8_lossy(line);
        let Some(at) = line.find("//") else { continue };
        let comment = line[at + 2..].trim();
        let Some(rest) = comment.strip_prefix("nwscript:") else {
            continue;
        };
        let Some(list) = rest
            .trim()
            .strip_prefix("allow(")
            .and_then(|r| r.split(')').next())
        else {
            continue;
        };
        let codes: Vec<String> = list
            .split(',')
            .map(|c| c.trim().to_ascii_uppercase())
            .filter(|c| !c.is_empty())
            .collect();
        let alone = line[..at].trim().is_empty();
        out.push((if alone { n + 2 } else { n + 1 }, codes));
    }
    out
}

/// Drop diagnostics a directive allows: lints only, on the line it covers.
pub fn apply_directives(diags: Vec<Diagnostic>, map: &nwscript_diag::SourceMap) -> Vec<Diagnostic> {
    let mut cache: HashMap<nwscript_diag::FileId, Vec<(usize, Vec<String>)>> = HashMap::new();
    diags
        .into_iter()
        .filter(|d| {
            if d.code.severity != nwscript_diag::Severity::Warning {
                return true;
            }
            let Some(s): Option<Span> = d.primary_span() else {
                return true;
            };
            let file = map.get(s.file);
            let line = file.line_col(s.start).line;
            let dirs = cache
                .entry(s.file)
                .or_insert_with(|| directives(&file.bytes));
            !dirs
                .iter()
                .any(|(l, cs)| *l == line && cs.iter().any(|c| c == d.code.id))
        })
        .collect()
}
