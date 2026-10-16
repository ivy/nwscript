//! The census recogniser: a recursive-descent parser for the grammar in
//! `docs/grammar.md`, which counts each production it takes. It builds only
//! the expression shapes the counts need; it is not the compiler's parser.

use super::Census;
use super::lex::{Kind, Token};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct Param {
    pub ty: String,
    /// The form of the default value, if the parameter has one.
    pub default: Option<&'static str>,
}

/// Engine functions from `nwscript.nss`: name to parameters.
pub type Engine = BTreeMap<String, Vec<Param>>;

const TYPES: &[&str] = &[
    "void",
    "int",
    "float",
    "string",
    "object",
    "vector",
    "action",
    "effect",
    "event",
    "location",
    "talent",
    "itemproperty",
];

/// Binary operator levels, loosest first.
const LEVELS: &[(&str, &[&str])] = &[
    ("logor", &["||"]),
    ("logand", &["&&"]),
    ("bitor", &["|"]),
    ("bitxor", &["^"]),
    ("bitand", &["&"]),
    ("equality", &["==", "!="]),
    ("relational", &["<", ">", "<=", ">="]),
    ("shift", &["<<", ">>", ">>>"]),
    ("additive", &["+", "-"]),
    ("multiplicative", &["*", "/", "%"]),
];

const ASSIGN_OPS: &[&str] = &[
    "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", ">>>=",
];

fn op_name(op: &str) -> &'static str {
    match op {
        "||" => "logor",
        "&&" => "logand",
        "|" => "bitor",
        "^" => "bitxor",
        "&" => "bitand",
        "==" => "eq",
        "!=" => "ne",
        "<" => "lt",
        ">" => "gt",
        "<=" => "le",
        ">=" => "ge",
        "<<" => "shl",
        ">>" => "shr",
        ">>>" => "ushr",
        "+" => "add",
        "-" => "sub",
        "*" => "mul",
        "/" => "div",
        "%" => "mod",
        "!" => "not",
        "~" => "bitnot",
        "++" => "inc",
        "--" => "dec",
        "=" => "assign",
        "+=" => "add_assign",
        "-=" => "sub_assign",
        "*=" => "mul_assign",
        "/=" => "div_assign",
        "%=" => "mod_assign",
        "&=" => "bitand_assign",
        "|=" => "bitor_assign",
        "^=" => "bitxor_assign",
        "<<=" => "shl_assign",
        ">>=" => "shr_assign",
        ">>>=" => "ushr_assign",
        _ => "unknown",
    }
}

#[derive(Clone, Debug)]
enum E {
    Int,
    Float,
    Str,
    Ident,
    ObjSelf,
    ObjInvalid,
    Vector,
    Paren(Box<E>),
    Unary(String, Box<E>),
    Postfix,
    Binary(usize, String),
    Assign(String),
    Cond,
    Call(String),
    Member,
}

/// What a statement was, for block, switch and if analysis.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum S {
    Decl,
    Break,
    Return,
    Continue,
    Block { ends_in_jump: bool },
    Label { default: bool },
    If { has_else: bool },
    Other,
}

impl S {
    fn jumps(self) -> bool {
        matches!(
            self,
            S::Break | S::Return | S::Continue | S::Block { ends_in_jump: true }
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Brk {
    Loop,
    Switch,
}

pub struct Parser<'a> {
    t: Vec<Token>,
    p: usize,
    file: &'a str,
    c: &'a mut Census,
    engine: &'a Engine,
    /// User functions of every file, for calls that omit defaulted arguments.
    user: &'a Engine,
    /// Prototypes found in this file, for building the engine table.
    pub protos: Engine,
    /// Every function this file declares or defines, first declaration wins.
    pub functions: Engine,
    proto_ret: BTreeMap<String, String>,
    brk: Vec<Brk>,
    /// Block depth of each open switch body.
    switch_depth: Vec<usize>,
    depth: usize,
    seen_decl: bool,
    seen_function_def: bool,
    declared: BTreeSet<String>,
    defined: BTreeMap<String, String>,
    /// User calls to names not yet declared in this file.
    early_calls: BTreeSet<String>,
    current_fn: String,
}

type R<T> = Option<T>;

impl<'a> Parser<'a> {
    pub fn new(
        t: Vec<Token>,
        file: &'a str,
        c: &'a mut Census,
        engine: &'a Engine,
        user: &'a Engine,
    ) -> Self {
        Parser {
            t,
            p: 0,
            file,
            c,
            engine,
            user,
            protos: Engine::new(),
            functions: Engine::new(),
            proto_ret: BTreeMap::new(),
            brk: Vec::new(),
            switch_depth: Vec::new(),
            depth: 0,
            seen_decl: false,
            seen_function_def: false,
            declared: BTreeSet::new(),
            defined: BTreeMap::new(),
            early_calls: BTreeSet::new(),
            current_fn: String::new(),
        }
    }

    fn hit(&mut self, key: &str) {
        let line = self.t[self.p.saturating_sub(1)].line;
        self.c.hit_at(key, self.file, line);
    }

    fn peek(&self) -> &Token {
        &self.t[self.p.min(self.t.len() - 1)]
    }

    fn peek_at(&self, k: usize) -> &Token {
        &self.t[(self.p + k).min(self.t.len() - 1)]
    }

    fn is(&self, s: &str) -> bool {
        let t = self.peek();
        matches!(t.kind, Kind::Punct | Kind::Ident) && t.text == s
    }

    fn eat(&mut self, s: &str) -> bool {
        if self.is(s) {
            self.p += 1;
            true
        } else {
            false
        }
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if t.kind != Kind::Eof {
            self.p += 1;
        }
        t
    }

    fn fail<T>(&mut self, what: &str) -> R<T> {
        let t = self.peek().clone();
        self.c.error(
            self.file,
            t.line,
            &format!("expected {what}, found `{}`", t.text),
        );
        None
    }

    fn expect(&mut self, s: &str) -> R<()> {
        if self.eat(s) {
            Some(())
        } else {
            self.fail(&format!("`{s}`"))
        }
    }

    fn ident(&mut self) -> R<String> {
        if self.peek().kind == Kind::Ident
            && !super::lex::KEYWORDS.contains(&self.peek().text.as_str())
        {
            Some(self.bump().text)
        } else {
            self.fail("an identifier")
        }
    }

    fn at_type(&self) -> bool {
        let t = self.peek();
        t.kind == Kind::Ident && (TYPES.contains(&t.text.as_str()) || t.text == "struct")
    }

    /// `type`; returns its census name (`struct` for any struct type).
    fn ty(&mut self) -> R<String> {
        if self.eat("struct") {
            self.ident()?;
            return Some("struct".into());
        }
        if self.at_type() {
            return Some(self.bump().text);
        }
        self.fail("a type")
    }

    pub fn unit(&mut self) {
        while self.peek().kind != Kind::Eof {
            if self.peek().kind == Kind::Directive {
                let d = self.bump();
                self.directive(&d.text);
                continue;
            }
            if self.eat(";") {
                self.hit("top.empty_declaration");
                continue;
            }
            if self.top().is_none() {
                self.recover();
            }
        }
        self.finish();
    }

    fn finish(&mut self) {
        let main = self.defined.get("main").cloned();
        let sc = self.defined.get("StartingConditional").cloned();
        match (&main, &sc) {
            (Some(r), None) => self.hit(&format!("entry.main.returns_{r}")),
            (None, Some(r)) => self.hit(&format!("entry.starting_conditional.returns_{r}")),
            (Some(_), Some(_)) => self.hit("entry.both"),
            (None, None) => self.hit("entry.none"),
        }
        let early: Vec<String> = self
            .early_calls
            .iter()
            .filter(|n| self.declared.contains(*n))
            .cloned()
            .collect();
        for _ in early {
            self.hit("sem.call_before_declaration");
        }
    }

    fn recover(&mut self) {
        let mut depth = 0i32;
        loop {
            let t = self.bump();
            if t.kind == Kind::Eof {
                return;
            }
            match t.text.as_str() {
                "{" => depth += 1,
                "}" => {
                    depth -= 1;
                    if depth <= 0 {
                        return;
                    }
                }
                ";" if depth == 0 => return,
                _ => {}
            }
        }
    }

    fn directive(&mut self, line: &str) {
        let body = line[1..].trim_start();
        let name: String = body
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if line[1..].starts_with([' ', '\t']) {
            self.hit("pp.space_after_hash");
        }
        let rest = body[name.len()..].trim();
        match name.as_str() {
            "include" => {
                self.hit("pp.include");
                if self.seen_decl {
                    self.hit("pp.include.after_declaration");
                }
                let Some(q) = rest.strip_prefix('"') else {
                    self.hit("pp.include.not_quoted");
                    return;
                };
                let Some(end) = q.find('"') else {
                    self.hit("pp.include.unterminated");
                    return;
                };
                let target = &q[..end];
                let after = q[end + 1..].trim();
                if !after.is_empty() {
                    self.hit(if after.starts_with("//") || after.starts_with("/*") {
                        "pp.include.trailing_comment"
                    } else {
                        "pp.include.trailing_text"
                    });
                }
                if target.contains('.') {
                    self.hit("pp.include.with_extension");
                }
                if target != target.to_ascii_lowercase() {
                    self.hit("pp.include.not_lowercase");
                }
                self.c.include(self.file, target);
            }
            "define" => {
                self.hit("pp.define");
                let mut parts = rest.split_whitespace();
                let _name = parts.next();
                match parts.next() {
                    Some(v) if v.bytes().all(|b| b.is_ascii_digit()) => {
                        self.hit("pp.define.value_int")
                    }
                    Some(_) => self.hit("pp.define.value_word"),
                    None => self.hit("pp.define.no_value"),
                }
            }
            _ => self.hit("pp.other"),
        }
    }

    fn top(&mut self) -> R<()> {
        self.seen_decl = true;
        if self.eat("const") {
            self.hit("decl.const");
        }
        if self.is("struct") && self.peek_at(2).text == "{" {
            return self.struct_def();
        }
        let ty = self.ty()?;
        let name = self.ident()?;
        if self.eat("(") {
            return self.function(ty, name);
        }
        self.hit("top.global");
        self.hit(&format!("top.global.type.{ty}"));
        if self.seen_function_def {
            self.hit("top.global.after_function_definition");
        }
        let n = self.declarators(true)?;
        if n > 1 {
            self.hit("top.global.multi_declarator");
        }
        self.expect(";")
    }

    /// The rest of a declarator list after the first name; returns how many.
    fn declarators(&mut self, global: bool) -> R<usize> {
        let mut n = 0;
        loop {
            n += 1;
            let k = if global { "top.global" } else { "stmt.decl" };
            self.hit(&format!("{k}.declarator"));
            if self.eat("=") {
                let e = self.expr()?;
                if global {
                    let class = classify(&e);
                    self.hit(&format!("top.global.init.{class}"));
                } else {
                    self.hit("stmt.decl.init");
                }
            } else if global {
                self.hit("top.global.uninitialized");
            }
            if !self.eat(",") {
                return Some(n);
            }
            self.ident()?;
        }
    }

    fn struct_def(&mut self) -> R<()> {
        self.expect("struct")?;
        self.ident()?;
        self.expect("{")?;
        self.hit("top.struct");
        while !self.eat("}") {
            let ty = self.ty()?;
            self.hit(&format!("struct.member.type.{ty}"));
            self.ident()?;
            self.hit("struct.member");
            while self.eat(",") {
                self.hit("struct.member.multi_declarator");
                self.ident()?;
                self.hit("struct.member");
            }
            self.expect(";")?;
        }
        self.expect(";")
    }

    fn function(&mut self, ret: String, name: String) -> R<()> {
        let mut params = Vec::new();
        if self.eat(")") {
            self.hit("param.list.empty");
        } else if self.is("void") && self.peek_at(1).text == ")" {
            self.bump();
            self.bump();
            self.hit("param.list.void");
        } else {
            loop {
                params.push(self.param()?);
                if !self.eat(",") {
                    break;
                }
            }
            self.expect(")")?;
        }
        let mut seen_default = false;
        for p in &params {
            if p.default.is_some() {
                seen_default = true;
            } else if seen_default {
                self.c.hit("param.required_after_default", self.file);
            }
        }
        let defaults = params.iter().filter(|p| p.default.is_some()).count();
        self.functions
            .entry(name.clone())
            .or_insert_with(|| params.clone());
        self.hit(&format!("function.return.{ret}"));
        self.declared.insert(name.clone());
        if self.eat(";") {
            self.hit("top.function.prototype");
            if self.protos.contains_key(&name) {
                self.hit("top.function.prototype_repeated");
            }
            if defaults > 0 {
                self.hit("param.default.in_prototype");
            }
            if self.defined.contains_key(&name) {
                self.hit("top.function.prototype_after_definition");
            }
            self.proto_ret.insert(name.clone(), ret);
            self.protos.insert(name, params);
            return Some(());
        }
        self.hit("top.function.definition");
        if defaults > 0 {
            self.hit("param.default.in_definition");
        }
        if let Some(proto) = self.protos.get(&name).cloned() {
            self.hit("top.function.definition_after_prototype");
            let types = |ps: &[Param]| ps.iter().map(|p| p.ty.clone()).collect::<Vec<_>>();
            if types(&proto) != types(&params) || self.proto_ret.get(&name) != Some(&ret) {
                self.hit("top.function.definition_differs_from_prototype");
            }
            let pd = proto.iter().any(|p| p.default.is_some());
            let dd = defaults > 0;
            match (pd, dd) {
                (true, true) => self.hit("param.default.in_prototype_and_definition"),
                (true, false) => self.hit("param.default.in_prototype_only"),
                (false, true) => self.hit("param.default.in_definition_only"),
                (false, false) => {}
            }
        }
        if self.defined.contains_key(&name) {
            self.hit("top.function.redefinition");
        }
        if (name == "main" || name == "StartingConditional") && !params.is_empty() {
            self.hit("entry.has_params");
        }
        self.defined.insert(name.clone(), ret);
        self.seen_function_def = true;
        self.current_fn = name;
        self.depth = 0;
        let r = self.block(true);
        self.current_fn.clear();
        r.map(|_| ())
    }

    fn param(&mut self) -> R<Param> {
        let ty = self.ty()?;
        self.hit("param");
        self.hit(&format!("param.type.{ty}"));
        if self.is(",") || self.is(")") {
            self.hit("param.unnamed");
        } else {
            self.ident()?;
        }
        let mut default = None;
        if self.eat("=") {
            let e = self.expr()?;
            self.hit("param.default");
            let class = classify(&e);
            self.hit(&format!("param.default.{class}"));
            default = Some(class);
        }
        Some(Param { ty, default })
    }

    /// `{ ... }`; returns whether its last statement jumps.
    fn block(&mut self, body: bool) -> R<S> {
        self.expect("{")?;
        if !body {
            self.hit("stmt.block");
        }
        self.depth += 1;
        let mut last = S::Other;
        let mut seen_stmt = false;
        let mut empty = true;
        while !self.eat("}") {
            if self.peek().kind == Kind::Eof {
                return self.fail("`}`");
            }
            empty = false;
            let s = self.stmt()?;
            if s == S::Decl && seen_stmt {
                self.hit("stmt.decl.after_statement");
            }
            if s != S::Decl {
                seen_stmt = true;
            }
            last = s;
        }
        if empty {
            self.hit(if body {
                "function.body.empty"
            } else {
                "stmt.block.empty"
            });
        }
        self.depth -= 1;
        Some(S::Block {
            ends_in_jump: last.jumps(),
        })
    }

    fn stmt(&mut self) -> R<S> {
        let t = self.peek().clone();
        if t.kind == Kind::Directive {
            self.bump();
            self.hit("pp.inside_function");
            self.directive(&t.text);
            return Some(S::Other);
        }
        if t.kind == Kind::Ident {
            match t.text.as_str() {
                "if" => return self.if_stmt(),
                "while" => {
                    self.bump();
                    self.hit("stmt.while");
                    self.paren_cond()?;
                    self.loop_body("stmt.while.body_unbraced")?;
                    return Some(S::Other);
                }
                "do" => {
                    self.bump();
                    self.hit("stmt.do_while");
                    self.loop_body("stmt.do_while.body_unbraced")?;
                    self.expect("while")?;
                    self.paren_cond()?;
                    self.expect(";")?;
                    return Some(S::Other);
                }
                "for" => return self.for_stmt(),
                "switch" => return self.switch_stmt(),
                "case" | "default" => return self.label(),
                "break" => {
                    self.bump();
                    match self.brk.last() {
                        Some(Brk::Switch) => self.hit("stmt.break.in_switch"),
                        Some(Brk::Loop) => self.hit("stmt.break.in_loop"),
                        None => self.hit("stmt.break.outside"),
                    }
                    self.expect(";")?;
                    return Some(S::Break);
                }
                "continue" => {
                    self.bump();
                    self.hit("stmt.continue");
                    if !self.brk.contains(&Brk::Loop) {
                        self.hit("stmt.continue.outside_loop");
                    }
                    self.expect(";")?;
                    return Some(S::Continue);
                }
                "return" => {
                    self.bump();
                    if self.eat(";") {
                        self.hit("stmt.return.void");
                    } else {
                        let e = self.expr()?;
                        self.hit("stmt.return.value");
                        if matches!(e, E::Paren(_)) {
                            self.hit("stmt.return.value_parenthesized");
                        }
                        self.expect(";")?;
                    }
                    return Some(S::Return);
                }
                "const" => {
                    self.bump();
                    self.hit("decl.const");
                    return self.local_decl();
                }
                "struct" if self.peek_at(2).text == "{" => {
                    self.hit("stmt.struct_definition");
                    self.struct_def()?;
                    return Some(S::Decl);
                }
                _ if self.at_type() => return self.local_decl(),
                _ => {}
            }
        }
        if self.is("{") {
            return self.block(false);
        }
        if self.eat(";") {
            self.hit("stmt.empty");
            return Some(S::Other);
        }
        let e = self.expr()?;
        self.hit("stmt.expr");
        let k = match &e {
            E::Call(_) => "call",
            E::Assign(op) if op == "=" => "assign",
            E::Assign(_) => "compound_assign",
            E::Postfix => "postfix_incdec",
            E::Unary(op, _) if op == "++" || op == "--" => "prefix_incdec",
            _ => "no_effect",
        };
        self.hit(&format!("stmt.expr.{k}"));
        self.expect(";")?;
        Some(S::Other)
    }

    fn local_decl(&mut self) -> R<S> {
        let ty = self.ty()?;
        self.ident()?;
        self.hit("stmt.decl");
        self.hit(&format!("stmt.decl.type.{ty}"));
        if self.switch_depth.last() == Some(&self.depth) {
            self.hit("stmt.decl.in_case_unbraced");
        }
        let n = self.declarators(false)?;
        if n > 1 {
            self.hit("stmt.decl.multi_declarator");
        }
        self.expect(";")?;
        Some(S::Decl)
    }

    fn paren_cond(&mut self) -> R<()> {
        self.expect("(")?;
        self.expr()?;
        self.expect(")")
    }

    fn loop_body(&mut self, unbraced: &str) -> R<S> {
        if !self.is("{") {
            self.hit(unbraced);
        }
        self.brk.push(Brk::Loop);
        let s = self.stmt();
        self.brk.pop();
        s
    }

    fn if_stmt(&mut self) -> R<S> {
        self.expect("if")?;
        self.hit("stmt.if");
        self.paren_cond()?;
        let then_braced = self.is("{");
        if !then_braced {
            self.hit("stmt.if.then_unbraced");
        }
        let then = self.stmt()?;
        if !then_braced && then == (S::If { has_else: true }) {
            self.hit("stmt.if.dangling_else");
        }
        if self.eat("else") {
            self.hit("stmt.if.else");
            if self.is("if") {
                self.hit("stmt.if.else_if");
            } else if !self.is("{") {
                self.hit("stmt.if.else_unbraced");
            }
            self.stmt()?;
            return Some(S::If { has_else: true });
        }
        Some(S::If { has_else: false })
    }

    fn for_stmt(&mut self) -> R<S> {
        self.expect("for")?;
        self.hit("stmt.for");
        self.expect("(")?;
        let mut parts = Vec::new();
        if !self.is(";") {
            parts.push("init");
            if self.at_type() {
                self.hit("stmt.for.init_declaration");
                self.ty()?;
                self.ident()?;
                self.declarators(false)?;
            } else {
                self.expr_list()?;
            }
        }
        self.expect(";")?;
        if !self.is(";") {
            parts.push("cond");
            self.expr()?;
        }
        self.expect(";")?;
        if !self.is(")") {
            parts.push("step");
            self.expr_list()?;
        }
        self.expect(")")?;
        let name = if parts.is_empty() {
            "none".to_string()
        } else {
            parts.join("_")
        };
        self.hit(&format!("stmt.for.parts.{name}"));
        self.loop_body("stmt.for.body_unbraced")?;
        Some(S::Other)
    }

    fn expr_list(&mut self) -> R<()> {
        self.expr()?;
        while self.eat(",") {
            self.hit("stmt.for.comma_list");
            self.expr()?;
        }
        Some(())
    }

    fn switch_stmt(&mut self) -> R<S> {
        self.expect("switch")?;
        self.hit("stmt.switch");
        if !self.brk.is_empty() && self.brk.contains(&Brk::Switch) {
            self.hit("stmt.switch.nested");
        }
        self.paren_cond()?;
        self.expect("{")?;
        self.depth += 1;
        self.brk.push(Brk::Switch);
        self.switch_depth.push(self.depth);
        let mut items = Vec::new();
        while !self.eat("}") {
            if self.peek().kind == Kind::Eof {
                return self.fail("`}`");
            }
            items.push(self.stmt()?);
        }
        self.switch_depth.pop();
        self.brk.pop();
        self.depth -= 1;
        self.switch_groups(&items);
        Some(S::Other)
    }

    fn switch_groups(&mut self, items: &[S]) {
        if items.is_empty() {
            self.hit("stmt.switch.empty");
            return;
        }
        if !matches!(items[0], S::Label { .. }) {
            self.hit("stmt.switch.statement_before_first_label");
        }
        // Split into groups: a run of labels, then the statements up to the next label.
        let mut groups: Vec<(Vec<bool>, Vec<S>)> = Vec::new();
        for &s in items {
            match s {
                S::Label { default } => {
                    if groups.last().is_none_or(|g| !g.1.is_empty()) {
                        groups.push((Vec::new(), Vec::new()));
                    }
                    if let Some(g) = groups.last_mut() {
                        g.0.push(default);
                    }
                }
                _ => {
                    if groups.is_empty() {
                        groups.push((Vec::new(), Vec::new()));
                    }
                    if let Some(g) = groups.last_mut() {
                        g.1.push(s);
                    }
                }
            }
        }
        let last = groups.len() - 1;
        for (i, (labels, body)) in groups.iter().enumerate() {
            if labels.len() > 1 {
                self.hit("stmt.switch.stacked_labels");
            }
            if labels.contains(&true) && i != last {
                self.hit("stmt.switch.default_not_last");
            }
            if body.is_empty() {
                if i == last {
                    self.hit("stmt.switch.trailing_label_without_statement");
                }
                continue;
            }
            if body.len() == 1 && matches!(body[0], S::Block { .. }) {
                self.hit("stmt.switch.group_braced");
                if body[0].jumps() {
                    self.hit("stmt.switch.jump_inside_braces");
                }
            }
            let jumps = body.last().is_some_and(|s| s.jumps());
            if !jumps {
                self.hit(if i == last {
                    "stmt.switch.last_group_falls_out"
                } else {
                    "stmt.switch.fallthrough"
                });
            }
        }
    }

    fn label(&mut self) -> R<S> {
        let kw = self.bump().text;
        if self.switch_depth.last() != Some(&self.depth) {
            self.hit("stmt.switch.label_nested");
        }
        if kw == "default" {
            self.hit("stmt.switch.default");
            self.expect(":")?;
            return Some(S::Label { default: true });
        }
        let e = self.expr()?;
        self.hit("stmt.switch.case");
        let class = classify(&e);
        self.hit(&format!("stmt.switch.case.{class}"));
        self.expect(":")?;
        Some(S::Label { default: false })
    }

    // Expressions.

    fn expr(&mut self) -> R<E> {
        let lhs = self.cond()?;
        let t = self.peek().clone();
        if t.kind == Kind::Punct && ASSIGN_OPS.contains(&t.text.as_str()) {
            self.bump();
            let rhs = self.expr()?;
            self.hit(&format!("expr.assign.{}", op_name(&t.text)));
            if !matches!(lhs, E::Ident | E::Member) {
                self.hit("expr.assign.target_not_lvalue");
            }
            if matches!(lhs, E::Member) {
                self.hit("expr.assign.to_member");
            }
            if matches!(rhs, E::Assign(_)) {
                self.hit("expr.assign.chained");
            }
            return Some(E::Assign(t.text));
        }
        Some(lhs)
    }

    fn cond(&mut self) -> R<E> {
        let c = self.binary(0)?;
        if self.eat("?") {
            self.hit("expr.conditional");
            self.expr()?;
            self.expect(":")?;
            self.cond()?;
            return Some(E::Cond);
        }
        Some(c)
    }

    fn binary(&mut self, level: usize) -> R<E> {
        if level == LEVELS.len() {
            return self.unary();
        }
        let mut lhs = self.binary(level + 1)?;
        loop {
            let t = self.peek().clone();
            if t.kind != Kind::Punct || !LEVELS[level].1.contains(&t.text.as_str()) {
                return Some(lhs);
            }
            self.bump();
            let rhs = self.binary(level + 1)?;
            self.hit(&format!("expr.binary.{}", op_name(&t.text)));
            if t.text == "+" && (matches!(lhs, E::Str) || matches!(rhs, E::Str)) {
                self.hit("expr.binary.add.string_literal_operand");
            }
            for (side, child) in [("left", &lhs), ("right", &rhs)] {
                match child {
                    E::Binary(l, _) if *l != level => {
                        let key = format!("prec.{}_over_{}", LEVELS[level].0, LEVELS[*l].0);
                        self.hit(&key);
                    }
                    E::Binary(_, inner_op) if side == "left" => {
                        self.hit(&format!("assoc.left.{}", LEVELS[level].0));
                        let order_matters =
                            |o: &str| !matches!(o, "+" | "*" | "&&" | "||" | "&" | "|" | "^");
                        if order_matters(&t.text) || order_matters(inner_op) {
                            self.hit(&format!("assoc.left.{}.order_matters", LEVELS[level].0));
                        }
                    }
                    E::Unary(op, _) if op == "!" && (level == 5 || level == 6) => {
                        self.hit("prec.not_operand_of_comparison");
                    }
                    E::Assign(_) => self.hit("expr.assign.operand_unparenthesized"),
                    _ => {}
                }
            }
            for side in [&lhs, &rhs] {
                if let E::Paren(inner) = side
                    && matches!(**inner, E::Assign(_))
                {
                    self.hit("expr.assign.as_operand");
                }
            }
            lhs = E::Binary(level, t.text);
        }
    }

    fn unary(&mut self) -> R<E> {
        let t = self.peek().clone();
        if t.kind == Kind::Punct && matches!(t.text.as_str(), "-" | "!" | "~" | "+" | "++" | "--") {
            self.bump();
            let e = self.unary()?;
            let name = match t.text.as_str() {
                "-" => "neg",
                "+" => "plus",
                "++" => "preinc",
                "--" => "predec",
                o => op_name(o),
            };
            self.hit(&format!("expr.unary.{name}"));
            if t.text == "-" {
                match e {
                    E::Int => self.hit("expr.unary.neg.int_literal"),
                    E::Float => self.hit("expr.unary.neg.float_literal"),
                    _ => self.hit("expr.unary.neg.other"),
                }
            }
            return Some(E::Unary(t.text, Box::new(e)));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> R<E> {
        let mut e = self.primary()?;
        loop {
            if self.is("(") {
                let E::Ident = e else {
                    return self.fail("a function name before `(`");
                };
                // The callee's name is the token before `(`.
                let name = self.t[self.p - 1].text.clone();
                self.bump();
                let mut args = Vec::new();
                if !self.eat(")") {
                    loop {
                        args.push(self.expr()?);
                        if !self.eat(",") {
                            break;
                        }
                    }
                    self.expect(")")?;
                }
                self.call(&name, &args);
                e = E::Call(name);
            } else if self.eat(".") {
                let field = self.ident()?;
                self.hit("expr.member");
                if matches!(e, E::Member) {
                    self.hit("expr.member.chained");
                }
                if matches!(e, E::Call(_)) {
                    self.hit("expr.member.of_call");
                }
                if matches!(field.as_str(), "x" | "y" | "z") {
                    self.hit("expr.member.xyz");
                } else {
                    self.hit("expr.member.named_field");
                }
                e = E::Member;
            } else if self.is("++") || self.is("--") {
                let op = self.bump().text;
                self.hit(if op == "++" {
                    "expr.postfix.inc"
                } else {
                    "expr.postfix.dec"
                });
                if !matches!(e, E::Ident | E::Member) {
                    self.hit("expr.postfix.target_not_lvalue");
                }
                e = E::Postfix;
            } else {
                return Some(e);
            }
        }
    }

    fn call(&mut self, name: &str, args: &[E]) {
        self.hit("expr.call");
        if args.is_empty() {
            self.hit("expr.call.no_arguments");
        }
        if name == self.current_fn {
            self.hit("sem.recursive_call");
        }
        let (params, k) = if let Some(p) = self.engine.get(name) {
            (p.clone(), "expr.call.engine")
        } else {
            if !self.declared.contains(name) {
                self.early_calls.insert(name.to_string());
            }
            match self.user.get(name) {
                Some(p) => (p.clone(), "expr.call.user"),
                None => {
                    self.hit("expr.call.user");
                    self.hit("expr.call.user.undeclared");
                    return;
                }
            }
        };
        self.hit(k);
        let required = params.iter().filter(|p| p.default.is_none()).count();
        if args.len() < params.len() && args.len() >= required {
            self.hit(&format!("{k}.omits_defaults"));
            for p in &params[args.len()..] {
                if let Some(class) = p.default {
                    self.hit(&format!("{k}.default_used.{class}"));
                }
            }
        }
        if args.len() > params.len() || args.len() < required {
            self.hit(&format!("{k}.arity_mismatch"));
        }
        for (p, a) in params.iter().zip(args) {
            if p.ty != "action" {
                continue;
            }
            self.hit("action_arg");
            match a {
                E::Call(n) if self.engine.contains_key(n) => self.hit("action_arg.engine_call"),
                E::Call(_) => self.hit("action_arg.user_call"),
                _ => self.hit("action_arg.not_a_call"),
            }
        }
    }

    fn primary(&mut self) -> R<E> {
        let t = self.peek().clone();
        match t.kind {
            Kind::Int => {
                self.bump();
                Some(E::Int)
            }
            Kind::Float => {
                self.bump();
                Some(E::Float)
            }
            Kind::Str => {
                self.bump();
                if self.peek().kind == Kind::Str {
                    self.hit("expr.string.adjacent");
                }
                Some(E::Str)
            }
            Kind::Ident if t.text == "OBJECT_SELF" => {
                self.bump();
                self.hit("expr.object_self");
                Some(E::ObjSelf)
            }
            Kind::Ident if t.text == "OBJECT_INVALID" => {
                self.bump();
                self.hit("expr.object_invalid");
                Some(E::ObjInvalid)
            }
            Kind::Ident if !super::lex::KEYWORDS.contains(&t.text.as_str()) => {
                self.bump();
                Some(E::Ident)
            }
            Kind::Punct if t.text == "(" => {
                self.bump();
                let e = self.expr()?;
                self.expect(")")?;
                self.hit("expr.paren");
                Some(E::Paren(Box::new(e)))
            }
            Kind::Punct if t.text == "[" => {
                self.bump();
                self.hit("expr.vector_literal");
                let mut n = 0;
                if !self.is("]") {
                    loop {
                        let e = self.expr()?;
                        n += 1;
                        let class = classify(&e);
                        self.hit(&format!("expr.vector_literal.element.{class}"));
                        if !self.eat(",") {
                            break;
                        }
                    }
                }
                self.expect("]")?;
                self.hit(&format!("expr.vector_literal.elements_{n}"));
                Some(E::Vector)
            }
            _ => self.fail("an expression"),
        }
    }
}

/// The form of a constant-like expression: defaults, initialisers, case labels.
fn classify(e: &E) -> &'static str {
    match e {
        E::Int => "int_literal",
        E::Float => "float_literal",
        E::Str => "string_literal",
        E::Ident => "identifier",
        E::ObjSelf => "object_self",
        E::ObjInvalid => "object_invalid",
        E::Vector => "vector_literal",
        E::Unary(op, inner) if op == "-" => match **inner {
            E::Int => "negative_int_literal",
            E::Float => "negative_float_literal",
            _ => "expression",
        },
        E::Call(_) => "call",
        _ => "expression",
    }
}
