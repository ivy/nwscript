//! Reading inputs and writing outputs, where `-` means stdin or stdout.

use nwscript::{InputRecord, OutputRecord, sha256_hex};
use std::io::{Read, Write};
use std::path::Path;

/// Read a file, or stdin for `-`, recording it with `role`.
pub fn read_input(path: &Path, role: &str) -> Result<(Vec<u8>, InputRecord), String> {
    let bytes = if path.as_os_str() == "-" {
        let mut b = Vec::new();
        std::io::stdin()
            .read_to_end(&mut b)
            .map_err(|e| format!("reading stdin: {e}"))?;
        b
    } else {
        std::fs::read(path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))?
    };
    let record = InputRecord {
        role: role.into(),
        path: path.display().to_string(),
        sha256: sha256_hex(&bytes),
    };
    Ok((bytes, record))
}

/// Write a file, or stdout for `-`, returning its record.
pub fn write_output(path: &Path, bytes: &[u8]) -> Result<OutputRecord, String> {
    if path.as_os_str() == "-" {
        let mut out = std::io::stdout().lock();
        out.write_all(bytes)
            .and_then(|()| out.flush())
            .map_err(|e| format!("writing stdout: {e}"))?;
    } else {
        std::fs::write(path, bytes)
            .map_err(|e| format!("cannot write `{}`: {e}", path.display()))?;
    }
    Ok(OutputRecord {
        path: path.display().to_string(),
        sha256: sha256_hex(bytes),
        bytes: bytes.len(),
    })
}

/// A file name for diagnostics: the path, or `<stdin>`.
pub fn display_name(path: &Path) -> String {
    if path.as_os_str() == "-" {
        "<stdin>".into()
    } else {
        path.display().to_string()
    }
}
