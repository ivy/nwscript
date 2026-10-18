//! `xtask bench driver MODE FORMAT LIST OUT`: Rakata as a benchmark
//! comparator, speaking the protocol every batch driver speaks (see
//! `conformance/bench.toml`). One thread, the files in list order.
//!
//! - `start`: load nothing and exit; 3 with the reason on stderr when the
//!   format is not one this tool reads.
//! - `read`: read every file.
//! - `write`: read every file and write it again, in memory.
//! - `check`: read and write every file; a written file that differs from
//!   the original is saved as `OUT.d/I.EXT`.
//!
//! OUT receives one JSON line per file: `{"i": I, "ok": true}` or
//! `{"i": I, "error": "..."}`, and in `check` mode `"write"`: `identical`,
//! `different` (with `"out"`), or `"write_error"`.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::formats::{self, Input};

pub const UNSUPPORTED: u8 = 3;

pub fn main(args: &[String]) -> Result<std::process::ExitCode, String> {
    let [mode, format, list, out] = args else {
        return Err("usage: xtask bench driver MODE FORMAT LIST OUT".into());
    };
    if formats::by_name(format).is_none() {
        eprintln!("unsupported: no format `{format}`");
        return Ok(std::process::ExitCode::from(UNSUPPORTED));
    }
    let out = PathBuf::from(out);
    let mut sink = std::io::BufWriter::new(
        fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?,
    );
    let writes = formats::writes(format);
    match mode.as_str() {
        "start" => return Ok(std::process::ExitCode::SUCCESS),
        "write" if !writes => {
            eprintln!("unsupported: Rakata does not write {format}");
            return Ok(std::process::ExitCode::from(UNSUPPORTED));
        }
        "read" | "write" | "check" => {}
        other => return Err(format!("unknown mode `{other}`")),
    }
    let text = fs::read_to_string(list).map_err(|e| format!("{list}: {e}"))?;
    let saved = out.with_extension("d");
    if mode == "check" {
        let _ = fs::remove_dir_all(&saved);
        fs::create_dir_all(&saved).map_err(|e| format!("{}: {e}", saved.display()))?;
    }
    for (i, line) in text.lines().filter(|l| !l.is_empty()).enumerate() {
        let path = Path::new(line);
        let record = match fs::read(path) {
            Err(e) => serde_json::json!({"i": i, "error": e.to_string()}),
            Ok(bytes) => {
                let mdx = (format == "mdl")
                    .then(|| fs::read(path.with_extension("mdx")).ok())
                    .flatten();
                let input = Input {
                    bytes: &bytes,
                    mdx: mdx.as_deref(),
                };
                one(mode, format, writes, &input, path, i, &saved)
            }
        };
        writeln!(sink, "{record}").map_err(|e| e.to_string())?;
    }
    sink.flush().map_err(|e| e.to_string())?;
    Ok(std::process::ExitCode::SUCCESS)
}

fn one(
    mode: &str,
    format: &str,
    writes: bool,
    input: &Input,
    path: &Path,
    i: usize,
    saved: &Path,
) -> serde_json::Value {
    match mode {
        "read" => match formats::read(format, input) {
            Ok(()) => serde_json::json!({"i": i, "ok": true}),
            Err(e) => serde_json::json!({"i": i, "error": e}),
        },
        "write" => match formats::write(format, input) {
            Ok(w) => {
                std::hint::black_box(&w);
                serde_json::json!({"i": i, "ok": true})
            }
            Err(e) => serde_json::json!({"i": i, "error": e}),
        },
        _ => {
            if let Err(e) = formats::read(format, input) {
                return serde_json::json!({"i": i, "error": e});
            }
            if !writes {
                return serde_json::json!({"i": i, "ok": true});
            }
            match formats::write(format, input) {
                Ok(w) if w == input.bytes => {
                    serde_json::json!({"i": i, "ok": true, "write": "identical"})
                }
                Ok(w) => {
                    let ext = path
                        .extension()
                        .map(|e| e.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let p = saved.join(format!("{i}.{ext}"));
                    match fs::write(&p, &w) {
                        Ok(()) => serde_json::json!({
                            "i": i, "ok": true, "write": "different", "out": p.display().to_string()
                        }),
                        Err(e) => serde_json::json!({"i": i, "ok": true, "write_error": e.to_string()}),
                    }
                }
                Err(e) => serde_json::json!({"i": i, "ok": true, "write_error": e}),
            }
        }
    }
}
