//! Running bytecode under an engine.

use nwscript_diag::{Diagnostic, FileId, SourceMap, Span, codes};
use nwscript_vm::{Engine, Limits, Routine, Run, run};

/// Decode and run NCS bytes. A decoding error is the only `Err`; a fault
/// while running is in the `Run` and, as a diagnostic, in the second value.
pub fn run_bytes(
    name: &str,
    bytes: &[u8],
    routines: &[Routine],
    engine: &mut dyn Engine,
    limits: Limits,
    map: &mut SourceMap,
) -> Result<(Run, Option<Diagnostic>), Diagnostic> {
    let program = nwscript_ncs::decode_named(name, bytes, map)?;
    let file = FileId(map.files().count() as u32 - 1);
    let r = run(&program, routines, engine, limits);
    let d = r
        .fault
        .as_ref()
        .map(|f| fault_diagnostic(f, &r, &program, file));
    Ok((r, d))
}

fn fault_diagnostic(
    f: &nwscript_vm::Fault,
    r: &Run,
    p: &nwscript_ncs::Program,
    file: FileId,
) -> Diagnostic {
    let len = p
        .index_at(f.offset)
        .map_or(1, |i| p.instructions()[i].len());
    let span = Span::new(file, f.offset as usize, f.offset as usize + len);
    let at = p
        .index_at(f.offset)
        .map(|i| p.instructions()[i].to_string())
        .unwrap_or_default();
    if f.kind.is_engine_dependent() {
        let mut d = Diagnostic::new(&codes::E0502, f.to_string()).primary(span, at);
        if !r.unmodelled.is_empty() {
            d = d.note(format!(
                "depends on unmodelled routines: {}",
                r.unmodelled.join(", ")
            ));
        }
        d
    } else {
        Diagnostic::new(&codes::E0501, f.to_string()).primary(span, at)
    }
}
