//! Bytecode in and out: disassembly and assembly, each one call.

use nwscript_diag::{Diagnostic, SourceMap};
use nwscript_ncs::{Program, asm, decode_named, encode};

pub use nwscript_ncs::asm::PrintOptions;

/// Decode NCS bytes and print them as assembly. Errors point into the
/// bytes, registered in `map` under `name`.
pub fn disassemble(
    name: &str,
    bytes: &[u8],
    opts: &PrintOptions,
    map: &mut SourceMap,
) -> Result<(Program, String), Diagnostic> {
    let program = decode_named(name, bytes, map)?;
    let text = asm::print(&program, opts);
    Ok((program, text))
}

/// Parse assembly text into NCS bytes. Errors point into the text,
/// registered in `map` under `name`.
pub fn assemble(name: &str, text: &[u8], map: &mut SourceMap) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let file = map.add(name, text.to_vec());
    asm::parse(file, text).map(|p| encode(&p))
}
