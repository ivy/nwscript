//! Walking the checked program: every expression node in a block, statement
//! or expression, outer before inner, in source order.

use crate::hir::{Arg, Block, Declared, Expr, ExprKind, ForInit, Stmt, StmtKind};

/// Call `f` on every expression in `block`, nested ones included.
pub fn block_exprs(block: &Block, f: &mut dyn FnMut(&Expr)) {
    for s in &block.stmts {
        stmt_exprs(s, f);
    }
}

/// Call `f` on every expression in `stmt`, nested ones included.
pub fn stmt_exprs(stmt: &Stmt, f: &mut dyn FnMut(&Expr)) {
    let decls = |ds: &[Declared], f: &mut dyn FnMut(&Expr)| {
        for d in ds {
            if let Some(e) = &d.init {
                expr_exprs(e, f);
            }
        }
    };
    match &stmt.kind {
        StmtKind::Block(b) => block_exprs(b, f),
        StmtKind::Decl(ds) => decls(ds, f),
        StmtKind::Expr(e) => expr_exprs(e, f),
        StmtKind::If {
            cond,
            then,
            otherwise,
        } => {
            expr_exprs(cond, f);
            stmt_exprs(then, f);
            if let Some(o) = otherwise {
                stmt_exprs(o, f);
            }
        }
        StmtKind::While { cond, body } => {
            expr_exprs(cond, f);
            stmt_exprs(body, f);
        }
        StmtKind::DoWhile { body, cond } => {
            stmt_exprs(body, f);
            expr_exprs(cond, f);
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            match init {
                ForInit::Exprs(es) => es.iter().for_each(|e| expr_exprs(e, f)),
                ForInit::Decl(ds) => decls(ds, f),
            }
            if let Some(c) = cond {
                expr_exprs(c, f);
            }
            step.iter().for_each(|e| expr_exprs(e, f));
            stmt_exprs(body, f);
        }
        StmtKind::Switch { value, body, .. } => {
            expr_exprs(value, f);
            block_exprs(body, f);
        }
        StmtKind::Return(Some(e)) => expr_exprs(e, f),
        StmtKind::Return(None)
        | StmtKind::Case(_)
        | StmtKind::Default
        | StmtKind::Break
        | StmtKind::Continue
        | StmtKind::Empty => {}
    }
}

/// Call `f` on `e` and every expression inside it.
pub fn expr_exprs(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    f(e);
    match &e.kind {
        ExprKind::Literal(_) | ExprKind::Constant { .. } | ExprKind::Var(_) | ExprKind::Error => {}
        ExprKind::Member { base, .. } => expr_exprs(base, f),
        ExprKind::Call(c) => {
            for a in &c.args {
                match a {
                    Arg::Value(x) | Arg::Action(x) => expr_exprs(x, f),
                    Arg::Default(_) => {}
                }
            }
        }
        ExprKind::Unary { operand, .. } => expr_exprs(operand, f),
        ExprKind::Binary { lhs, rhs, .. } => {
            expr_exprs(lhs, f);
            expr_exprs(rhs, f);
        }
        ExprKind::Assign { target, value, .. } => {
            expr_exprs(target, f);
            expr_exprs(value, f);
        }
        ExprKind::IncDec { target, .. } => expr_exprs(target, f),
        ExprKind::Cond {
            cond,
            then,
            otherwise,
        } => {
            expr_exprs(cond, f);
            expr_exprs(then, f);
            expr_exprs(otherwise, f);
        }
        ExprKind::Vector(parts) => parts.iter().for_each(|p| expr_exprs(p, f)),
    }
}
