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

pub use verify::{CompileError, Compiler};

use ast::{FuncKind, Script};
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

/// One function of the original program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    /// Its name in the recovered source: `main`, `StartingConditional`,
    /// `sub_OFFSET` (the hex offset `nws disasm` labels it with), or
    /// `(globals)` for the code that initialises the globals.
    pub name: String,
    /// Its byte range in the original file; the first function's includes
    /// the entry stub.
    pub start: u32,
    pub end: u32,
    pub status: Status,
}

/// A recovered program, ready to print.
pub struct Decompiled<'t> {
    script: Script,
    table: &'t EngineTable,
    pub functions: Vec<Function>,
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
    })
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

    fn print(&self, script: &Script) -> print::Printed {
        let notes: Vec<String> = self
            .functions
            .iter()
            .map(|f| f.status.to_string())
            .collect();
        let total = self.functions.len();
        let header = format!(
            "Recovered from NCS bytecode: {} of {total} functions verified by recompiling.",
            self.verified()
        );
        print::print(
            script,
            &print::Context {
                table: self.table,
                notes: &notes,
                header: &header,
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
            if let Some((_, msg)) = rejected.iter().find(|r| r.0 == i) {
                f.status = Status::Unverified {
                    at: None,
                    detail: format!("does not compile: {msg}"),
                };
                continue;
            }
            f.status = match results.get(i) {
                Some(verify::Compared::Same) => Status::Verified,
                Some(verify::Compared::DiffersAt(at)) => Status::Unverified {
                    at: Some(*at),
                    detail: String::new(),
                },
                _ => Status::Unverified {
                    at: None,
                    detail: "no function of the recompiled program corresponds to it".into(),
                },
            };
        }
    }
}
