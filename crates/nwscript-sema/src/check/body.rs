//! Checking function bodies, global initialisers and parameter defaults:
//! scopes, name resolution, types, calls.

use super::ops::{binary_type, fold, unary_type};
use super::{Checker, suggest};
use crate::hir::*;
use crate::types::{ConstValue, Type};
use nwscript_diag::{Diagnostic, Span, codes};
use nwscript_syntax::SyntaxElement;
use nwscript_syntax::ast::{self, AstNode, LiteralValue, PrefixOp};
use std::collections::HashMap;

/// The signature a call is checked against.
struct Sig {
    name: String,
    ret: Type,
    params: Vec<(String, Type, Option<ConstValue>)>,
    engine: bool,
    decl: Span,
}

/// The state of checking one function body, global initialiser or default.
pub(super) struct Body<'c, 't> {
    pub c: &'c mut Checker<'t>,
    /// The file the code is in, as an index into the checker's files.
    pub fi: usize,
    /// Declarations at or before this position in source order are visible.
    pub seq: u32,
    pub params: Vec<Param>,
    pub ret: Type,
    pub locals: Vec<Local>,
    pub scopes: Vec<Scope>,
    stack: Vec<(ScopeId, HashMap<String, LocalId>)>,
    loops: u32,
    switches: u32,
    pub callees: Vec<FuncId>,
    pub call_sites: Vec<(FuncId, Span)>,
}

impl<'c, 't> Body<'c, 't> {
    pub fn new(c: &'c mut Checker<'t>, fi: usize, seq: u32, params: Vec<Param>, ret: Type) -> Self {
        Body {
            c,
            fi,
            seq,
            params,
            ret,
            locals: Vec::new(),
            scopes: Vec::new(),
            stack: Vec::new(),
            loops: 0,
            switches: 0,
            callees: Vec::new(),
            call_sites: Vec::new(),
        }
    }

    fn span(&self, el: impl Into<SyntaxElement>) -> Span {
        self.c.files[self.fi].parse.span_of(el)
    }

    fn node_span(&self, n: &impl AstNode) -> Span {
        self.span(n.syntax().clone())
    }

    fn error(&mut self, d: Diagnostic) {
        self.c.diags.push(d);
    }

    fn push_scope(&mut self, span: Span) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        self.scopes.push(Scope {
            parent: self.stack.last().map(|(s, _)| *s),
            locals: Vec::new(),
            span,
        });
        self.stack.push((id, HashMap::new()));
        id
    }

    fn pop_scope(&mut self) {
        self.stack.pop();
    }

    // ---- statements ------------------------------------------------------

    /// A function's body: its block, then the check that a function with a
    /// value returns one on every path.
    pub fn function_body(&mut self, b: &ast::Block, name_span: Span, name: &str) -> Block {
        let block = self.block(b, None);
        if self.ret != Type::Void && !block_returns(&block) {
            let ret = self.c.show(&self.ret);
            self.error(
                Diagnostic::new(
                    &codes::E0316,
                    format!("`{name}` can reach its end without returning a value"),
                )
                .primary(name_span, format!("declared to return `{ret}`"))
                .secondary(Span::new(block.span.file, block.span.end.saturating_sub(1), block.span.end), "the end of the body is reachable")
                .help("end the body with a `return`, or give every branch of the last `if` an `else` that returns; a `switch` or a loop never counts as returning"),
            );
        }
        block
    }

    fn block(&mut self, b: &ast::Block, mut labels: Option<&mut Vec<SwitchLabel>>) -> Block {
        let span = self.node_span(b);
        let scope = self.push_scope(span);
        let mut stmts = Vec::new();
        for (i, s) in b.statements().enumerate() {
            let st = match (&s, labels.as_deref_mut()) {
                (ast::Stmt::Case(l), Some(labels)) => self.case_label(l, i, labels),
                (ast::Stmt::Default(l), Some(labels)) => {
                    let span = self.node_span(l);
                    if let Some(prev) = labels.iter().find(|l| l.value.is_none()) {
                        let prev = prev.span;
                        self.error(
                            Diagnostic::new(&codes::E0315, "a switch with two `default` labels")
                                .primary(span, "second `default`")
                                .secondary(prev, "first `default`"),
                        );
                    }
                    labels.push(SwitchLabel {
                        value: None,
                        index: i,
                        span,
                    });
                    Stmt {
                        kind: StmtKind::Default,
                        span,
                    }
                }
                _ => self.stmt(&s),
            };
            stmts.push(st);
        }
        self.pop_scope();
        Block { scope, stmts, span }
    }

    fn case_label(
        &mut self,
        l: &ast::CaseLabel,
        index: usize,
        labels: &mut Vec<SwitchLabel>,
    ) -> Stmt {
        let span = self.node_span(l);
        let value = l.value().map(|e| self.expr(&e));
        let folded = match &value {
            Some(e) if is_error(e) => None,
            Some(e) => match fold(e) {
                Some(ConstValue::Int(v)) => Some(v),
                _ => {
                    let what = if e.ty == Type::Int {
                        "not a constant".to_string()
                    } else {
                        format!("a `{}`, not an `int` constant", self.c.show(&e.ty))
                    };
                    self.error(
                        Diagnostic::new(&codes::E0315, "a `case` label that is not an integer constant")
                            .primary(e.span, what)
                            .help("a label is an integer literal, a negated one, or an `int` constant from nwscript.nss such as `FORCE_POWER_CHOKE`; a script's own globals are variables"),
                    );
                    None
                }
            },
            None => None,
        };
        if let Some(v) = folded
            && let Some(prev) = labels.iter().find(|l| l.value == Some(v))
        {
            let prev = prev.span;
            self.error(
                Diagnostic::new(
                    &codes::E0328,
                    format!("two `case` labels with the value {v}"),
                )
                .primary(span, "second label")
                .secondary(prev, "first label"),
            );
        }
        labels.push(SwitchLabel {
            value: folded,
            index,
            span,
        });
        Stmt {
            kind: folded.map_or(StmtKind::Empty, StmtKind::Case),
            span,
        }
    }

    fn sub_stmt(&mut self, s: Option<ast::Stmt>, at: Span) -> Box<Stmt> {
        Box::new(match s {
            Some(s) => self.stmt(&s),
            None => Stmt {
                kind: StmtKind::Empty,
                span: at,
            },
        })
    }

    fn cond(&mut self, e: Option<ast::Expr>, at: Span, what: &str) -> Expr {
        match e {
            Some(e) => {
                let x = self.expr(&e);
                if !is_error(&x) && x.ty != Type::Int {
                    let ty = self.c.show(&x.ty);
                    self.error(
                        Diagnostic::new(&codes::E0305, format!("{what} is a `{ty}`, not an `int`"))
                            .primary(x.span, format!("this is a `{ty}`"))
                            .help("a condition is an `int`: zero is false, anything else true; compare the value to get one"),
                    );
                }
                x
            }
            None => error_expr(at),
        }
    }

    fn stmt(&mut self, s: &ast::Stmt) -> Stmt {
        let span = self.node_span(s);
        let kind = match s {
            ast::Stmt::Block(b) => StmtKind::Block(self.block(b, None)),
            ast::Stmt::Local(d) => StmtKind::Decl(self.local_decl(d)),
            ast::Stmt::Struct(d) => {
                let span = self.node_span(d);
                self.error(
                    Diagnostic::new(&codes::E0325, "a struct declared inside a function")
                        .primary(span, "")
                        .help("declare the struct at the top level of the file, before the functions that use it"),
                );
                StmtKind::Empty
            }
            ast::Stmt::Expr(e) => match e.expr() {
                Some(x) => StmtKind::Expr(self.expr(&x)),
                None => StmtKind::Empty,
            },
            ast::Stmt::Empty(_) => StmtKind::Empty,
            ast::Stmt::If(i) => StmtKind::If {
                cond: self.cond(i.condition(), span, "the condition of `if`"),
                then: self.sub_stmt(i.then_branch(), span),
                otherwise: i.else_branch().map(|e| Box::new(self.stmt(&e))),
            },
            ast::Stmt::While(w) => {
                let cond = self.cond(w.condition(), span, "the condition of `while`");
                self.loops += 1;
                let body = self.sub_stmt(w.body(), span);
                self.loops -= 1;
                StmtKind::While { cond, body }
            }
            ast::Stmt::Do(d) => {
                self.loops += 1;
                let body = self.sub_stmt(d.body(), span);
                self.loops -= 1;
                let cond = self.cond(d.condition(), span, "the condition of `do ... while`");
                StmtKind::DoWhile { body, cond }
            }
            ast::Stmt::For(f) => {
                let mut scope = None;
                let init = match f.init() {
                    Some(i) => match i.decl() {
                        Some(d) => {
                            scope = Some(self.push_scope(span));
                            ForInit::Decl(self.local_decl(&d))
                        }
                        None => ForInit::Exprs(i.exprs().map(|e| self.expr(&e)).collect()),
                    },
                    None => ForInit::Exprs(Vec::new()),
                };
                let cond = f
                    .condition()
                    .map(|c| self.cond(Some(c), span, "the condition of `for`"));
                let step = f.step().iter().map(|e| self.expr(e)).collect();
                self.loops += 1;
                let body = self.sub_stmt(f.body(), span);
                self.loops -= 1;
                if scope.is_some() {
                    self.pop_scope();
                }
                StmtKind::For {
                    scope,
                    init,
                    cond,
                    step,
                    body,
                }
            }
            ast::Stmt::Switch(sw) => {
                let value = self.cond(sw.value(), span, "the value of `switch`");
                self.switches += 1;
                let mut labels = Vec::new();
                let body = match sw.body() {
                    Some(ast::Stmt::Block(b)) => self.block(&b, Some(&mut labels)),
                    other => {
                        // A switch whose body is not a block has no labels to
                        // jump to; the statement still checks.
                        let span = other.as_ref().map_or(span, |o| self.node_span(o));
                        let scope = self.push_scope(span);
                        let stmts = other.map(|o| self.stmt(&o)).into_iter().collect();
                        self.pop_scope();
                        Block { scope, stmts, span }
                    }
                };
                self.switches -= 1;
                StmtKind::Switch {
                    value,
                    body,
                    labels,
                }
            }
            ast::Stmt::Case(_) | ast::Stmt::Default(_) => {
                self.error(
                    Diagnostic::new(&codes::E0315, "a label outside the body of a `switch`")
                        .primary(span, "")
                        .help("`case` and `default` labels stand directly in a switch's braces, not in a block nested inside them"),
                );
                StmtKind::Empty
            }
            ast::Stmt::Break(_) => {
                if self.loops == 0 && self.switches == 0 {
                    self.error(
                        Diagnostic::new(&codes::E0314, "`break` outside a loop or switch")
                            .primary(span, "nothing to break out of"),
                    );
                }
                StmtKind::Break
            }
            ast::Stmt::Continue(_) => {
                if self.loops == 0 {
                    self.error(
                        Diagnostic::new(&codes::E0314, "`continue` outside a loop")
                            .primary(span, "no loop to continue"),
                    );
                }
                StmtKind::Continue
            }
            ast::Stmt::Return(r) => {
                let value = r.value().map(|v| self.expr(&v));
                self.check_return(value.as_ref(), span);
                StmtKind::Return(value)
            }
        };
        Stmt { kind, span }
    }

    fn check_return(&mut self, value: Option<&Expr>, span: Span) {
        let ret = self.ret.clone();
        match (value, &ret) {
            (Some(v), Type::Void) => self.error(
                Diagnostic::new(
                    &codes::E0317,
                    "a `return` with a value in a `void` function",
                )
                .primary(v.span, "this value has nowhere to go")
                .help("remove the value, or declare the function with the type it returns"),
            ),
            (None, Type::Void) => {}
            (None, t) => {
                let t = self.c.show(t);
                self.error(
                    Diagnostic::new(
                        &codes::E0317,
                        format!("a `return` without a value in a function that returns `{t}`"),
                    )
                    .primary(span, format!("needs a `{t}`")),
                )
            }
            (Some(v), t) => {
                if !is_error(v) && v.ty != *t {
                    let (vt, t) = (self.c.show(&v.ty), self.c.show(t));
                    self.error(
                        Diagnostic::new(
                            &codes::E0305,
                            format!("returning a `{vt}` from a function that returns `{t}`"),
                        )
                        .primary(v.span, format!("this is a `{vt}`"))
                        .help(conversion_help(&v.ty, &ret)),
                    );
                }
            }
        }
    }

    pub fn local_decl(&mut self, d: &ast::LocalDecl) -> Vec<Declared> {
        let ty = d
            .ty()
            .and_then(|t| self.c.resolve_type(self.fi, &t, self.seq));
        if let (Some(t), Some(ast_ty)) = (&ty, d.ty())
            && matches!(t, Type::Void | Type::Action)
        {
            let span = self.node_span(&ast_ty);
            self.error(
                Diagnostic::new(&codes::E0323, format!("a variable of type `{t}`"))
                    .primary(span, "no value has this type")
                    .help("variables hold `int`, `float`, `string`, `object`, `vector`, a struct, or an engine structure such as `effect`"),
            );
        }
        let mut out = Vec::new();
        for decl in d.declarators() {
            let Some(name) = decl.name() else { continue };
            let span = self.node_span(&decl);
            let init = decl.init().map(|e| self.expr(&e));
            if let (Some(i), Some(t)) = (&init, &ty) {
                self.expect_type(i, t, "the initialiser");
            }
            let local = self.declare_local(name.text(), ty.clone().unwrap_or(Type::Void), span);
            out.push(Declared { local, init });
        }
        out
    }

    fn declare_local(&mut self, name: &str, ty: Type, span: Span) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        let (scope, names) = self
            .stack
            .last_mut()
            .expect("a local is declared in a scope");
        let scope = *scope;
        if let Some(prev) = names.get(name) {
            let prev = self.locals[prev.0 as usize].span;
            self.c.diags.push(
                Diagnostic::new(
                    &codes::E0313,
                    format!("`{name}` is declared twice in the same block"),
                )
                .primary(span, "second declaration")
                .secondary(prev, "first declaration")
                .help("rename one of them, or assign to the first instead of declaring again"),
            );
        }
        self.stack
            .last_mut()
            .unwrap()
            .1
            .insert(name.to_string(), id);
        self.scopes[scope.0 as usize].locals.push(id);
        self.locals.push(Local {
            name: name.to_string(),
            ty,
            scope,
            span,
        });
        id
    }

    /// Report a value whose type is not the one required.
    fn expect_type(&mut self, value: &Expr, want: &Type, what: &str) -> bool {
        if is_error(value) || value.ty == *want || *want == Type::Void {
            return true;
        }
        let (vt, wt) = (self.c.show(&value.ty), self.c.show(want));
        self.error(
            Diagnostic::new(
                &codes::E0305,
                format!("{what} is a `{vt}` where a `{wt}` is expected"),
            )
            .primary(value.span, format!("this is a `{vt}`"))
            .help(conversion_help(&value.ty, want)),
        );
        false
    }

    // ---- expressions -----------------------------------------------------

    pub fn expr(&mut self, e: &ast::Expr) -> Expr {
        let span = self.node_span(e);
        match e {
            ast::Expr::Literal(l) => self.literal(l, span),
            ast::Expr::Name(n) => self.name(n, span),
            ast::Expr::Paren(p) => match p.expr() {
                Some(inner) => self.expr(&inner),
                None => error_expr(span),
            },
            ast::Expr::Call(c) => self.call(c, span),
            ast::Expr::Member(m) => self.member(m, span),
            ast::Expr::Postfix(p) => self.incdec(p.operand(), p.op(), false, span),
            ast::Expr::Prefix(p) => {
                match p.op() {
                    Some(PrefixOp::Inc) => self.incdec(p.operand(), Some(IncDec::Inc), true, span),
                    Some(PrefixOp::Dec) => self.incdec(p.operand(), Some(IncDec::Dec), true, span),
                    Some(op) => {
                        let op = match op {
                            PrefixOp::Neg => UnaryOp::Neg,
                            PrefixOp::Plus => UnaryOp::Plus,
                            PrefixOp::Not => UnaryOp::Not,
                            _ => UnaryOp::BitNot,
                        };
                        let Some(operand) = p.operand().map(|o| self.expr(&o)) else {
                            return error_expr(span);
                        };
                        let ty = if is_error(&operand) {
                            None
                        } else {
                            let t = unary_type(op, &operand.ty);
                            if t.is_none() {
                                let ot = self.c.show(&operand.ty);
                                let tok = p.op_token().map_or(span, |t| self.span(t));
                                self.error(
                                Diagnostic::new(&codes::E0305, format!("`{}` does not apply to a `{ot}`", tok_text(p.op_token())))
                                    .primary(tok, "")
                                    .secondary(operand.span, format!("this is a `{ot}`"))
                                    .help("`-` takes an `int` or a `float`; `!` and `~` take an `int`"),
                            );
                            }
                            t
                        };
                        match ty {
                            Some(ty) => Expr {
                                kind: ExprKind::Unary {
                                    op,
                                    operand: Box::new(operand),
                                },
                                ty,
                                span,
                            },
                            None => error_expr(span),
                        }
                    }
                    None => error_expr(span),
                }
            }
            ast::Expr::Binary(b) => self.binary(b, span),
            ast::Expr::Assign(a) => self.assign(a, span),
            ast::Expr::Cond(c) => {
                let cond = self.cond(c.condition(), span, "the condition of `?:`");
                let then = c.then_expr().map(|e| self.expr(&e));
                let otherwise = c.else_expr().map(|e| self.expr(&e));
                let (Some(then), Some(otherwise)) = (then, otherwise) else {
                    return error_expr(span);
                };
                if is_error(&then) || is_error(&otherwise) {
                    return error_expr(span);
                }
                if then.ty != otherwise.ty {
                    let (a, b) = (self.c.show(&then.ty), self.c.show(&otherwise.ty));
                    self.error(
                        Diagnostic::new(
                            &codes::E0305,
                            format!("the branches of `?:` are a `{a}` and a `{b}`"),
                        )
                        .primary(otherwise.span, format!("this is a `{b}`"))
                        .secondary(then.span, format!("this is a `{a}`"))
                        .help("both branches have the same type"),
                    );
                    return error_expr(span);
                }
                let ty = then.ty.clone();
                Expr {
                    kind: ExprKind::Cond {
                        cond: Box::new(cond),
                        then: Box::new(then),
                        otherwise: Box::new(otherwise),
                    },
                    ty,
                    span,
                }
            }
            ast::Expr::Vector(v) => {
                let parts: Vec<Expr> = v.elements().map(|x| self.expr(&x)).collect();
                if parts.len() != 3 {
                    self.error(
                        Diagnostic::new(
                            &codes::E0305,
                            format!("a vector of {} elements", parts.len()),
                        )
                        .primary(span, "a vector has three: `[x, y, z]`"),
                    );
                    return error_expr(span);
                }
                let mut ok = true;
                for p in &parts {
                    ok &= self.expect_type(p, &Type::Float, "a vector element");
                }
                if !ok || parts.iter().any(is_error) {
                    return error_expr(span);
                }
                let parts: [Expr; 3] = parts.try_into().expect("three elements");
                Expr {
                    kind: ExprKind::Vector(Box::new(parts)),
                    ty: Type::Vector,
                    span,
                }
            }
        }
    }

    fn literal(&mut self, l: &ast::Literal, span: Span) -> Expr {
        let v = match l.value() {
            Some(LiteralValue::Int(i)) => match i32::try_from(i)
                .or_else(|_| u32::try_from(i).map(|u| u as i32))
            {
                Ok(v) => ConstValue::Int(v),
                Err(_) => {
                    self.error(
                        Diagnostic::new(&codes::E0204, "an integer literal too large for an `int`")
                            .primary(span, "an `int` is 32 bits")
                            .help("the largest is 2147483647, or 0xffffffff written in hex"),
                    );
                    return error_expr(span);
                }
            },
            Some(LiteralValue::Float(f)) => ConstValue::Float(f),
            Some(LiteralValue::String(s)) => ConstValue::String(s),
            Some(LiteralValue::ObjectSelf) => ConstValue::Object(0),
            Some(LiteralValue::ObjectInvalid) => ConstValue::Object(1),
            None => return error_expr(span),
        };
        Expr {
            ty: v.ty(),
            kind: ExprKind::Literal(v),
            span,
        }
    }

    /// A name used as a value: a local, a parameter, a global, or a
    /// `nwscript.nss` constant, searched in that order.
    fn name(&mut self, n: &ast::NameRef, span: Span) -> Expr {
        let name = n.name();
        for (_, names) in self.stack.iter().rev() {
            if let Some(&id) = names.get(&name) {
                return Expr {
                    kind: ExprKind::Var(Var::Local(id)),
                    ty: self.locals[id.0 as usize].ty.clone(),
                    span,
                };
            }
        }
        if let Some(i) = self.params.iter().position(|p| p.name == name) {
            return Expr {
                kind: ExprKind::Var(Var::Param(i as u32)),
                ty: self.params[i].ty.clone(),
                span,
            };
        }
        if let Some(&id) = self.c.global_names.get(&name) {
            let g = &self.c.globals[id.0 as usize];
            let (gs, gspan, ty) = (g.seq, g.span, g.ty.clone());
            if gs >= self.seq {
                self.error(
                    Diagnostic::new(
                        &codes::E0307,
                        format!("`{name}` is used before its declaration"),
                    )
                    .primary(span, "used here")
                    .secondary(gspan, "declared here, later")
                    .help("move the declaration above its first use"),
                );
            }
            return Expr {
                kind: ExprKind::Var(Var::Global(id)),
                ty,
                span,
            };
        }
        if let Some(c) = self.c.table.constant(&name) {
            return Expr {
                ty: c.value.ty(),
                kind: ExprKind::Constant {
                    name,
                    value: c.value.clone(),
                },
                span,
            };
        }
        let mut d = Diagnostic::new(
            &codes::E0303,
            format!("no variable or constant named `{name}`"),
        )
        .primary(
            span,
            "not declared in this script, its includes, or nwscript.nss",
        );
        if let Some(s) = self.suggest_value(&name) {
            d = d.help(format!("did you mean `{s}`?"));
        } else if self.c.function_names.contains_key(&name)
            || self.c.table.function(&name).is_some()
        {
            d = d.help(format!(
                "`{name}` is a function; call it with `{name}(...)`"
            ));
        }
        self.error(d);
        error_expr(span)
    }

    fn suggest_value(&self, name: &str) -> Option<String> {
        let mut cands: Vec<&str> = Vec::new();
        for (_, names) in &self.stack {
            cands.extend(names.keys().map(String::as_str));
        }
        cands.extend(self.params.iter().map(|p| p.name.as_str()));
        cands.extend(self.c.global_names.keys().map(String::as_str));
        cands.extend(self.c.table.constants.iter().map(|c| c.name.as_str()));
        suggest(name, cands.into_iter()).map(str::to_string)
    }

    fn member(&mut self, m: &ast::MemberExpr, span: Span) -> Expr {
        let Some(base) = m.base().map(|b| self.expr(&b)) else {
            return error_expr(span);
        };
        let Some(field) = m.field() else {
            return error_expr(span);
        };
        if is_error(&base) {
            return error_expr(span);
        }
        let fname = field.text().to_string();
        let fspan = self.span(field);
        let found = match &base.ty {
            Type::Vector => match fname.as_str() {
                "x" => Some((Member::Component(0), Type::Float)),
                "y" => Some((Member::Component(1), Type::Float)),
                "z" => Some((Member::Component(2), Type::Float)),
                _ => None,
            },
            Type::Struct(s) => self.c.struct_names.get(s).and_then(|&(id, _)| {
                let def = &self.c.structs[id.0 as usize];
                def.fields.iter().position(|f| f.name == fname).map(|i| {
                    (
                        Member::Field {
                            ty: id,
                            index: i as u32,
                        },
                        def.fields[i].ty.clone(),
                    )
                })
            }),
            _ => None,
        };
        match found {
            Some((member, ty)) => Expr {
                kind: ExprKind::Member {
                    base: Box::new(base),
                    member,
                },
                ty,
                span,
            },
            None => {
                let bt = self.c.show(&base.ty);
                let help = match &base.ty {
                    Type::Vector => "a vector's members are `x`, `y` and `z`".to_string(),
                    Type::Struct(s) => match self.c.struct_names.get(s) {
                        Some(&(id, _)) => {
                            let names: Vec<String> = self.c.structs[id.0 as usize]
                                .fields
                                .iter()
                                .map(|f| format!("`{}`", f.name))
                                .collect();
                            format!("`struct {s}` has {}", names.join(", "))
                        }
                        None => format!("`struct {s}` is not declared"),
                    },
                    _ => "only structs and vectors have members".to_string(),
                };
                self.error(
                    Diagnostic::new(&codes::E0321, format!("a `{bt}` has no member `{fname}`"))
                        .primary(fspan, "no such member")
                        .secondary(base.span, format!("this is a `{bt}`"))
                        .help(help),
                );
                error_expr(span)
            }
        }
    }

    fn incdec(
        &mut self,
        operand: Option<ast::Expr>,
        op: Option<IncDec>,
        prefix: bool,
        span: Span,
    ) -> Expr {
        let (Some(operand), Some(op)) = (operand, op) else {
            return error_expr(span);
        };
        let target = self.expr(&operand);
        if is_error(&target) {
            return error_expr(span);
        }
        let sym = if op == IncDec::Inc { "++" } else { "--" };
        if !target.is_place() {
            self.error(
                Diagnostic::new(
                    &codes::E0310,
                    format!("`{sym}` of something that is not a variable"),
                )
                .primary(target.span, "not a variable")
                .help("`++` and `--` change a variable, a struct field or a vector component"),
            );
            return error_expr(span);
        }
        if target.ty != Type::Int {
            let t = self.c.show(&target.ty);
            self.error(
                Diagnostic::new(&codes::E0305, format!("`{sym}` of a `{t}`"))
                    .primary(target.span, format!("this is a `{t}`"))
                    .help(format!(
                        "`{sym}` takes an `int`; for a `float`, write `x = x + 1.0`"
                    )),
            );
            return error_expr(span);
        }
        Expr {
            kind: ExprKind::IncDec {
                op,
                prefix,
                target: Box::new(target),
            },
            ty: Type::Int,
            span,
        }
    }

    fn binary(&mut self, b: &ast::BinExpr, span: Span) -> Expr {
        let (Some(op), Some(l), Some(r)) = (b.op(), b.lhs(), b.rhs()) else {
            return error_expr(span);
        };
        let lhs = self.expr(&l);
        let rhs = self.expr(&r);
        if is_error(&lhs) || is_error(&rhs) {
            return error_expr(span);
        }
        match binary_type(op, &lhs.ty, &rhs.ty) {
            Some(ty) => Expr {
                kind: ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                ty,
                span,
            },
            None => {
                let (lt, rt) = (self.c.show(&lhs.ty), self.c.show(&rhs.ty));
                let sym = tok_text(b.op_token());
                let tok = b.op_token().map_or(span, |t| self.span(t));
                self.error(
                    Diagnostic::new(
                        &codes::E0305,
                        format!("`{sym}` does not apply to a `{lt}` and a `{rt}`"),
                    )
                    .primary(tok, "")
                    .secondary(lhs.span, format!("a `{lt}`"))
                    .secondary(rhs.span, format!("a `{rt}`"))
                    .help(operator_help(op)),
                );
                error_expr(span)
            }
        }
    }

    fn assign(&mut self, a: &ast::AssignExpr, span: Span) -> Expr {
        let (Some(op), Some(t), Some(v)) = (a.op(), a.target(), a.value()) else {
            return error_expr(span);
        };
        let target = self.expr(&t);
        let value = self.expr(&v);
        if is_error(&target) || is_error(&value) {
            return error_expr(span);
        }
        let sym = tok_text(a.op_token());
        if !target.is_place() {
            self.error(
                Diagnostic::new(&codes::E0310, format!("`{sym}` stores to something that is not a variable"))
                    .primary(target.span, "not a variable")
                    .help("the left side of an assignment is a variable, a struct field or a vector component"),
            );
            return error_expr(span);
        }
        let ok = match op.binary_op() {
            None => target.ty == value.ty,
            Some(bop) => binary_type(bop, &target.ty, &value.ty).as_ref() == Some(&target.ty),
        };
        if !ok {
            let (tt, vt) = (self.c.show(&target.ty), self.c.show(&value.ty));
            let help = if op == AssignOp::Assign {
                conversion_help(&value.ty, &target.ty)
            } else {
                format!(
                    "`a {sym} b` is `a = a {} b`, and its result must be a `{tt}`",
                    &sym[..sym.len() - 1]
                )
            };
            self.error(
                Diagnostic::new(&codes::E0305, format!("`{sym}` of a `{vt}` to a `{tt}`"))
                    .primary(value.span, format!("this is a `{vt}`"))
                    .secondary(target.span, format!("this is a `{tt}`"))
                    .help(help),
            );
            return error_expr(span);
        }
        let ty = target.ty.clone();
        Expr {
            kind: ExprKind::Assign {
                op,
                target: Box::new(target),
                value: Box::new(value),
            },
            ty,
            span,
        }
    }

    /// Resolve a call's name: a script function declared so far, then an
    /// engine routine.
    fn resolve_call(&mut self, name: &str, name_span: Span) -> Option<(Callee, Sig)> {
        if let Some(&id) = self.c.function_names.get(name) {
            let f = &self.c.functions[id.0 as usize];
            let sig = Sig {
                name: f.name.clone(),
                ret: f.ret.clone(),
                params: f
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), p.ty.clone(), p.default.clone()))
                    .collect(),
                engine: false,
                decl: f
                    .prototypes
                    .first()
                    .copied()
                    .or(f.definition)
                    .unwrap_or(name_span),
            };
            if f.decl_seq > self.seq {
                let decl = sig.decl;
                self.error(
                    Diagnostic::new(&codes::E0307, format!("`{name}` is called before its declaration"))
                        .primary(name_span, "called here")
                        .secondary(decl, "declared here, later")
                        .help(format!("declare a prototype above the first call, such as `{};`, or move the function up", self.c.prototype_text(id))),
                );
            }
            return Some((Callee::User(id), sig));
        }
        let (n, f) = self.c.table.function(name)?;
        if f.recovered && self.c.recovered_reported.insert(n) {
            self.c.diags.push(
                Diagnostic::new(&codes::E0301, format!("`{name}`'s declaration in nwscript.nss names types that do not exist"))
                    .primary(name_span, "called here")
                    .secondary(f.span, "the declaration")
                    .help("correct the type names in nwscript.nss (types are lower case: `int`, `object`)")
                    .note("the routine keeps its number; the signature this call is checked against is a recovery"),
            );
        }
        Some((
            Callee::Engine(n),
            Sig {
                name: f.name.clone(),
                ret: f.ret.clone(),
                params: f
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), p.ty.clone(), p.default.clone()))
                    .collect(),
                engine: true,
                decl: f.span,
            },
        ))
    }

    fn call(&mut self, c: &ast::CallExpr, span: Span) -> Expr {
        let Some(name_tok) = c.name() else {
            return error_expr(span);
        };
        let name = name_tok.text().to_string();
        let name_span = self.span(name_tok);
        let args_ast: Vec<ast::Expr> = c.args().collect();
        let Some((callee, sig)) = self.resolve_call(&name, name_span) else {
            for a in &args_ast {
                self.expr(a);
            }
            let mut d = Diagnostic::new(&codes::E0304, format!("no function named `{name}`"))
                .primary(
                    name_span,
                    "not declared in this script, its includes, or nwscript.nss",
                );
            let mut cands: Vec<&str> = self.c.function_names.keys().map(String::as_str).collect();
            cands.extend(self.c.table.functions.iter().map(|f| f.name.as_str()));
            if let Some(s) = suggest(&name, cands.into_iter()) {
                d = d.help(format!("did you mean `{s}`?"));
            } else {
                d = d.help("declare it in this script or an include; `nws fn --grep PATTERN` searches the engine's routines");
            }
            self.error(d);
            return error_expr(span);
        };
        if let Callee::User(id) = callee {
            if !self.callees.contains(&id) {
                self.callees.push(id);
            }
            self.call_sites.push((id, name_span));
        }
        let (arity_code, type_code) = if sig.engine {
            (&codes::E0308, &codes::E0309)
        } else {
            (&codes::E0306, &codes::E0305)
        };
        let mut args = Vec::new();
        for (i, a) in args_ast.iter().enumerate() {
            let param = sig.params.get(i);
            let x = self.expr(a);
            match param {
                Some((pname, Type::Action, _)) => {
                    let is_void_call = matches!(x.kind, ExprKind::Call(_)) && x.ty == Type::Void;
                    if !is_error(&x) && !is_void_call {
                        let what = match &x.kind {
                            ExprKind::Call(_) => {
                                format!("a call that returns `{}`", self.c.show(&x.ty))
                            }
                            _ => "not a call".to_string(),
                        };
                        self.error(
                            Diagnostic::new(&codes::E0326, format!("argument {} of `{}`, `{pname}`, is an action, and this is {what}", i + 1, sig.name))
                                .primary(x.span, what)
                                .secondary(sig.decl, "the parameter is declared `action`")
                                .help("pass a call to a function that returns `void`; the engine stores it and runs it later. To run a call that returns a value, wrap it in a `void` function of your own"),
                        );
                    }
                    args.push(Arg::Action(x));
                }
                Some((pname, pty, _)) => {
                    if !is_error(&x) && x.ty != *pty {
                        let (vt, pt) = (self.c.show(&x.ty), self.c.show(pty));
                        let mut d = Diagnostic::new(
                            type_code,
                            format!(
                                "argument {} of `{}` is a `{vt}`, but `{pname}` is a `{pt}`",
                                i + 1,
                                sig.name
                            ),
                        )
                        .primary(x.span, format!("this is a `{vt}`"))
                        .secondary(sig.decl, "declared here")
                        .help(conversion_help(&x.ty, pty));
                        if sig.engine {
                            d = d.note("BioWare's compiler accepted vanilla scripts like this because it read declarations that did not ship; `--compat bioware` accepts them too");
                        }
                        self.error(d);
                    }
                    args.push(Arg::Value(x));
                }
                None => args.push(Arg::Value(x)),
            }
        }
        if args_ast.len() > sig.params.len() {
            let extra = self.node_span(&args_ast[sig.params.len()]);
            let mut d = Diagnostic::new(
                arity_code,
                format!(
                    "`{}` takes {} but is given {}",
                    sig.name,
                    plural(sig.params.len(), "argument"),
                    args_ast.len()
                ),
            )
            .primary(extra, "unexpected argument")
            .secondary(sig.decl, "declared here");
            if sig.engine {
                d = d.note("BioWare's compiler accepted vanilla scripts like this because it read declarations that did not ship; `--compat bioware` accepts them too");
            }
            self.error(d.help(format!(
                "`{}`",
                self.c.signature_text(&sig.name, &sig.ret, &sig.params)
            )));
        } else {
            for (i, (pname, _, default)) in sig.params.iter().enumerate().skip(args_ast.len()) {
                match default {
                    Some(v) => args.push(Arg::Default(v.clone())),
                    None => {
                        let at = c.arg_list().map_or(name_span, |l| self.node_span(&l));
                        let required = sig.params.iter().filter(|p| p.2.is_none()).count();
                        let mut d = Diagnostic::new(
                            arity_code,
                            format!(
                                "`{}` needs {} but is given {}",
                                sig.name,
                                plural(required, "argument"),
                                args_ast.len()
                            ),
                        )
                        .primary(at, format!("argument {} (`{pname}`) is missing", i + 1))
                        .secondary(sig.decl, "declared here")
                        .help(format!(
                            "`{}`",
                            self.c.signature_text(&sig.name, &sig.ret, &sig.params)
                        ));
                        if sig.engine {
                            d = d.note("BioWare's compiler accepted vanilla scripts like this because it read declarations that did not ship; `--compat bioware` accepts them too");
                        }
                        self.error(d);
                        break;
                    }
                }
            }
        }
        Expr {
            kind: ExprKind::Call(Call {
                callee,
                args,
                name_span,
            }),
            ty: sig.ret,
            span,
        }
    }
}

/// True when a statement returns on every path, by the rule BioWare's
/// language uses: a `return`, a block with such a statement, or an `if`
/// whose branches both are. A loop or a `switch` never counts.
pub(super) fn stmt_returns(s: &Stmt) -> bool {
    match &s.kind {
        StmtKind::Return(_) => true,
        StmtKind::Block(b) => block_returns(b),
        StmtKind::If {
            then,
            otherwise: Some(o),
            ..
        } => stmt_returns(then) && stmt_returns(o),
        _ => false,
    }
}

fn block_returns(b: &Block) -> bool {
    b.stmts.iter().any(stmt_returns)
}

pub(super) fn is_error(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Error)
}

pub(super) fn error_expr(span: Span) -> Expr {
    Expr {
        kind: ExprKind::Error,
        ty: Type::Void,
        span,
    }
}

fn tok_text(t: Option<nwscript_syntax::SyntaxToken>) -> String {
    t.map(|t| t.text().to_string()).unwrap_or_default()
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// What to write to turn a `from` into a `to`, where the engine has a
/// routine for it.
fn conversion_help(from: &Type, to: &Type) -> String {
    let via = match (from, to) {
        (Type::Int, Type::Float) => Some("IntToFloat(x), or write the literal as `1.0`"),
        (Type::Float, Type::Int) => Some("FloatToInt(x)"),
        (Type::Int, Type::String) => Some("IntToString(x)"),
        (Type::Float, Type::String) => Some("FloatToString(x)"),
        (Type::String, Type::Int) => Some("StringToInt(x)"),
        (Type::String, Type::Float) => Some("StringToFloat(x)"),
        (Type::Object, Type::String) => Some("GetTag(x), or ObjectToString(x)"),
        _ => None,
    };
    match via {
        Some(v) => format!("NWScript converts no value implicitly; convert it with {v}"),
        None => "NWScript converts no value implicitly; pass a value of the declared type".into(),
    }
}

fn operator_help(op: BinaryOp) -> &'static str {
    use BinaryOp::*;
    match op {
        LogOr | LogAnd | BitOr | BitXor | BitAnd | Shl | Shr | UShr | Mod => {
            "this operator takes two `int`s"
        }
        Eq | Ne => "both sides of a comparison have the same type",
        Lt | Gt | Le | Ge => {
            "this comparison takes two `int`s or two `float`s; convert one side with IntToFloat or FloatToInt"
        }
        Add => {
            "`+` adds `int`s and `float`s in any mix, joins two `string`s, or adds two `vector`s; convert with IntToString and the like"
        }
        Sub => "`-` subtracts `int`s and `float`s in any mix, or two `vector`s",
        Mul => "`*` multiplies `int`s and `float`s in any mix, or a `vector` and a `float`",
        Div => "`/` divides `int`s and `float`s in any mix, or a `vector` by a `float`",
    }
}
