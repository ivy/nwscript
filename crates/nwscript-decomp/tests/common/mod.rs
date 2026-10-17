//! Test helpers: the corpus, the engine table, and the interim verifier,
//! nwnnsscomp run as a black box behind the [`Compiler`] interface.

#![allow(dead_code)]

use nwscript_decomp::CompileError;
use nwscript_sema::EngineTable;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The corpus root, or `None` (with a message naming the variable) when it is absent.
pub fn corpus() -> Option<PathBuf> {
    let dir = std::env::var_os("NWSCRIPT_CORPUS")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/nwscript/corpus/k1"))
        })?;
    if dir.join("ncs").is_dir() && dir.join("nwscript.nss").is_file() {
        Some(dir)
    } else {
        eprintln!(
            "skipping: no corpus at {} (set NWSCRIPT_CORPUS; `cargo xtask corpus extract` writes one)",
            dir.display()
        );
        None
    }
}

pub fn table(corpus: &Path) -> EngineTable {
    let bytes = std::fs::read(corpus.join("nwscript.nss")).expect("nwscript.nss");
    EngineTable::from_nwscript(nwscript_diag::FileId(0), &bytes).0
}

/// nwnnsscomp: `$NWNNSSCOMP`, else modshop's native build beside this checkout.
pub fn nwnnsscomp() -> Option<PathBuf> {
    let p = std::env::var_os("NWNNSSCOMP")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                PathBuf::from(h)
                    .join("src/github.com/ivy/modshop/games/kotor/data/tools/nwnnsscomp")
            })
        })?;
    if p.is_file() {
        Some(p)
    } else {
        eprintln!(
            "skipping: no nwnnsscomp at {} (set NWNNSSCOMP)",
            p.display()
        );
        None
    }
}

/// Compile `source` with nwnnsscomp `-v1.00` in `work`, against `include`
/// (a directory whose nwscript.nss has line 5145 corrected: nwnnsscomp
/// never returns on the shipped one).
pub fn oracle_compile(
    bin: &Path,
    include: &Path,
    work: &Path,
    source: &[u8],
) -> Result<Vec<u8>, CompileError> {
    let fail = |m: String| CompileError {
        message: m,
        line: None,
    };
    let _ = std::fs::remove_dir_all(work);
    std::fs::create_dir_all(work).map_err(|e| fail(format!("work directory: {e}")))?;
    std::fs::write(work.join("t.nss"), source).map_err(|e| fail(e.to_string()))?;
    let log = std::fs::File::create(work.join("log")).map_err(|e| fail(e.to_string()))?;
    let mut child = Command::new(bin)
        .args(["-c", "-g", "-q", "-v1.00", "-i"])
        .arg(include)
        .arg("t.nss")
        .current_dir(work)
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(|e| fail(e.to_string()))?)
        .stderr(log)
        .spawn()
        .map_err(|e| fail(format!("starting nwnnsscomp: {e}")))?;
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() > Duration::from_secs(30) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(fail("nwnnsscomp timed out".into()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(2)),
            Err(e) => return Err(fail(e.to_string())),
        }
    };
    let out = std::fs::read(work.join("t.ncs"));
    match out {
        Ok(b) if status.success() => Ok(b),
        _ => {
            let text = std::fs::read_to_string(work.join("log")).unwrap_or_default();
            let line = text
                .lines()
                .find(|l| l.contains("Error"))
                .unwrap_or("failed");
            // `t.nss(12): Error: ...`
            let n = line
                .strip_prefix("t.nss(")
                .and_then(|r| r.split(')').next())
                .and_then(|n| n.parse().ok());
            Err(CompileError {
                message: line.trim_start_matches("t.nss").to_string(),
                line: n,
            })
        }
    }
}
