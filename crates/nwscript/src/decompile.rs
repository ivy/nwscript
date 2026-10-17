//! Source from bytecode, each function verified by recompiling it.
//!
//! The compiler that verifies is the caller's: anything implementing
//! [`Compiler`], source bytes in and NCS bytes out.

use nwscript_diag::{Diagnostic, SourceMap, Span, codes};
use nwscript_ncs::decode;
use nwscript_sema::EngineTable;

pub use nwscript_decomp::{CompileError, Compiler, Decompiled, Function, Status};

/// Decode NCS bytes and recover their source; with a compiler, verify each
/// function. Errors point into the bytes, registered in `map` under `name`.
/// The second value warns (E0602) about each function that is not verified.
pub fn decompile<'t>(
    name: &str,
    bytes: &[u8],
    table: &'t EngineTable,
    compiler: Option<&mut dyn Compiler>,
    map: &mut SourceMap,
) -> Result<(Decompiled<'t>, Vec<Diagnostic>), Diagnostic> {
    let file = map.add_binary(name, bytes.to_vec());
    let program = decode(file, bytes)?;
    let mut d = nwscript_decomp::decompile(&program, table).map_err(|e| {
        let start = bytes.len().min(13);
        Diagnostic::new(&codes::E0601, format!("{name}: {e}")).primary(
            Span::new(file, start, bytes.len().min(19)),
            "the code starts here",
        )
    })?;
    if let Some(c) = compiler {
        d.verify(&program, c);
    }
    let mut warnings = Vec::new();
    for f in &d.functions {
        if matches!(f.status, Status::Verified | Status::Unchecked) {
            continue;
        }
        let at = match &f.status {
            Status::Unverified { at: Some(at), .. } => *at as usize,
            _ => f.start as usize,
        };
        warnings.push(
            Diagnostic::new(&codes::E0602, format!("{}: {}", f.name, f.status)).primary(
                Span::new(file, at, (f.end as usize).max(at)),
                "the function's bytes",
            ),
        );
    }
    Ok((d, warnings))
}

/// The project's compiler as a verifier: the recovered source is checked
/// through a resolver (for `nwscript.nss`) and compiled for a game.
///
/// Verification asks whether source compiles to given bytes, not whether
/// it is good style, so no lints run and what BioWare's compiler and
/// nwnnsscomp accepted is accepted (`--compat bioware`). The declarations
/// are the checker's: [`crate::Declarations::BioWare`] restores the
/// eleven-argument `ActionStartConversation` BioWare's bytecode calls.
pub struct ProjectCompiler<R: crate::Resolver> {
    checker: crate::Checker,
    resolver: R,
    game: crate::Game,
    levels: nwscript_diag::Levels,
}

impl<R: crate::Resolver> ProjectCompiler<R> {
    pub fn new(resolver: R, game: crate::Game, declarations: crate::Declarations) -> Self {
        let mut levels = nwscript_diag::Levels::default();
        levels.accepted = codes::NWNNSSCOMP_ACCEPTED
            .iter()
            .chain(codes::BIOWARE_ACCEPTED)
            .map(|c| c.id)
            .collect();
        ProjectCompiler {
            checker: crate::Checker::with_declarations(declarations, game),
            resolver,
            game,
            levels,
        }
    }
}

impl<R: crate::Resolver> Compiler for ProjectCompiler<R> {
    fn compile(&mut self, source: &[u8]) -> Result<Vec<u8>, CompileError> {
        let mut map = SourceMap::new();
        let out = self
            .checker
            .check(&mut self.resolver, &mut map, "decompiled.nss", source);
        let main = map.files().next().map(|(id, _)| id);
        let first_error = out
            .diagnostics
            .into_iter()
            .filter_map(|d| self.levels.apply(d))
            .find(|d| d.is_error());
        let fail = |d: &Diagnostic| CompileError {
            message: format!("error[{}]: {}", d.code.id, d.message),
            line: d
                .primary_span()
                .filter(|s| Some(s.file) == main)
                .map(|s| map.get(s.file).line_col(s.start).line as u32),
        };
        if let Some(d) = first_error {
            return Err(fail(&d));
        }
        let analysis = out.analysis.ok_or_else(|| CompileError {
            message: "no nwscript.nss to check against".into(),
            line: None,
        })?;
        crate::compile::compile(&analysis.program, self.game)
            .map(|p| nwscript_ncs::encode(&p))
            .map_err(|d| fail(&d))
    }
}
