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
