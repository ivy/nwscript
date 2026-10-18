//! Source text from the recovered program.
//!
//! The text is built as a `String` in which every byte of a string literal
//! above 0x7F is the `char` of the same number; [`to_bytes`] turns it back
//! into the Windows-1252 bytes NWScript source is.

use crate::ast::*;
use nwscript_sema::{ConstValue, EngineTable};
use std::fmt::Write;

/// What the printer needs besides the program.
pub struct Context<'a> {
    pub table: &'a EngineTable,
    /// The comment written above each function, by function index.
    pub notes: &'a [String],
    /// A header comment for the whole file.
    pub header: &'a str,
    /// The libraries whose functions replaced recovered ones, as
    /// `#include` lines after the header.
    pub includes: &'a [String],
}

/// The text and the lines each function occupies (first, last; 1-based).
pub struct Printed {
    pub text: String,
    pub lines: Vec<(u32, u32)>,
}

/// The source bytes of printed text.
pub fn to_bytes(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect()
}

pub fn print(s: &Script, cx: &Context) -> Printed {
    let mut p = Printer {
        s,
        cx,
        out: String::new(),
    };
    p.script()
}

struct Printer<'a> {
    s: &'a Script,
    cx: &'a Context<'a>,
    out: String,
}

const INDENT: &str = "    ";

impl Printer<'_> {
    fn line(&self) -> u32 {
        self.out.matches('\n').count() as u32 + 1
    }

    fn script(&mut self) -> Printed {
        let mut lines = vec![(0, 0); self.s.funcs.len()];
        for l in self.cx.header.lines() {
            let _ = writeln!(self.out, "// {l}");
        }
        if !self.cx.header.is_empty() {
            self.out.push('\n');
        }
        for inc in self.cx.includes {
            let _ = writeln!(self.out, "#include \"{inc}\"");
        }
        if !self.cx.includes.is_empty() {
            self.out.push('\n');
        }
        for st in self.s.structs.iter().filter(|st| !st.folded) {
            let _ = writeln!(self.out, "struct {} {{", st.name);
            for (i, f) in st.fields.iter().enumerate() {
                let _ = writeln!(self.out, "{INDENT}{} {};", self.ty(Some(*f)), st.field(i));
            }
            self.out.push_str("};\n\n");
        }
        // Globals first: functions refer to them.
        for (i, f) in self.s.funcs.iter().enumerate() {
            if f.kind != FuncKind::Globals {
                continue;
            }
            let silent = self.cx.notes.get(i).is_none_or(|n| n.is_empty());
            if silent && f.body.as_ref().is_ok_and(|b| b.is_empty()) {
                continue;
            }
            let first = self.line();
            self.note(i);
            match &f.body {
                Ok(body) => {
                    let mut k = 0;
                    while k < body.len() {
                        k = self.stmt_at(body, k, 0);
                    }
                }
                Err(_) => self.out.push_str("// (the globals are not shown)\n"),
            }
            lines[i] = (first, self.line().saturating_sub(1).max(first));
            self.out.push('\n');
        }
        let users: Vec<usize> = (0..self.s.funcs.len())
            .filter(|&i| self.s.funcs[i].kind == FuncKind::User && !self.s.funcs[i].folded)
            .collect();
        for &i in &users {
            let sig = self.signature(i);
            let _ = writeln!(self.out, "{sig};");
        }
        if !users.is_empty() {
            self.out.push('\n');
        }
        for (i, f) in self.s.funcs.iter().enumerate() {
            if f.kind == FuncKind::Globals || f.folded {
                continue;
            }
            let first = self.line();
            self.note(i);
            let sig = self.signature(i);
            let _ = writeln!(self.out, "{sig}\n{{");
            match &f.body {
                Ok(body) => self.stmts(body, 1),
                Err(_) => {
                    if let Some(t) = f.ret {
                        let _ = writeln!(
                            self.out,
                            "{INDENT}{} placeholder;\n{INDENT}return placeholder;",
                            self.ty(Some(t))
                        );
                    }
                }
            }
            self.out.push_str("}\n");
            lines[i] = (first, self.line() - 1);
            self.out.push('\n');
        }
        Printed {
            text: std::mem::take(&mut self.out),
            lines,
        }
    }

    fn note(&mut self, i: usize) {
        if let Some(n) = self.cx.notes.get(i) {
            for l in n.lines() {
                let _ = writeln!(self.out, "// {l}");
            }
        }
    }

    fn signature(&self, i: usize) -> String {
        let f = &self.s.funcs[i];
        let ret = match f.ret {
            None => "void".to_string(),
            Some(t) => self.ty(Some(t)),
        };
        let params: Vec<String> = f
            .params
            .iter()
            .enumerate()
            .map(|(k, &v)| {
                let mut p = format!("{} {}", self.ty(self.s.vars[v].ty), self.s.vars[v].name);
                if let Some(Some(d)) = f.defaults.get(k) {
                    p.push_str(" = ");
                    p.push_str(&self.default_value(d));
                }
                p
            })
            .collect();
        format!("{ret} {}({})", f.name, params.join(", "))
    }

    /// A parameter default: a literal, negative ones included.
    fn default_value(&self, e: &Expr) -> String {
        match e {
            Expr::Int(i) => i.to_string(),
            Expr::Float(f) => float_text(*f),
            Expr::Vector([x, y, z]) => {
                format!(
                    "[{}, {}, {}]",
                    float_text(*x),
                    float_text(*y),
                    float_text(*z)
                )
            }
            other => self.expr(other, 0),
        }
    }

    fn ty(&self, t: Option<Ty>) -> String {
        type_text(self.s, self.cx.table, t)
    }

    fn place(&self, p: &Place) -> String {
        let Place::Var(v) = p;
        let (v, field) = self.s.root(*v);
        let var = &self.s.vars[v];
        match field {
            None => var.name.clone(),
            Some(n) => match var.ty {
                Some(Ty::Vector) => format!("{}.{}", var.name, ["x", "y", "z"][n.min(2)]),
                Some(Ty::Struct(i)) => format!("{}.{}", var.name, self.s.structs[i].field(n)),
                _ => format!("{}.f{n}", var.name),
            },
        }
    }

    fn stmts(&mut self, body: &[Stmt], depth: usize) {
        let mut k = 0;
        while k < body.len() {
            k = self.stmt_at(body, k, depth);
        }
    }

    /// Print `body[k]` and return the index of the next statement to
    /// print: a declaration takes the assignment after it as its initialiser.
    fn stmt_at(&mut self, body: &[Stmt], k: usize, depth: usize) -> usize {
        let ind = INDENT.repeat(depth);
        if let Stmt::Decl(v) = &body[k] {
            let (root, part) = self.s.root(*v);
            if part.is_some_and(|n| n > 0) {
                return k + 1;
            }
            let ty = self.ty(self.s.vars[root].ty);
            let name = self.s.vars[root].name.clone();
            // The next statement that prints something, if it assigns this variable.
            let mut j = k + 1;
            while let Some(Stmt::Decl(w)) = body.get(j) {
                if self.s.root(*w).1.is_some_and(|n| n > 0) && self.s.root(*w).0 == root {
                    j += 1;
                } else {
                    break;
                }
            }
            if let Some(Stmt::Expr(Expr::Assign(p, e))) = body.get(j)
                && self.place_root(p) == Some(root)
                && !mentions(self.s, e, root)
            {
                let e = self.expr(e, 1);
                let _ = writeln!(self.out, "{ind}{ty} {name} = {e};");
                return j + 1;
            }
            let _ = writeln!(self.out, "{ind}{ty} {name};");
            return k + 1;
        }
        self.stmt(&body[k], depth);
        k + 1
    }

    /// The whole variable a place writes, when it writes all of one.
    fn place_root(&self, p: &Place) -> Option<VarId> {
        let Place::Var(v) = p;
        match self.s.root(*v) {
            (r, None) => Some(r),
            _ => None,
        }
    }

    fn block(&mut self, body: &[Stmt], depth: usize) {
        let ind = INDENT.repeat(depth);
        self.out.push_str("{\n");
        self.stmts(body, depth + 1);
        let _ = write!(self.out, "{ind}}}");
    }

    fn stmt(&mut self, st: &Stmt, depth: usize) {
        let ind = INDENT.repeat(depth);
        match st {
            Stmt::Decl(_) => unreachable!("declarations print in stmt_at"),
            Stmt::Expr(e) => {
                let e = self.expr(e, 0);
                let _ = writeln!(self.out, "{ind}{e};");
            }
            Stmt::If(c, t, e) => {
                let _ = write!(self.out, "{ind}");
                self.if_chain(c, t, e.as_deref(), depth);
                self.out.push('\n');
            }
            Stmt::While(c, b) => {
                let c = self.expr(c, 0);
                let _ = write!(self.out, "{ind}while ({c}) ");
                self.block(b, depth);
                self.out.push('\n');
            }
            Stmt::For(i, c, st, b) => {
                let (i, c, st) = (self.expr(i, 0), self.expr(c, 0), self.expr(st, 0));
                let _ = write!(self.out, "{ind}for ({i}; {c}; {st}) ");
                self.block(b, depth);
                self.out.push('\n');
            }
            Stmt::DoWhile(b, c) => {
                let _ = write!(self.out, "{ind}do ");
                self.block(b, depth);
                let c = self.expr(c, 0);
                let _ = writeln!(self.out, " while ({c});");
            }
            Stmt::Switch(c, b) => {
                let c = self.expr(c, 0);
                let _ = writeln!(self.out, "{ind}switch ({c}) {{");
                let inner = INDENT.repeat(depth + 1);
                let mut k = 0;
                while k < b.len() {
                    match &b[k] {
                        Stmt::Case(v) => {
                            let _ = writeln!(self.out, "{inner}case {}:", int_text(*v));
                            k += 1;
                        }
                        Stmt::Default => {
                            let _ = writeln!(self.out, "{inner}default:");
                            k += 1;
                        }
                        _ => k = self.stmt_at(b, k, depth + 2),
                    }
                }
                let _ = writeln!(self.out, "{ind}}}");
            }
            Stmt::Case(_) | Stmt::Default => {}
            Stmt::Block(b) => {
                let _ = write!(self.out, "{ind}");
                self.block(b, depth);
                self.out.push('\n');
            }
            Stmt::Break => {
                let _ = writeln!(self.out, "{ind}break;");
            }
            Stmt::Continue => {
                let _ = writeln!(self.out, "{ind}continue;");
            }
            Stmt::Return(None) => {
                let _ = writeln!(self.out, "{ind}return;");
            }
            Stmt::Return(Some(e)) => {
                let e = self.expr(e, 0);
                let _ = writeln!(self.out, "{ind}return {e};");
            }
        }
    }

    fn if_chain(&mut self, c: &Expr, t: &[Stmt], e: Option<&[Stmt]>, depth: usize) {
        let c = self.expr(c, 0);
        let _ = write!(self.out, "if ({c}) ");
        self.block(t, depth);
        match e {
            None => {}
            Some([Stmt::If(c2, t2, e2)]) => {
                self.out.push_str(" else ");
                self.if_chain(c2, t2, e2.as_deref(), depth);
            }
            Some(e) => {
                self.out.push_str(" else ");
                self.block(e, depth);
            }
        }
    }

    /// An expression in a context binding at `outer` (0: none).
    fn expr(&self, e: &Expr, outer: u8) -> String {
        let (text, prec) = self.expr_prec(e);
        if prec < outer {
            format!("({text})")
        } else {
            text
        }
    }

    fn args(&self, args: &[Expr]) -> String {
        args.iter()
            .map(|a| self.expr(a, 0))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn expr_prec(&self, e: &Expr) -> (String, u8) {
        const PRIMARY: u8 = 15;
        const UNARY: u8 = 13;
        match e {
            Expr::Int(i) => (int_text(*i), PRIMARY),
            Expr::Float(f) => (float_text(*f), PRIMARY),
            Expr::Str(s) => (string_text(s), PRIMARY),
            Expr::Object(0) => ("OBJECT_SELF".into(), PRIMARY),
            Expr::Object(_) => ("OBJECT_INVALID".into(), PRIMARY),
            Expr::Vector([x, y, z]) => (
                format!(
                    "[{}, {}, {}]",
                    float_text(*x),
                    float_text(*y),
                    float_text(*z)
                ),
                PRIMARY,
            ),
            Expr::Place(p) => (self.place(p), PRIMARY),
            Expr::Engine { routine, args } => {
                let f = &self.cx.table.functions[usize::from(*routine)];
                // Trailing arguments equal to their defaults are left out.
                let mut n = args.len();
                while n > 0 {
                    match f.params.get(n - 1).and_then(|p| p.default.as_ref()) {
                        Some(d) if same_const(d, &args[n - 1]) => n -= 1,
                        _ => break,
                    }
                }
                (format!("{}({})", f.name, self.args(&args[..n])), PRIMARY)
            }
            Expr::User { func, args } => {
                let f = &self.s.funcs[*func];
                let mut n = args.len();
                while n > 0 {
                    match f.defaults.get(n - 1) {
                        Some(Some(d)) if d == &args[n - 1] => n -= 1,
                        _ => break,
                    }
                }
                (format!("{}({})", f.name, self.args(&args[..n])), PRIMARY)
            }
            Expr::Member(x, n) => (
                format!("{}.{}", self.expr(x, 14), ["x", "y", "z"][(*n).min(2)]),
                14,
            ),
            Expr::Unary(op, x) => {
                let sym = match op {
                    UnOp::Neg => "-",
                    UnOp::Not => "!",
                    UnOp::Comp => "~",
                };
                let mut inner = self.expr(x, UNARY);
                // `- -x` would lex as a decrement.
                if (*op == UnOp::Neg && inner.starts_with('-')) || inner.starts_with(sym) {
                    inner = format!("({inner})");
                }
                (format!("{sym}{inner}"), UNARY)
            }
            Expr::Binary(op, l, r) => {
                let p = op.prec();
                let side = |x: &Expr, right: bool| -> String {
                    let (t, cp) = self.expr_prec(x);
                    let paren = cp < p
                        || (right && cp == p)
                        || (cp > p && is_binary(x) && !confirmed(p, cp));
                    if paren { format!("({t})") } else { t }
                };
                (
                    format!("{} {} {}", side(l, false), op.text(), side(r, true)),
                    p,
                )
            }
            Expr::Assign(pl, x) => (format!("{} = {}", self.place(pl), self.expr(x, 1)), 1),
            Expr::IncDec { place, inc, post } => {
                let op = if *inc { "++" } else { "--" };
                let name = self.place(place);
                if *post {
                    (format!("{name}{op}"), 14)
                } else {
                    (format!("{op}{name}"), UNARY)
                }
            }
        }
    }
}

/// A type as the source spells it; an unknown type prints as `int`, the
/// type an unused slot defaults to.
pub(crate) fn type_text(s: &Script, table: &EngineTable, t: Option<Ty>) -> String {
    match t {
        None | Some(Ty::Int) => "int".into(),
        Some(Ty::Float) => "float".into(),
        Some(Ty::String) => "string".into(),
        Some(Ty::Object) => "object".into(),
        Some(Ty::Vector) => "vector".into(),
        Some(Ty::Engine(n)) => table
            .structures
            .get(usize::from(n))
            .cloned()
            .unwrap_or_else(|| format!("engine{n}")),
        Some(Ty::Struct(i)) => format!("struct {}", s.structs[i].name),
    }
}

fn is_binary(e: &Expr) -> bool {
    matches!(e, Expr::Binary(..))
}

/// Whether an operator binding at `inner` may stand unparenthesised as an
/// operand of one binding at `outer`: the relations BioWare's scripts
/// exercise (docs/grammar.md, "Precedence and associativity"). Elsewhere
/// the text keeps parentheses, so it does not lean on an unconfirmed rule.
fn confirmed(outer: u8, inner: u8) -> bool {
    match outer {
        3 | 4 => matches!(inner, 8 | 9 | 11 | 12),
        8 | 9 => matches!(inner, 11 | 12),
        11 => inner == 12,
        _ => false,
    }
}

/// Whether an expression reads or writes variable `v` (or a part of it).
fn mentions(s: &Script, e: &Expr, v: VarId) -> bool {
    let place = |p: &Place| {
        let Place::Var(w) = p;
        *w == v || s.root(*w).0 == v
    };
    match e {
        Expr::Place(p) => place(p),
        Expr::Engine { args, .. } | Expr::User { args, .. } => {
            args.iter().any(|a| mentions(s, a, v))
        }
        Expr::Unary(_, x) | Expr::Member(x, _) => mentions(s, x, v),
        Expr::Binary(_, l, r) => mentions(s, l, v) || mentions(s, r, v),
        Expr::Assign(p, x) => place(p) || mentions(s, x, v),
        Expr::IncDec { place: p, .. } => place(p),
        _ => false,
    }
}

/// Whether an argument is the constant a default supplies.
fn same_const(d: &ConstValue, e: &Expr) -> bool {
    match (d, e) {
        (ConstValue::Int(a), Expr::Int(b)) => a == b,
        (ConstValue::Float(a), Expr::Float(b)) => a.to_bits() == b.to_bits(),
        (ConstValue::String(a), Expr::Str(b)) => a == b,
        (ConstValue::Object(a), Expr::Object(b)) => a == b,
        (ConstValue::Vector(a), Expr::Vector(b)) => {
            a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
        }
        _ => false,
    }
}

/// An integer literal. A negative value has no literal form (`-1` is
/// negation applied to `1`), so it is written as its two's-complement hex.
pub fn int_text(i: i32) -> String {
    if i < 0 {
        format!("0x{:08x}", i as u32)
    } else {
        i.to_string()
    }
}

/// A float literal that reads back as the same `f32`: Rust's shortest
/// round-trip digits, always with a decimal point.
pub fn float_text(f: f32) -> String {
    let mut s = format!("{f}");
    if !s.contains('.') && !s.contains("inf") && !s.contains("NaN") {
        s.push_str(".0");
    }
    s
}

fn string_text(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &c in b {
        match c {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            b'\n' => s.push_str("\\n"),
            c => s.push(char::from(c)),
        }
    }
    s.push('"');
    s
}

/// Whether a float has a literal form: finite and not negative.
pub fn float_writable(f: f32) -> bool {
    f.is_finite() && !f.is_sign_negative()
}
