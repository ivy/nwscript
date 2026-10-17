//! The engine's function and constant table, read from `nwscript.nss` with
//! the same parser as any script.
//!
//! An engine routine's number, the operand of `ACTION`, is its prototype's
//! position among the file's prototypes, from zero. A prototype whose types
//! do not resolve keeps its slot: the table records a recovered signature,
//! marked as such, and reports the types it could not resolve.

use crate::types::{ConstValue, Type};
use nwscript_diag::{Diagnostic, FileId, Span, codes};
use nwscript_syntax::ast::{self, AstNode, Expr, LiteralValue, PrefixOp, TypeKind};
use nwscript_syntax::{Parse, parse};
use std::collections::HashMap;

/// One parameter of an engine routine.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub default: Option<ConstValue>,
}

/// An engine routine.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub name: String,
    pub ret: Type,
    pub params: Vec<Param>,
    /// True when a type in the prototype did not resolve and the signature
    /// is the table's best reading of it.
    pub recovered: bool,
    /// The prototype's span in `nwscript.nss`.
    pub span: Span,
    /// The comment block above the prototype, without comment markers.
    pub doc: String,
}

/// A constant `nwscript.nss` declares.
#[derive(Clone, Debug, PartialEq)]
pub struct Constant {
    pub name: String,
    pub value: ConstValue,
    pub span: Span,
}

/// The engine's routines, constants and structure types.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EngineTable {
    /// Engine structure type names, by number.
    pub structures: Vec<String>,
    /// Routines by `ACTION` number.
    pub functions: Vec<Function>,
    pub constants: Vec<Constant>,
    by_name: HashMap<String, usize>,
    constant_by_name: HashMap<String, usize>,
}

impl EngineTable {
    /// The routine with this name and its number.
    pub fn function(&self, name: &str) -> Option<(u16, &Function)> {
        self.by_name
            .get(name)
            .map(|&i| (i as u16, &self.functions[i]))
    }

    pub fn constant(&self, name: &str) -> Option<&Constant> {
        self.constant_by_name.get(name).map(|&i| &self.constants[i])
    }

    /// The type an engine structure's name stands for.
    pub fn structure(&self, name: &str) -> Option<Type> {
        self.structures
            .iter()
            .position(|s| s == name)
            .map(|n| Type::Engine(n as u8))
    }

    /// Read `nwscript.nss`. The table is always returned; the diagnostics
    /// say what in the file could not be read and how it was recovered.
    pub fn from_nwscript(file: FileId, bytes: &[u8]) -> (EngineTable, Vec<Diagnostic>) {
        let p = parse(file, bytes);
        let mut diags: Vec<Diagnostic> = p.diagnostics().to_vec();
        let tree = p.tree();
        let mut t = EngineTable::default();
        let mut structures: Vec<(u32, String)> = tree
            .defines()
            .filter_map(|d| d.engine_structure())
            .collect();
        structures.sort();
        t.structures = structures.into_iter().map(|(_, n)| n).collect();

        for g in tree.globals() {
            let ty = g
                .ty()
                .and_then(|ty| t.resolve_type(&p, &ty, &mut Vec::new()));
            for d in g.declarators() {
                let Some(name) = d.name() else { continue };
                let span = p.span_of(d.syntax().clone());
                match d.init().map(|e| t.const_value(&p, &e)) {
                    Some(Ok(value)) => {
                        if let Some(ty) = &ty
                            && *ty != value.ty()
                            && !(*ty == Type::Float && value.ty() == Type::Int)
                        {
                            diags.push(
                                Diagnostic::new(
                                    &codes::E0302,
                                    format!(
                                        "constant `{}` is declared {ty} but initialised with {}",
                                        name.text(),
                                        value.ty()
                                    ),
                                )
                                .primary(span, ""),
                            );
                        }
                        let value = match (&ty, value) {
                            (Some(Type::Float), ConstValue::Int(i)) => ConstValue::Float(i as f32),
                            (_, v) => v,
                        };
                        t.constant_by_name
                            .insert(name.text().to_string(), t.constants.len());
                        t.constants.push(Constant {
                            name: name.text().to_string(),
                            value,
                            span,
                        });
                    }
                    Some(Err(d)) => diags.push(d),
                    None => diags.push(
                        Diagnostic::new(
                            &codes::E0302,
                            format!("constant `{}` has no value", name.text()),
                        )
                        .primary(span, "nwscript.nss declares constants with an initialiser"),
                    ),
                }
            }
        }

        for f in tree.functions() {
            if let Some(func) = t.function_decl(&p, &f, &mut diags) {
                t.by_name
                    .entry(func.name.clone())
                    .or_insert(t.functions.len());
                t.functions.push(func);
            }
        }
        (t, diags)
    }

    /// One prototype of `nwscript.nss`, or `None` without a name.
    fn function_decl(
        &self,
        p: &Parse,
        f: &ast::FunctionDecl,
        diags: &mut Vec<Diagnostic>,
    ) -> Option<Function> {
        let name = f.name()?;
        let span = p.span_of(f.syntax().clone());
        let mut unresolved: Vec<(Span, String, Type)> = Vec::new();
        let ret = f
            .ret_type()
            .and_then(|ty| self.resolve_type(p, &ty, &mut unresolved))
            .unwrap_or(Type::Void);
        let mut params = Vec::new();
        for prm in f.params() {
            let ty = prm
                .ty()
                .and_then(|ty| self.resolve_type(p, &ty, &mut unresolved))
                .unwrap_or(Type::Int);
            let default = match prm.default_value().map(|e| self.const_value(p, &e)) {
                Some(Ok(v)) => Some(match (&ty, v) {
                    (Type::Float, ConstValue::Int(i)) => ConstValue::Float(i as f32),
                    (_, v) => v,
                }),
                Some(Err(d)) => {
                    diags.push(d);
                    None
                }
                None => None,
            };
            params.push(Param {
                name: prm.name().map(|n| n.text().to_string()).unwrap_or_default(),
                ty,
                default,
            });
        }
        if let Some(((first, written, guess), rest)) = unresolved.split_first() {
            let mut d = Diagnostic::new(
                &codes::E0301,
                format!("`{}` declares types that do not exist", name.text()),
            )
            .primary(*first, format!("`{written}` is no type; read as `{guess}`"));
            for (s, w, g) in rest {
                d = d.secondary(*s, format!("`{w}` is no type; read as `{g}`"));
            }
            diags.push(d.help("types are lower case: `int`, `float`, `string`, `object`, `vector`, `action`, or an engine structure").note("the routine keeps its number; the signature shown is a recovery"));
        }
        Some(Function {
            name: name.text().to_string(),
            ret,
            params,
            recovered: !unresolved.is_empty(),
            span,
            doc: doc_comment(f),
        })
    }

    /// Replace the declarations of routines with those `bytes` declares, a
    /// file of prototypes in `nwscript.nss`'s form. Each routine keeps its
    /// number. This is how a profile compiles against declarations other
    /// than the shipped ones, such as the older `ActionStartConversation`
    /// BioWare's own scripts were compiled against. A prototype naming no
    /// routine in the table is reported and ignored.
    ///
    /// A prototype under a `// nwscript: routine N` comment is routine `N`
    /// instead, whatever its name: the table's earlier eras, where a routine
    /// had another number or a number had another routine. The name then
    /// calls routine `N`, and the routine `N` replaces is no longer callable
    /// by its own name.
    pub fn overlay(&mut self, file: FileId, bytes: &[u8]) -> Vec<Diagnostic> {
        let p = parse(file, bytes);
        let mut diags: Vec<Diagnostic> = p.diagnostics().to_vec();
        for f in p.tree().functions() {
            let Some(func) = self.function_decl(&p, &f, &mut diags) else {
                continue;
            };
            let slot = match routine_directive(&func.doc) {
                None => self.by_name.get(&func.name).copied(),
                Some(Ok(n)) if n < self.functions.len() => Some(n),
                Some(Ok(n)) => {
                    diags.push(
                        Diagnostic::new(
                            &codes::E0304,
                            format!(
                                "the overlay places `{}` at routine {n}, past the table's last",
                                func.name
                            ),
                        )
                        .primary(func.span, "declared here")
                        .help(format!(
                            "nwscript.nss declares routines 0 to {}",
                            self.functions.len().saturating_sub(1)
                        )),
                    );
                    continue;
                }
                Some(Err(text)) => {
                    diags.push(
                        Diagnostic::new(&codes::E0304, format!("`{text}` is no routine number"))
                            .primary(func.span, "the prototype it applies to")
                            .help("write `// nwscript: routine N` with N a routine number, on the line above the prototype"),
                    );
                    continue;
                }
            };
            let Some(i) = slot else {
                diags.push(
                    Diagnostic::new(
                        &codes::E0304,
                        format!("the overlay declares `{}`, which is no engine routine", func.name),
                    )
                    .primary(func.span, "not in nwscript.nss")
                    .help("an overlay changes the declarations of routines nwscript.nss has; `// nwscript: routine N` above a prototype places it at routine N"),
                );
                continue;
            };
            let displaced = std::mem::replace(&mut self.functions[i], func);
            if displaced.name != self.functions[i].name
                && self.by_name.get(&displaced.name) == Some(&i)
            {
                self.by_name.remove(&displaced.name);
            }
            self.by_name.insert(self.functions[i].name.clone(), i);
        }
        diags
    }

    /// A type as written, or `None` after a syntax error. An unknown name is
    /// recorded in `unresolved` with the type it is read as.
    fn resolve_type(
        &self,
        p: &Parse,
        ty: &ast::Type,
        unresolved: &mut Vec<(Span, String, Type)>,
    ) -> Option<Type> {
        Some(match ty.kind()? {
            TypeKind::Void => Type::Void,
            TypeKind::Int => Type::Int,
            TypeKind::Float => Type::Float,
            TypeKind::String => Type::String,
            TypeKind::Object => Type::Object,
            TypeKind::Vector => Type::Vector,
            TypeKind::Action => Type::Action,
            TypeKind::Struct(n) => {
                Type::Struct(n.map(|n| n.text().to_string()).unwrap_or_default())
            }
            TypeKind::Named(tok) => match self.structure(tok.text()) {
                Some(t) => t,
                None => {
                    let guess = guess_type(tok.text());
                    unresolved.push((
                        p.span_of(tok.clone()),
                        tok.text().to_string(),
                        guess.clone(),
                    ));
                    guess
                }
            },
        })
    }

    /// The value of a constant expression as `nwscript.nss` writes them: a
    /// literal, a negated literal, an earlier constant's name, `OBJECT_SELF`
    /// or `OBJECT_INVALID`, or a vector of float literals.
    fn const_value(&self, p: &Parse, e: &Expr) -> Result<ConstValue, Diagnostic> {
        let bad = |e: &Expr, what: &str| {
            Diagnostic::new(
                &codes::E0302,
                format!("{what} is not a constant this table can read"),
            )
            .primary(
                p.span_of(e.syntax().clone()),
                "expected a literal, a constant's name, or a vector of literals",
            )
        };
        match e {
            Expr::Literal(l) => match l.value() {
                Some(LiteralValue::Int(i)) => i32::try_from(i)
                    .or_else(|_| u32::try_from(i).map(|u| u as i32))
                    .map(ConstValue::Int)
                    .map_err(|_| bad(e, "this integer")),
                Some(LiteralValue::Float(f)) => Ok(ConstValue::Float(f)),
                Some(LiteralValue::String(s)) => Ok(ConstValue::String(s)),
                Some(LiteralValue::ObjectSelf) => Ok(ConstValue::Object(0)),
                Some(LiteralValue::ObjectInvalid) => Ok(ConstValue::Object(1)),
                None => Err(bad(e, "this literal")),
            },
            Expr::Prefix(pe) if pe.op() == Some(PrefixOp::Neg) => {
                match pe.operand().map(|o| self.const_value(p, &o)) {
                    Some(Ok(ConstValue::Int(i))) => Ok(ConstValue::Int(i.wrapping_neg())),
                    Some(Ok(ConstValue::Float(f))) => Ok(ConstValue::Float(-f)),
                    _ => Err(bad(e, "this negation")),
                }
            }
            Expr::Paren(pe) => pe
                .expr()
                .map_or_else(|| Err(bad(e, "this"))?, |inner| self.const_value(p, &inner)),
            Expr::Name(n) => self
                .constant(&n.name())
                .map(|c| c.value.clone())
                .ok_or_else(|| bad(e, &format!("`{}`", n.name()))),
            Expr::Vector(v) => {
                let parts: Vec<f32> = v
                    .elements()
                    .map(|x| match self.const_value(p, &x) {
                        Ok(ConstValue::Float(f)) => Ok(f),
                        Ok(ConstValue::Int(i)) => Ok(i as f32),
                        _ => Err(bad(&x, "this vector element")),
                    })
                    .collect::<Result<_, _>>()?;
                <[f32; 3]>::try_from(parts)
                    .map(ConstValue::Vector)
                    .map_err(|_| bad(e, "a vector without three elements"))
            }
            _ => Err(bad(e, "this expression")),
        }
    }
}

/// The type an unknown type name most likely means: the type it spells in
/// another case, or the type it starts with (`OBJECT_ID` is an object).
fn guess_type(name: &str) -> Type {
    let lower = name.to_ascii_lowercase();
    for (word, t) in [
        ("int", Type::Int),
        ("float", Type::Float),
        ("string", Type::String),
        ("object", Type::Object),
        ("vector", Type::Vector),
        ("void", Type::Void),
    ] {
        if lower == word || lower.starts_with(&format!("{word}_")) {
            return t;
        }
    }
    Type::Int
}

/// The comment lines directly above a prototype.
/// The routine number a `nwscript: routine N` line of a prototype's comment
/// gives: `None` without one, the text after `routine` when it is no number.
fn routine_directive(doc: &str) -> Option<Result<usize, String>> {
    doc.lines().find_map(|l| {
        let rest = l.trim().strip_prefix("nwscript:")?.trim_start();
        let n = rest.strip_prefix("routine")?.trim();
        Some(n.parse().map_err(|_| n.to_string()))
    })
}

fn doc_comment(f: &ast::FunctionDecl) -> String {
    use nwscript_syntax::SyntaxKind;
    let mut lines = Vec::new();
    let mut tok = f.syntax().first_token().and_then(|t| t.prev_token());
    let mut newlines = 0;
    while let Some(t) = tok {
        match t.kind() {
            SyntaxKind::LINE_COMMENT => {
                lines.push(t.text().trim_start_matches('/').trim().to_string());
                newlines = 0;
            }
            SyntaxKind::NEWLINE => {
                newlines += 1;
                if newlines > 1 {
                    break;
                }
            }
            SyntaxKind::WHITESPACE => {}
            _ => break,
        }
        tok = t.prev_token();
    }
    lines.reverse();
    lines.join("\n")
}
