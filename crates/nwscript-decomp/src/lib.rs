//! Source from bytecode, each function verified by recompilation.
//!
//! [`decompile`] reads a decoded NCS [`Program`] back into NWScript source:
//! the globals, `main` or `StartingConditional`, and every function the
//! program calls, with their parameters, locals, control flow and
//! expressions. Types come from the stack (`RSADD`'s type byte, the
//! operators' type bytes) and from the engine routines' declarations in
//! `nwscript.nss`.
//!
//! Recovered source that recompiles to the original bytes is right by
//! construction, so [`Decompiled::verify`] compiles the recovered script
//! with a [`Compiler`] the caller supplies and compares each function's
//! instructions with the original's. Every function then carries a
//! [`Status`]: verified, unverified with the first offset that differs, or
//! not recovered with the reason; the printed source states it in a comment
//! above the function. The crate does no I/O and runs no compiler of its own.
//!
//! ```
//! use nwscript_decomp::{decompile, Status};
//! # let table = nwscript_sema::EngineTable::default();
//! let program = nwscript_ncs::asm::parse(
//!     nwscript_diag::FileId(0),
//!     b"JSR main\nRETN\nmain:\nRETN\n",
//! ).unwrap();
//! let mut d = decompile(&program, &table).unwrap();
//! // Any closure from source bytes to NCS bytes verifies.
//! let mut compiler = |_: &[u8]| Ok(nwscript_ncs::encode(&program));
//! d.verify(&program, &mut compiler);
//! assert_eq!(d.functions[0].status, Status::Verified);
//! assert!(String::from_utf8_lossy(&d.source()).contains("void main()"));
//! ```

mod ast;
mod print;
mod recover;
mod verify;

pub use verify::{CompileError, Compiler, function_ranges, globals_bytes, shape};

use ast::{Expr, FuncKind, Place, Script, Stmt};
use nwscript_ncs::Program;
use nwscript_sema::EngineTable;
use std::fmt;

/// What is known about one recovered function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Recompiling the recovered source gives the original instructions.
    Verified,
    /// Recompiling gives something else: `at` is the original offset of the
    /// first instruction that differs, when the compiler produced code.
    Unverified { at: Option<u32>, detail: String },
    /// The bytecode has a shape the decompiler cannot read back.
    NotRecovered { reason: String },
    /// Recovered, and not checked: no verification was run.
    Unchecked,
}

impl Status {
    /// The status's name in JSON output: `verified`, `unverified`,
    /// `not_recovered` or `unchecked`.
    pub fn name(&self) -> &'static str {
        match self {
            Status::Verified => "verified",
            Status::Unverified { .. } => "unverified",
            Status::NotRecovered { .. } => "not_recovered",
            Status::Unchecked => "unchecked",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Verified => f.write_str("verified: recompiles to the original bytes"),
            Status::Unverified {
                at: Some(at),
                detail,
            } if detail.is_empty() => {
                write!(f, "unverified: recompiles differently from offset {at}")
            }
            Status::Unverified {
                at: Some(at),
                detail,
            } => {
                write!(
                    f,
                    "unverified: recompiles differently from offset {at}; {detail}"
                )
            }
            Status::Unverified { at: None, detail } => write!(f, "unverified: {detail}"),
            Status::NotRecovered { reason } => write!(f, "not recovered: {reason}"),
            Status::Unchecked => f.write_str("unchecked: recovered, not recompiled"),
        }
    }
}

/// What a recovered function is to the program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The code before the entry point that initialises the globals.
    Globals,
    /// `main` or `StartingConditional`.
    Entry,
    /// A function the script defines, `sub_OFFSET` until a fold names it.
    User,
}

/// One function of the original program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    /// Its name in the recovered source: `main`, `StartingConditional`,
    /// `sub_OFFSET` (the hex offset `nws disasm` labels it with), or
    /// `(globals)` for the code that initialises the globals.
    pub name: String,
    pub kind: Kind,
    /// Its byte range in the original file; the first function's includes
    /// the entry stub.
    pub start: u32,
    pub end: u32,
    pub status: Status,
}

/// Recovered functions and globals to replace by an include's: the
/// libraries to `#include`, the names the libraries give the leading
/// globals, and the library name of each function replaced. A fold is
/// applied by [`Decompiled::fold`] only when the folded source recompiles
/// to the original bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fold {
    /// The `#include` lines, in order, without quotes or extension.
    pub includes: Vec<String>,
    /// The libraries' names for the first `globals.len()` recovered
    /// globals, in declaration order; those declarations are dropped.
    pub globals: Vec<String>,
    /// Function index into [`Decompiled::functions`] and the name the
    /// library defines it under.
    pub functions: Vec<(usize, String)>,
    /// Recovered structs the libraries declare, by index into the
    /// recovered program's structs.
    pub structs: Vec<FoldStruct>,
}

/// A recovered struct that an included library declares: its name and
/// field names there. The recovered declaration is not printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldStruct {
    /// The struct's index, as [`Global::struct_index`] gives it.
    pub index: usize,
    pub name: String,
    /// One name per field, in order.
    pub fields: Vec<String>,
}

/// A recovered whole global variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Global {
    /// Its recovered name.
    pub name: String,
    /// Its size in bytes on the stack.
    pub size: u32,
    /// The recovered struct it is, when its slots were joined into one
    /// struct variable.
    pub struct_index: Option<usize>,
    /// How many fields that struct has.
    pub fields: usize,
}

/// A recovered program, ready to print.
pub struct Decompiled<'t> {
    script: Script,
    table: &'t EngineTable,
    pub functions: Vec<Function>,
    header: Option<Vec<String>>,
    includes: Vec<String>,
}

/// Recover source from a program. Functions start [`Status::Unchecked`] or
/// [`Status::NotRecovered`]; [`Decompiled::verify`] checks them. The error
/// is for a program with no recognisable entry point.
pub fn decompile<'t>(program: &Program, table: &'t EngineTable) -> Result<Decompiled<'t>, String> {
    let script = recover::recover(program, table)?;
    let functions = script
        .funcs
        .iter()
        .map(|f| Function {
            name: f.name.clone(),
            kind: match f.kind {
                FuncKind::Globals => Kind::Globals,
                FuncKind::Entry => Kind::Entry,
                FuncKind::User => Kind::User,
            },
            start: f.start,
            end: f.end,
            status: match &f.body {
                Ok(_) => Status::Unchecked,
                Err(r) => Status::NotRecovered { reason: r.clone() },
            },
        })
        .collect();
    Ok(Decompiled {
        script,
        table,
        functions,
        header: None,
        includes: Vec::new(),
    })
}

/// The engine calls in a byte range whose argument count is not what the
/// table declares: bytecode compiled against an older `nwscript.nss`,
/// which a compiler reading this one cannot reproduce.
fn stale_calls(p: &Program, table: &EngineTable, start: u32, end: u32) -> String {
    let mut seen: Vec<(u16, u8)> = Vec::new();
    for (at, x) in p.iter() {
        if at < start || at >= end {
            continue;
        }
        if let nwscript_ncs::Operands::Action { routine, argc } = x.operands
            && let Some(f) = table.functions.get(usize::from(routine))
            && usize::from(argc) != f.params.len()
            && !seen.contains(&(routine, argc))
        {
            seen.push((routine, argc));
        }
    }
    seen.iter()
        .map(|&(r, argc)| {
            let f = &table.functions[usize::from(r)];
            format!(
                "the bytecode passes {} {argc} arguments where nwscript.nss declares {}, so it was compiled against an older declaration",
                f.name,
                f.params.len()
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The whole global variables in declaration order: the first slot of each
/// variable the globals function declares.
fn global_roots(script: &Script) -> Vec<usize> {
    let mut roots = Vec::new();
    for f in &script.funcs {
        if f.kind != FuncKind::Globals {
            continue;
        }
        if let Ok(body) = &f.body {
            for st in body {
                if let Stmt::Decl(v) = st {
                    let root = script.root(*v).0;
                    if !roots.contains(&root) {
                        roots.push(root);
                    }
                }
            }
        }
    }
    roots
}

/// How many times verification sets a function aside that does not
/// compile and tries the rest again.
const COMPILE_RETRIES: usize = 8;

impl Decompiled<'_> {
    /// How many functions are verified.
    pub fn verified(&self) -> usize {
        self.functions
            .iter()
            .filter(|f| f.status == Status::Verified)
            .count()
    }

    /// The recovered source: Windows-1252 bytes, a comment above each
    /// function stating its status.
    pub fn source(&self) -> Vec<u8> {
        print::to_bytes(&self.print(&self.script).text)
    }

    /// Print these lines, each a `//` comment, above the source in place
    /// of the default header.
    pub fn set_header(&mut self, lines: Vec<String>) {
        self.header = Some(lines);
    }

    /// The libraries a fold included, in order.
    pub fn includes(&self) -> &[String] {
        &self.includes
    }

    /// The whole global variables in declaration order. A vector or struct
    /// whose slots were joined counts once, at its full size; one whose
    /// slots were not joined appears as one variable per slot.
    pub fn globals(&self) -> Vec<Global> {
        global_roots(&self.script)
            .iter()
            .map(|&v| {
                let var = &self.script.vars[v];
                let struct_index = match var.ty {
                    Some(ast::Ty::Struct(i)) => Some(i),
                    _ => None,
                };
                Global {
                    name: var.name.clone(),
                    size: var.size,
                    struct_index,
                    fields: struct_index.map_or(0, |i| self.script.structs[i].fields.len()),
                }
            })
            .collect()
    }

    /// A function's signature as types alone: the return type (`void` for
    /// none) and each parameter's type, `None` for a parameter the bytecode
    /// never reads, whose type cannot be known.
    pub fn signature(&self, i: usize) -> Option<(String, Vec<Option<String>>)> {
        let f = self.script.funcs.get(i)?;
        let ret = match f.ret {
            None => "void".to_string(),
            Some(t) => print::type_text(&self.script, self.table, Some(t)),
        };
        let params = f
            .params
            .iter()
            .map(|&v| {
                self.script.vars[v]
                    .ty
                    .map(|t| print::type_text(&self.script, self.table, Some(t)))
            })
            .collect();
        Some((ret, params))
    }

    /// The source as `fold` would print it, unverified: what [`Decompiled::fold`]
    /// compiles, for a caller that wants to see why a fold was refused.
    pub fn folded_source(&self, fold: &Fold) -> Result<Vec<u8>, String> {
        let script = self.apply(fold)?;
        Ok(print::to_bytes(
            &self.print_with(&script, &fold.includes).text,
        ))
    }

    /// Replace recovered functions and leading globals by a library's,
    /// if the folded source still recompiles to the original bytes: every
    /// function verified before must compare the same after. Otherwise
    /// nothing changes and the error says which function differed.
    pub fn fold(
        &mut self,
        fold: Fold,
        original: &Program,
        compiler: &mut dyn Compiler,
    ) -> Result<(), String> {
        let script = self.apply(&fold)?;
        let printed = self.print_with(&script, &fold.includes);
        let bytes = compiler
            .compile(&print::to_bytes(&printed.text))
            .map_err(|e| format!("the folded source does not compile: {}", e.message))?;
        let mut map = nwscript_diag::SourceMap::new();
        let re = nwscript_ncs::decode_named("folded.ncs", &bytes, &mut map)
            .map_err(|d| format!("the folded compile does not decode: {}", d.message))?;
        let results = verify::compare(original, &re);
        for (i, f) in self.functions.iter().enumerate() {
            if f.status != Status::Verified {
                continue;
            }
            match results.get(i) {
                Some(verify::Compared::Same) => {}
                Some(verify::Compared::DiffersAt(at)) => {
                    return Err(format!(
                        "{} recompiles differently when folded, from offset {at}",
                        f.name
                    ));
                }
                _ => {
                    return Err(format!(
                        "{} has no counterpart in the folded compile",
                        f.name
                    ));
                }
            }
        }
        self.script = script;
        self.includes = fold.includes;
        Ok(())
    }

    /// The script with `fold` applied: globals renamed and the leading
    /// ones dropped, functions renamed and marked folded.
    fn apply(&self, fold: &Fold) -> Result<Script, String> {
        let mut script = self.script.clone();
        let roots = global_roots(&script);
        if fold.globals.len() > roots.len() {
            return Err(format!(
                "the fold names {} globals but the script declares {}",
                fold.globals.len(),
                roots.len()
            ));
        }
        for (&v, name) in roots.iter().zip(&fold.globals) {
            script.vars[v].name = name.clone();
        }
        let root_of: Vec<usize> = (0..script.vars.len()).map(|v| script.root(v).0).collect();
        let dropped: std::collections::HashSet<usize> =
            roots[..fold.globals.len()].iter().copied().collect();
        for f in &mut script.funcs {
            if f.kind == FuncKind::Globals
                && let Ok(body) = &mut f.body
            {
                body.retain(|st| match st {
                    Stmt::Decl(v) | Stmt::Expr(Expr::Assign(Place::Var(v), _)) => {
                        !dropped.contains(&root_of[*v])
                    }
                    _ => true,
                });
            }
        }
        for (i, name) in &fold.functions {
            let f = script
                .funcs
                .get_mut(*i)
                .ok_or_else(|| format!("no function {i} to fold"))?;
            if f.kind != FuncKind::User {
                return Err(format!(
                    "{} is not a function a library could define",
                    f.name
                ));
            }
            f.name = name.clone();
            f.folded = true;
        }
        for st in &fold.structs {
            let def = script
                .structs
                .get_mut(st.index)
                .ok_or_else(|| format!("no struct {} to fold", st.index))?;
            if st.fields.len() != def.fields.len() {
                return Err(format!(
                    "struct {} has {} fields but the fold names {}",
                    def.name,
                    def.fields.len(),
                    st.fields.len()
                ));
            }
            def.name = st.name.clone();
            def.field_names = st.fields.clone();
            def.folded = true;
        }
        Ok(script)
    }

    fn print(&self, script: &Script) -> print::Printed {
        self.print_with(script, &self.includes)
    }

    fn print_with(&self, script: &Script, includes: &[String]) -> print::Printed {
        let notes: Vec<String> = self
            .functions
            .iter()
            .map(|f| f.status.to_string())
            .collect();
        let total = self.functions.len();
        let header = match &self.header {
            Some(lines) => lines.join("\n"),
            None => format!(
                "Recovered from NCS bytecode: {} of {total} functions verified by recompiling.",
                self.verified()
            ),
        };
        print::print(
            script,
            &print::Context {
                table: self.table,
                notes: &notes,
                header: &header,
                includes,
            },
        )
    }

    /// Compile the recovered source and compare each function with
    /// `original`, the program it was recovered from. A function the
    /// compiler rejects is set aside (compiled as a placeholder with its
    /// signature) so the others can still be checked.
    pub fn verify(&mut self, original: &Program, compiler: &mut dyn Compiler) {
        let mut script = self.script.clone();
        let mut rejected: Vec<(usize, String)> = Vec::new();
        for _ in 0..=COMPILE_RETRIES {
            let printed = self.print(&script);
            match compiler.compile(&print::to_bytes(&printed.text)) {
                Ok(bytes) => {
                    self.compare(original, &bytes, &rejected);
                    return;
                }
                Err(e) => {
                    let culprit = e.line.and_then(|line| {
                        printed
                            .lines
                            .iter()
                            .position(|&(a, b)| a <= line && line <= b)
                    });
                    match culprit {
                        Some(i)
                            if script.funcs[i].body.is_ok()
                                && script.funcs[i].kind != FuncKind::Globals =>
                        {
                            script.funcs[i].body = Err(String::new());
                            rejected.push((i, e.message));
                        }
                        _ => {
                            for (i, f) in self.functions.iter_mut().enumerate() {
                                if matches!(f.status, Status::NotRecovered { .. }) {
                                    continue;
                                }
                                let msg = rejected
                                    .iter()
                                    .find(|r| r.0 == i)
                                    .map_or(&e.message, |r| &r.1);
                                f.status = Status::Unverified {
                                    at: None,
                                    detail: format!("the recovered script does not compile: {msg}"),
                                };
                            }
                            return;
                        }
                    }
                }
            }
        }
    }

    fn compare(&mut self, original: &Program, bytes: &[u8], rejected: &[(usize, String)]) {
        let mut map = nwscript_diag::SourceMap::new();
        let re = match nwscript_ncs::decode_named("recompiled.ncs", bytes, &mut map) {
            Ok(p) => p,
            Err(d) => {
                for f in &mut self.functions {
                    if !matches!(f.status, Status::NotRecovered { .. }) {
                        f.status = Status::Unverified {
                            at: None,
                            detail: format!("the compiler's output does not decode: {}", d.message),
                        };
                    }
                }
                return;
            }
        };
        let results = verify::compare(original, &re);
        for (i, f) in self.functions.iter_mut().enumerate() {
            if matches!(f.status, Status::NotRecovered { .. }) {
                continue;
            }
            let stale = stale_calls(original, self.table, f.start, f.end);
            if let Some((_, msg)) = rejected.iter().find(|r| r.0 == i) {
                let mut detail = format!("does not compile: {msg}");
                if !stale.is_empty() {
                    detail = format!("{detail}; {stale}");
                }
                f.status = Status::Unverified { at: None, detail };
                continue;
            }
            f.status = match results.get(i) {
                Some(verify::Compared::Same) => Status::Verified,
                Some(verify::Compared::DiffersAt(at)) => Status::Unverified {
                    at: Some(*at),
                    detail: stale,
                },
                _ => Status::Unverified {
                    at: None,
                    detail: "no function of the recompiled program corresponds to it".into(),
                },
            };
        }
    }
}
