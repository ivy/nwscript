//! Semantic analysis: from a script's syntax tree, its includes and the
//! engine table to a checked [`Program`] and its diagnostics.
//!
//! Includes are expanded where their `#include` stands, each file once: an
//! include reached a second time is skipped, and one that reaches itself is
//! an error. Then two passes over the expanded top-level items. The first
//! declares structs, globals and function signatures; the second checks
//! global initialisers and function bodies. A name is visible after its
//! declaration in source order, so a use before it is an error that can
//! still say where the declaration is.

mod body;
pub(crate) mod ops;

use crate::hir::*;
use crate::table::EngineTable;
use crate::types::{ConstValue, Type};
use crate::visit;
use body::Body;
use nwscript_diag::{Diagnostic, FileId, Span, codes};
use nwscript_syntax::Parse;
use nwscript_syntax::ast::{self, AstNode, Item, TypeKind};
use std::collections::{HashMap, HashSet};

/// Where analysis finds the files a script includes. The library does no
/// I/O; the caller answers from directories, memory or an archive.
pub trait Includes {
    /// The include written `#include "name"`, parsed, with the name to show
    /// for it. `Ok(None)` when there is no such file; `Err` with a message
    /// when it exists and could not be read.
    fn include(&mut self, name: &str) -> Result<Option<(String, Parse)>, String>;
}

/// Includes from a fixed set of parsed files, matched ignoring case.
#[derive(Default)]
pub struct ParsedIncludes {
    files: HashMap<String, (String, Parse)>,
}

impl ParsedIncludes {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an include by its name without `.nss`.
    pub fn with(mut self, name: &str, parse: Parse) -> Self {
        self.files
            .insert(name.to_ascii_lowercase(), (name.to_string(), parse));
        self
    }
}

impl Includes for ParsedIncludes {
    fn include(&mut self, name: &str) -> Result<Option<(String, Parse)>, String> {
        Ok(self.files.get(&name.to_ascii_lowercase()).cloned())
    }
}

/// The outcome of analysis: the program, always built, and every
/// diagnostic, the files' syntax errors included.
#[derive(Clone, Debug)]
pub struct Analysis {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
}

impl Analysis {
    pub fn has_errors(&self) -> bool {
        nwscript_diag::has_errors(&self.diagnostics)
    }
}

/// Check a script. `name` is the script's file name, used to show it and to
/// catch an include of the script itself.
pub fn analyze(
    name: &str,
    main: Parse,
    table: &EngineTable,
    includes: &mut dyn Includes,
) -> Analysis {
    let mut c = Checker {
        table,
        diags: main.diagnostics().to_vec(),
        files: vec![SourceUnit {
            name: name.to_string(),
            parse: main,
            included_from: None,
        }],
        structs: Vec::new(),
        struct_names: HashMap::new(),
        globals: Vec::new(),
        global_names: HashMap::new(),
        functions: Vec::new(),
        function_names: HashMap::new(),
        call_sites: Vec::new(),
        global_callees: Vec::new(),
        recovered_reported: HashSet::new(),
    };
    let stem = resource_name(name);
    let mut items = Vec::new();
    let mut seen = HashSet::from([stem.clone()]);
    c.expand(0, includes, &mut vec![stem], &mut seen, &mut items);
    // Structs first, so a struct named above its declaration still resolves;
    // that use is E0327, which the compat modes accept as nwnnsscomp does.
    for (seq, (fi, item)) in items.iter().enumerate() {
        if matches!(item, Item::Struct(_)) {
            c.declare(*fi, seq as u32, item);
        }
    }
    for (seq, (fi, item)) in items.iter().enumerate() {
        if !matches!(item, Item::Struct(_)) {
            c.declare(*fi, seq as u32, item);
        }
    }
    for (seq, (fi, item)) in items.iter().enumerate() {
        c.check_item(*fi, seq as u32, item);
    }
    c.finish()
}

/// A file name as a resource name: no directory, no `.nss`, lower case.
fn resource_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let lower = base.to_ascii_lowercase();
    lower.strip_suffix(".nss").unwrap_or(&lower).to_string()
}

pub(crate) struct Checker<'t> {
    pub table: &'t EngineTable,
    pub diags: Vec<Diagnostic>,
    pub files: Vec<SourceUnit>,
    pub structs: Vec<StructDef>,
    /// Each struct's id and the position of its declaration.
    pub struct_names: HashMap<String, (StructId, u32)>,
    pub globals: Vec<Global>,
    pub global_names: HashMap<String, GlobalId>,
    pub functions: Vec<Function>,
    pub function_names: HashMap<String, FuncId>,
    /// Calls to user functions in each function's body, by function.
    call_sites: Vec<Vec<(FuncId, Span)>>,
    /// Calls to user functions in global initialisers.
    global_callees: Vec<(FuncId, Span)>,
    /// Engine routines with recovered signatures already reported.
    pub recovered_reported: HashSet<u16>,
}

impl Checker<'_> {
    fn file_id(&self, fi: usize) -> FileId {
        self.files[fi].parse.file()
    }

    fn span(&self, fi: usize, el: impl Into<nwscript_syntax::SyntaxElement>) -> Span {
        self.files[fi].parse.span_of(el)
    }

    /// Append `fi`'s top-level items to `items`, each include's items where
    /// its directive stands.
    fn expand(
        &mut self,
        fi: usize,
        includes: &mut dyn Includes,
        stack: &mut Vec<String>,
        seen: &mut HashSet<String>,
        items: &mut Vec<(usize, Item)>,
    ) {
        for item in self.files[fi].parse.tree().items() {
            let Item::Include(inc) = &item else {
                items.push((fi, item));
                continue;
            };
            let Some(name) = inc.path() else { continue };
            let span = self.span(fi, inc.syntax().clone());
            let key = name.to_ascii_lowercase();
            if let Some(pos) = stack.iter().position(|s| *s == key) {
                let chain: Vec<String> = stack[pos..]
                    .iter()
                    .chain(std::iter::once(&key))
                    .map(|s| format!("`{s}`"))
                    .collect();
                self.diags.push(
                    Diagnostic::new(&codes::E0320, format!("`{name}` includes itself"))
                        .primary(span, format!("the chain: {}", chain.join(" includes ")))
                        .help("remove one `#include` from the chain; a file's declarations need to reach the script only once"),
                );
                continue;
            }
            if !seen.insert(key.clone()) {
                continue;
            }
            match includes.include(&name) {
                Ok(Some((shown, parse))) => {
                    self.diags.extend(parse.diagnostics().iter().cloned());
                    self.files.push(SourceUnit {
                        name: shown,
                        parse,
                        included_from: Some((self.file_id(fi), span)),
                    });
                    let child = self.files.len() - 1;
                    stack.push(key);
                    self.expand(child, includes, stack, seen, items);
                    stack.pop();
                }
                Ok(None) => self.diags.push(
                    Diagnostic::new(&codes::E0319, format!("no include file named `{name}`"))
                        .primary(span, format!("looked for `{name}.nss`"))
                        .help("an include is searched for in the script's own directory, then each `-I` directory in order, ignoring case"),
                ),
                Err(e) => self.diags.push(
                    Diagnostic::new(&codes::E0319, format!("the include file `{name}` could not be read"))
                        .primary(span, e),
                ),
            }
        }
    }

    // ---- types -------------------------------------------------------------

    /// A type as written, or `None` (reported) when it names nothing.
    pub fn resolve_type(&mut self, fi: usize, ty: &ast::Type, seq: u32) -> Option<Type> {
        Some(match ty.kind()? {
            TypeKind::Void => Type::Void,
            TypeKind::Int => Type::Int,
            TypeKind::Float => Type::Float,
            TypeKind::String => Type::String,
            TypeKind::Object => Type::Object,
            TypeKind::Vector => Type::Vector,
            TypeKind::Action => Type::Action,
            TypeKind::Struct(tok) => {
                let tok = tok?;
                let name = tok.text().to_string();
                match self.struct_names.get(&name) {
                    Some(&(_, s)) if s < seq => Type::Struct(name),
                    Some(&(id, _)) => {
                        let span = self.span(fi, tok);
                        let decl = self.structs[id.0 as usize].span;
                        self.diags.push(
                            Diagnostic::new(&codes::E0327, format!("`struct {name}` is used above its declaration"))
                                .primary(span, "used here")
                                .secondary(decl, "declared here, later")
                                .help("move the struct's declaration above its first use; nwnnsscomp accepted this, and `--compat` does too"),
                        );
                        Type::Struct(name)
                    }
                    found => {
                        let span = self.span(fi, tok);
                        let mut d =
                            Diagnostic::new(&codes::E0301, format!("no struct named `{name}`"))
                                .primary(span, "not declared above this point");
                        if let Some(&(id, _)) = found {
                            d = d.secondary(
                                self.structs[id.0 as usize].span,
                                "declared here, later",
                            );
                        }
                        self.diags.push(d.help(
                            "declare `struct NAME { ... };` at the top level before its first use",
                        ));
                        return None;
                    }
                }
            }
            TypeKind::Named(tok) => match self.table.structure(tok.text()) {
                Some(t) => t,
                None => {
                    let span = self.span(fi, tok.clone());
                    let mut names: Vec<&str> =
                        vec!["void", "int", "float", "string", "object", "vector"];
                    names.extend(self.table.structures.iter().map(String::as_str));
                    let mut d =
                        Diagnostic::new(&codes::E0301, format!("no type named `{}`", tok.text()))
                            .primary(span, "not a type");
                    d = match suggest(tok.text(), names.into_iter()) {
                        Some(s) => d.help(format!("did you mean `{s}`? Type names are case-sensitive")),
                        None => d.help("types are `int`, `float`, `string`, `object`, `vector`, `struct NAME`, or an engine structure nwscript.nss defines, such as `effect` or `location`"),
                    };
                    self.diags.push(d);
                    return None;
                }
            },
        })
    }

    /// A type as the user writes it.
    pub fn show(&self, ty: &Type) -> String {
        match ty {
            Type::Engine(n) => self
                .table
                .structures
                .get(usize::from(*n))
                .cloned()
                .unwrap_or_else(|| ty.to_string()),
            t => t.to_string(),
        }
    }

    pub fn signature_text(
        &self,
        name: &str,
        ret: &Type,
        params: &[(String, Type, Option<ConstValue>)],
    ) -> String {
        let ps: Vec<String> = params
            .iter()
            .map(|(n, t, d)| match d {
                Some(d) => format!("{} {n} = {d}", self.show(t)),
                None => format!("{} {n}", self.show(t)),
            })
            .collect();
        format!("{} {name}({})", self.show(ret), ps.join(", "))
    }

    pub fn prototype_text(&self, id: FuncId) -> String {
        let f = &self.functions[id.0 as usize];
        let ps: Vec<(String, Type, Option<ConstValue>)> = f
            .params
            .iter()
            .map(|p| (p.name.clone(), p.ty.clone(), p.default.clone()))
            .collect();
        self.signature_text(&f.name, &f.ret, &ps)
    }

    // ---- pass 1: declarations ----------------------------------------------

    fn declare(&mut self, fi: usize, seq: u32, item: &Item) {
        match item {
            Item::Struct(s) => self.declare_struct(fi, seq, s),
            Item::Global(g) => self.declare_global(fi, seq, g),
            Item::Function(f) => self.declare_function(fi, seq, f),
            Item::Include(_) | Item::Define(_) | Item::Empty(_) => {}
        }
    }

    fn declare_struct(&mut self, fi: usize, seq: u32, s: &ast::StructDecl) {
        let Some(name) = s.name() else { return };
        let name = name.text().to_string();
        let span = self.span(fi, s.syntax().clone());
        let mut fields: Vec<Field> = Vec::new();
        let mut offset = 0;
        for fd in s.fields() {
            let ty = fd.ty().and_then(|t| self.resolve_type(fi, &t, seq));
            if let (Some(t), Some(at)) = (&ty, fd.ty())
                && matches!(t, Type::Void | Type::Action)
            {
                let at = self.span(fi, at.syntax().clone());
                self.diags.push(
                    Diagnostic::new(&codes::E0323, format!("a struct field of type `{t}`"))
                        .primary(at, "no value has this type"),
                );
            }
            let ty = ty.unwrap_or(Type::Int);
            for d in fd.declarators() {
                let Some(fname) = d.name() else { continue };
                let fspan = self.span(fi, d.syntax().clone());
                if let Some(init) = d.init() {
                    let at = self.span(fi, init.syntax().clone());
                    self.diags.push(
                        Diagnostic::new(&codes::E0325, "an initialiser on a struct field")
                            .primary(at, "fields have no initialisers")
                            .help("assign the field after declaring a variable of the struct type"),
                    );
                }
                if let Some(prev) = fields.iter().find(|f| f.name == fname.text()) {
                    let prev = prev.span;
                    self.diags.push(
                        Diagnostic::new(
                            &codes::E0313,
                            format!("`struct {name}` has two fields named `{}`", fname.text()),
                        )
                        .primary(fspan, "second field")
                        .secondary(prev, "first field"),
                    );
                    continue;
                }
                let size = self.size_of(&ty);
                fields.push(Field {
                    name: fname.text().to_string(),
                    ty: ty.clone(),
                    offset,
                    span: fspan,
                });
                offset += size;
            }
        }
        if let Some(&(prev, _)) = self.struct_names.get(&name) {
            let prev = self.structs[prev.0 as usize].span;
            self.diags.push(
                Diagnostic::new(&codes::E0313, format!("`struct {name}` is declared twice"))
                    .primary(span, "second declaration")
                    .secondary(prev, "first declaration"),
            );
            return;
        }
        let id = StructId(self.structs.len() as u32);
        self.struct_names.insert(name.clone(), (id, seq));
        self.structs.push(StructDef {
            name,
            fields,
            size: offset,
            span,
        });
    }

    fn size_of(&self, ty: &Type) -> u32 {
        match ty {
            Type::Struct(n) => self
                .struct_names
                .get(n)
                .map_or(0, |&(id, _)| self.structs[id.0 as usize].size),
            t => t.size().unwrap_or(0),
        }
    }

    fn declare_global(&mut self, fi: usize, seq: u32, g: &ast::GlobalDecl) {
        let ty = g.ty().and_then(|t| self.resolve_type(fi, &t, seq));
        if let (Some(t), Some(at)) = (&ty, g.ty())
            && matches!(t, Type::Void | Type::Action)
        {
            let at = self.span(fi, at.syntax().clone());
            self.diags.push(
                Diagnostic::new(&codes::E0323, format!("a variable of type `{t}`"))
                    .primary(at, "no value has this type"),
            );
        }
        for d in g.declarators() {
            let Some(name) = d.name() else { continue };
            let name = name.text().to_string();
            let span = self.span(fi, d.syntax().clone());
            if let Some(&prev) = self.global_names.get(&name) {
                let prev = self.globals[prev.0 as usize].span;
                self.diags.push(
                    Diagnostic::new(
                        &codes::E0313,
                        format!("the global `{name}` is declared twice"),
                    )
                    .primary(span, "second declaration")
                    .secondary(prev, "first declaration"),
                );
                continue;
            }
            if let Some(k) = self.table.constant(&name) {
                self.diags.push(
                    Diagnostic::new(
                        &codes::E0313,
                        format!("`{name}` is already a constant in nwscript.nss"),
                    )
                    .primary(span, "declared again here")
                    .secondary(k.span, "the constant")
                    .help("rename the global; the constant is visible everywhere"),
                );
                continue;
            }
            let id = GlobalId(self.globals.len() as u32);
            self.global_names.insert(name.clone(), id);
            self.globals.push(Global {
                name,
                ty: ty.clone().unwrap_or(Type::Int),
                init: None,
                span,
                seq,
                referenced: false,
            });
        }
    }

    /// The span of a function's header: from its return type to the `)`.
    fn header_span(&self, fi: usize, f: &ast::FunctionDecl) -> Span {
        let whole = self.span(fi, f.syntax().clone());
        let end = f
            .param_list()
            .map(|l| self.span(fi, l.syntax().clone()).end)
            .or_else(|| f.name().map(|n| self.span(fi, n).end))
            .unwrap_or(whole.end);
        let start = f
            .ret_type()
            .map(|t| self.span(fi, t.syntax().clone()).start)
            .unwrap_or(whole.start);
        Span::new(whole.file, start, end)
    }

    fn declare_function(&mut self, fi: usize, seq: u32, f: &ast::FunctionDecl) {
        let Some(name_tok) = f.name() else { return };
        let name = name_tok.text().to_string();
        let name_span = self.span(fi, name_tok);
        let header = self.header_span(fi, f);
        let is_def = f.body().is_some();
        let ret = f
            .ret_type()
            .and_then(|t| self.resolve_type(fi, &t, seq))
            .unwrap_or(Type::Void);
        if ret == Type::Action {
            self.diags.push(
                Diagnostic::new(&codes::E0323, "a function that returns `action`")
                    .primary(name_span, "")
                    .help("a function returns `void` or a value"),
            );
        }
        let params = self.params(fi, seq, f);

        if let Some((_, ef)) = self.table.function(&name) {
            self.diags.push(
                Diagnostic::new(
                    &codes::E0313,
                    format!("`{name}` is already an engine routine"),
                )
                .primary(name_span, "declared again here")
                .secondary(ef.span, "the routine in nwscript.nss")
                .help("give the script's function another name"),
            );
            return;
        }

        let Some(&id) = self.function_names.get(&name) else {
            let id = FuncId(self.functions.len() as u32);
            self.function_names.insert(name.clone(), id);
            self.functions.push(Function {
                name,
                ret,
                params,
                prototypes: if is_def { vec![] } else { vec![header] },
                definition: is_def.then_some(header),
                decl_seq: seq,
                def_seq: is_def.then_some(seq),
                file: self.file_id(fi),
                body: None,
                locals: Vec::new(),
                scopes: Vec::new(),
                reachable: false,
                callees: Vec::new(),
            });
            self.call_sites.push(Vec::new());
            return;
        };

        let prev = &self.functions[id.0 as usize];
        let prev_span = prev
            .definition
            .or(prev.prototypes.first().copied())
            .unwrap_or(header);
        let prev_is_def = prev.definition.is_some();
        if is_def && prev_is_def {
            self.diags.push(
                Diagnostic::new(&codes::E0313, format!("`{name}` is defined twice"))
                    .primary(name_span, "second definition")
                    .secondary(prev_span, "first definition")
                    .help("remove one definition; if both come from includes, include only one of them"),
            );
            return;
        }
        let against = if prev_is_def {
            prev_span
        } else {
            prev.prototypes[0]
        };
        if prev.ret != ret {
            let (a, b) = (self.show(&ret), self.show(&prev.ret));
            let d = if is_def || prev_is_def {
                let (def_ty, proto_ty) = if is_def { (&a, &b) } else { (&b, &a) };
                Diagnostic::new(&codes::E0311, format!("`{name}` is defined to return `{def_ty}` but its prototype says `{proto_ty}`"))
                    .primary(name_span, format!("returns `{a}` here"))
                    .secondary(against, format!("returns `{b}` here"))
                    .help("make the prototype and the definition agree")
                    .note("BioWare's compiler accepted this in k_inc_ebonhawk; `--compat bioware` accepts it too, with the definition's type")
            } else {
                Diagnostic::new(
                    &codes::E0312,
                    format!("two prototypes of `{name}` disagree on its return type"),
                )
                .primary(name_span, format!("returns `{a}` here"))
                .secondary(against, format!("returns `{b}` here"))
            };
            self.diags.push(d);
        }
        let prev_params = prev.params.clone();
        let same_list = prev_params.len() == params.len()
            && prev_params.iter().zip(&params).all(|(a, b)| a.ty == b.ty);
        if !same_list {
            let show = |ps: &[Param]| -> String {
                ps.iter()
                    .map(|p| self.show(&p.ty))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            self.diags.push(
                Diagnostic::new(&codes::E0312, format!("the declarations of `{name}` disagree on its parameters"))
                    .primary(header, format!("({}) here", show(&params)))
                    .secondary(against, format!("({}) here", show(&prev_params)))
                    .help("make every prototype and the definition take the same parameter types in the same order"),
            );
        }
        let mut merged = params.clone();
        if same_list {
            for (i, a) in prev_params.iter().enumerate() {
                // The first declaration's default wins, the one a call
                // after the definition gets too.
                if a.default.is_some() {
                    merged[i].default = a.default.clone();
                }
            }
        }
        let fid = self.file_id(fi);
        let f = &mut self.functions[id.0 as usize];
        if is_def {
            f.ret = ret;
            if same_list {
                f.params = merged;
            }
            f.definition = Some(header);
            f.def_seq = Some(seq);
            f.file = fid;
        } else {
            f.prototypes.push(header);
            if same_list && !prev_is_def {
                // Defaults a later prototype adds apply from here on.
                for (p, m) in f.params.iter_mut().zip(merged) {
                    if p.default.is_none() {
                        p.default = m.default;
                    }
                }
            }
        }
    }

    fn params(&mut self, fi: usize, seq: u32, f: &ast::FunctionDecl) -> Vec<Param> {
        let mut out: Vec<Param> = Vec::new();
        let mut defaulted: Option<Span> = None;
        for prm in f.params() {
            let span = self.span(fi, prm.syntax().clone());
            let ty = prm.ty().and_then(|t| self.resolve_type(fi, &t, seq));
            if let (Some(t), Some(at)) = (&ty, prm.ty())
                && matches!(t, Type::Void | Type::Action)
            {
                let at = self.span(fi, at.syntax().clone());
                let help = if *t == Type::Action {
                    "only engine routines such as `DelayCommand` take an `action`; take the values the action needs instead, and make the call inside"
                } else {
                    "a parameter holds a value"
                };
                self.diags.push(
                    Diagnostic::new(&codes::E0323, format!("a parameter of type `{t}`"))
                        .primary(at, "")
                        .help(help),
                );
            }
            let ty = ty.unwrap_or(Type::Int);
            let default = prm.default_value().and_then(|e| {
                let mut b = Body::new(self, fi, seq, Vec::new(), Type::Void);
                let x = b.expr(&e);
                if body::is_error(&x) {
                    return None;
                }
                let tshow = self.show(&ty);
                match ops::fold(&x) {
                    Some(v) if v.ty() == ty => Some(v),
                    Some(v) => {
                        let vt = self.show(&v.ty());
                        self.diags.push(
                            Diagnostic::new(&codes::E0305, format!("the default of `{}` is a `{vt}` where a `{tshow}` is expected", prm.name().map(|n| n.text().to_string()).unwrap_or_default()))
                                .primary(x.span, format!("this is a `{vt}`"))
                                .help(if ty == Type::Float && v.ty() == Type::Int { "write the default as a float literal, such as `1.0`" } else { "give a default of the parameter's type" }),
                        );
                        None
                    }
                    None => {
                        self.diags.push(
                            Diagnostic::new(&codes::E0302, "a parameter's default that is not a constant")
                                .primary(x.span, "not a constant")
                                .help("a default is a literal, a negated literal, `OBJECT_SELF`, `OBJECT_INVALID`, a constant from nwscript.nss, or a vector of float literals; a script's own globals are variables"),
                        );
                        None
                    }
                }
            });
            if prm.default_value().is_some() {
                defaulted.get_or_insert(span);
            } else if let Some(first) = defaulted {
                self.diags.push(
                    Diagnostic::new(&codes::E0324, "a parameter without a default after one with a default")
                        .primary(span, "needs a default")
                        .secondary(first, "the first parameter with a default")
                        .help("a call can omit only trailing arguments, so every parameter after a defaulted one needs a default too; move this one earlier"),
                );
            }
            out.push(Param {
                name: prm.name().map(|n| n.text().to_string()).unwrap_or_default(),
                ty,
                default,
                span,
            });
        }
        out
    }

    // ---- pass 2: initialisers and bodies -----------------------------------

    fn check_item(&mut self, fi: usize, seq: u32, item: &Item) {
        match item {
            Item::Global(g) => {
                let ty = g.ty();
                for d in g.declarators() {
                    let Some(init) = d.init() else { continue };
                    let span = self.span(fi, d.syntax().clone());
                    let id = d
                        .name()
                        .and_then(|n| self.global_names.get(n.text()).copied())
                        .filter(|id| self.globals[id.0 as usize].span == span);
                    let mut b = Body::new(self, fi, seq, Vec::new(), Type::Void);
                    let x = b.expr(&init);
                    let calls = std::mem::take(&mut b.call_sites);
                    let Some(id) = id else { continue };
                    let gty = self.globals[id.0 as usize].ty.clone();
                    if !body::is_error(&x) && x.ty != gty && ty.is_some() {
                        let (xt, gt) = (self.show(&x.ty), self.show(&gty));
                        self.diags.push(
                            Diagnostic::new(&codes::E0305, format!("the initialiser of `{}` is a `{xt}` where a `{gt}` is expected", self.globals[id.0 as usize].name))
                                .primary(x.span, format!("this is a `{xt}`"))
                                .help("NWScript converts no value implicitly; initialise with a value of the declared type"),
                        );
                    }
                    self.global_callees.extend(calls);
                    self.globals[id.0 as usize].init = Some(x);
                }
            }
            Item::Function(f) => {
                let (Some(name), Some(body)) = (f.name(), f.body()) else {
                    return;
                };
                let Some(&id) = self.function_names.get(name.text()) else {
                    return;
                };
                if self.functions[id.0 as usize].def_seq != Some(seq) {
                    return;
                }
                let name_span = self.span(fi, name.clone());
                let func = &self.functions[id.0 as usize];
                let (params, ret) = (func.params.clone(), func.ret.clone());
                let mut b = Body::new(self, fi, seq, params, ret);
                let block = b.function_body(&body, name_span, name.text());
                let (locals, scopes, callees, sites) = (
                    std::mem::take(&mut b.locals),
                    std::mem::take(&mut b.scopes),
                    std::mem::take(&mut b.callees),
                    std::mem::take(&mut b.call_sites),
                );
                let f = &mut self.functions[id.0 as usize];
                f.body = Some(block);
                f.locals = locals;
                f.scopes = scopes;
                f.callees = callees;
                self.call_sites[id.0 as usize] = sites;
            }
            Item::Struct(_) | Item::Include(_) | Item::Define(_) | Item::Empty(_) => {}
        }
    }

    // ---- the whole program ---------------------------------------------------

    fn entry(&mut self) -> Option<Entry> {
        let mut found = Vec::new();
        for (kind, want) in [
            (EntryKind::Main, Type::Void),
            (EntryKind::StartingConditional, Type::Int),
        ] {
            let Some(&id) = self.function_names.get(kind.name()) else {
                continue;
            };
            let f = &self.functions[id.0 as usize];
            let Some(def) = f.definition else {
                let at = f.prototypes[0];
                self.diags.push(
                    Diagnostic::new(
                        &codes::E0318,
                        format!("`{}` is declared but never defined", kind.name()),
                    )
                    .primary(at, "")
                    .help("the entry point needs a body"),
                );
                continue;
            };
            if f.ret != want || !f.params.is_empty() {
                let shown = self.prototype_text(id);
                self.diags.push(
                    Diagnostic::new(&codes::E0318, format!("the entry point is `{shown}`"))
                        .primary(def, format!("must be `{} {}()`", want, kind.name()))
                        .help("a script's entry point is `void main()`, which the engine runs, or `int StartingConditional()`, which a dialogue runs to decide whether a line is available; neither takes parameters"),
                );
            }
            found.push((Entry { kind, function: id }, def));
        }
        if let [(_, a), (_, b)] = found[..] {
            self.diags.push(
                Diagnostic::new(
                    &codes::E0318,
                    "a script with both `main` and `StartingConditional`",
                )
                .primary(b, "")
                .secondary(a, "")
                .help("a script has one entry point: split it into two scripts"),
            );
        }
        found.first().map(|(e, _)| *e)
    }

    fn finish(mut self) -> Analysis {
        let entry = self.entry();
        if let Some(e) = entry {
            let mut queue: Vec<FuncId> = vec![e.function];
            queue.extend(self.global_callees.iter().map(|(f, _)| *f));
            while let Some(id) = queue.pop() {
                let f = &mut self.functions[id.0 as usize];
                if f.reachable {
                    continue;
                }
                f.reachable = true;
                queue.extend(f.callees.iter().copied());
            }
            // A reachable call to a function without a body, reported once
            // per function at its first call.
            let mut sites: Vec<(FuncId, Span)> = self.global_callees.clone();
            for (i, s) in self.call_sites.iter().enumerate() {
                if self.functions[i].reachable {
                    sites.extend(s.iter().copied());
                }
            }
            let mut reported = HashSet::new();
            for (id, at) in sites {
                let f = &self.functions[id.0 as usize];
                if f.definition.is_none() && reported.insert(id) {
                    let proto = f.prototypes[0];
                    self.diags.push(
                        Diagnostic::new(
                            &codes::E0322,
                            format!("`{}` is called but never defined", f.name),
                        )
                        .primary(at, "called here")
                        .secondary(proto, "declared here, with no body anywhere")
                        .help("define the function, or include the file that defines it"),
                    );
                }
            }
            self.mark_referenced_globals();
        }
        Analysis {
            program: Program {
                files: self.files,
                structs: self.structs,
                globals: self.globals,
                functions: self.functions,
                entry,
            },
            diagnostics: self.diags,
        }
    }

    fn mark_referenced_globals(&mut self) {
        let mut found: Vec<GlobalId> = Vec::new();
        let mut collect = |e: &Expr| {
            if let ExprKind::Var(Var::Global(g)) = e.kind {
                found.push(g);
            }
        };
        for f in self.functions.iter().filter(|f| f.reachable) {
            if let Some(b) = &f.body {
                visit::block_exprs(b, &mut collect);
            }
        }
        // Globals whose initialisers call script functions run code the
        // entry point depends on; their initialisers count as reachable too.
        for g in &self.globals {
            if let Some(init) = &g.init {
                let mut calls = false;
                visit::expr_exprs(init, &mut |e| {
                    calls |=
                        matches!(&e.kind, ExprKind::Call(c) if matches!(c.callee, Callee::User(_)));
                });
                if calls {
                    visit::expr_exprs(init, &mut collect);
                }
            }
        }
        while let Some(g) = found.pop() {
            let global = &mut self.globals[g.0 as usize];
            if global.referenced {
                continue;
            }
            global.referenced = true;
            if let Some(init) = &global.init {
                visit::expr_exprs(init, &mut |e| {
                    if let ExprKind::Var(Var::Global(g)) = e.kind {
                        found.push(g);
                    }
                });
            }
        }
    }
}

/// The candidate closest to `name`: one that differs only in case, or the
/// nearest by edit distance within a third of the name's length.
pub(crate) fn suggest<'a>(name: &str, cands: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let max = (name.len() / 3).clamp(1, 3);
    let mut best: Option<(usize, &str)> = None;
    for c in cands {
        if c == name {
            continue;
        }
        if c.eq_ignore_ascii_case(name) {
            return Some(c);
        }
        if c.len().abs_diff(name.len()) > max {
            continue;
        }
        let d = levenshtein(name.as_bytes(), c.as_bytes());
        if d <= max && best.is_none_or(|(bd, bc)| d < bd || (d == bd && c < bc)) {
            best = Some((d, c));
        }
    }
    best.map(|(_, c)| c)
}

/// Edit distance counting an adjacent transposition as one edit, ignoring
/// ASCII case.
fn levenshtein(a: &[u8], b: &[u8]) -> usize {
    let eq = |x: u8, y: u8| x.eq_ignore_ascii_case(&y);
    let w = b.len() + 1;
    let mut d = vec![0usize; (a.len() + 1) * w];
    for i in 0..=a.len() {
        for j in 0..=b.len() {
            d[i * w + j] = if i == 0 || j == 0 {
                i + j
            } else {
                let cost = usize::from(!eq(a[i - 1], b[j - 1]));
                let mut v = (d[(i - 1) * w + j - 1] + cost)
                    .min(d[(i - 1) * w + j] + 1)
                    .min(d[i * w + j - 1] + 1);
                if i > 1 && j > 1 && eq(a[i - 1], b[j - 2]) && eq(a[i - 2], b[j - 1]) {
                    v = v.min(d[(i - 2) * w + j - 2] + 1);
                }
                v
            };
        }
    }
    d[a.len() * w + b.len()]
}
