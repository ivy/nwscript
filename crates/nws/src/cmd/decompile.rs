//! `nws decompile`: source from bytecode, each function verified by
//! recompiling it.
//!
//! Until the project's own compiler lands, verification runs nwnnsscomp as a
//! black box behind the library's `Compiler` interface: the recovered source
//! is written to a scratch directory and compiled there with `-v1.00`
//! against the directory that holds `nwscript.nss`.

use super::Ctx;
use crate::Exit;
use crate::cli::Format;
use crate::io::{display_name, read_input, write_output};
use nwscript::Envelope;
use nwscript::decompile::{CompileError, Compiler, Status};
use nwscript::diag::SourceMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  nws -I corpus/oracle decompile k_act_mispaz.ncs              print the source
  nws -I corpus/oracle decompile k_act_mispaz.ncs -o out.nss   write it to a file
  nws -I corpus/oracle decompile --strict x.ncs                exit 3 unless every function verifies
  nws -I corpus/oracle decompile --no-verify x.ncs             skip recompiling

Each function is recompiled and compared with the original bytes; a comment
above it says `verified`, `unverified` with the first offset that differs, or
`not recovered` with the reason, and each function that is not verified is
also reported as warning E0602. Verification runs nwnnsscomp (--compiler, or
$NWNNSSCOMP) with -v1.00 against the directory holding nwscript.nss, which
-I or --nwscript names; nwnnsscomp never returns on KotOR 1's shipped
nwscript.nss, so point it at a copy with line 5145 corrected (the corpus's
oracle/ directory). In JSON mode result.functions[] has each function's name,
status and offset, and result.verified the count.")]
pub struct Args {
    /// The NCS file, or - for stdin.
    pub input: PathBuf,
    /// Write the source here instead of stdout.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// Do not recompile: every recovered function is reported as unchecked.
    #[arg(long)]
    pub no_verify: bool,
    /// Exit 3 if any function is not verified.
    #[arg(long, conflicts_with = "no_verify")]
    pub strict: bool,
    /// The compiler to verify with: an nwnnsscomp executable.
    #[arg(long, value_name = "PATH", env = "NWNNSSCOMP")]
    pub compiler: Option<PathBuf>,
    /// Seconds before a verifying compilation is abandoned.
    #[arg(long, value_name = "SECS", default_value_t = 30)]
    pub timeout: u64,
}

/// nwnnsscomp as a verifier: compiles in a scratch directory with `-v1.00`.
struct Nwnnsscomp {
    bin: PathBuf,
    include: PathBuf,
    work: PathBuf,
    timeout: Duration,
}

impl Compiler for Nwnnsscomp {
    fn compile(&mut self, source: &[u8]) -> Result<Vec<u8>, CompileError> {
        let fail = |m: String| CompileError {
            message: m,
            line: None,
        };
        std::fs::create_dir_all(&self.work).map_err(|e| fail(format!("scratch directory: {e}")))?;
        let src = self.work.join("decompiled.nss");
        let out = self.work.join("decompiled.ncs");
        let _ = std::fs::remove_file(&out);
        std::fs::write(&src, source).map_err(|e| fail(format!("scratch source: {e}")))?;
        let log_path = self.work.join("log");
        let log = std::fs::File::create(&log_path).map_err(|e| fail(e.to_string()))?;
        let mut child = Command::new(&self.bin)
            .args(["-c", "-g", "-q", "-v1.00", "-i"])
            .arg(&self.include)
            .arg("decompiled.nss")
            .current_dir(&self.work)
            .stdin(Stdio::null())
            .stdout(log.try_clone().map_err(|e| fail(e.to_string()))?)
            .stderr(log)
            .spawn()
            .map_err(|e| fail(format!("could not start `{}`: {e}", self.bin.display())))?;
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) if start.elapsed() > self.timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(fail(format!(
                        "the compiler did not finish in {}s (nwnnsscomp never returns on the shipped nwscript.nss; use a copy with line 5145 corrected)",
                        self.timeout.as_secs()
                    )));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(e) => return Err(fail(e.to_string())),
            }
        };
        match std::fs::read(&out) {
            Ok(b) if status.success() => Ok(b),
            _ => {
                let text = std::fs::read_to_string(&log_path).unwrap_or_default();
                let first = text
                    .lines()
                    .find(|l| l.contains("Error"))
                    .unwrap_or("the compiler wrote no bytecode");
                // `decompiled.nss(12): Error: ...`
                let line = first
                    .strip_prefix("decompiled.nss(")
                    .and_then(|r| r.split(')').next())
                    .and_then(|n| n.parse().ok());
                let message = first
                    .split_once(": ")
                    .filter(|_| line.is_some())
                    .map_or(first, |(_, m)| m)
                    .to_string();
                Err(CompileError { message, line })
            }
        }
    }
}

impl Drop for Nwnnsscomp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.work);
    }
}

pub fn run(ctx: &Ctx, args: &Args) -> Exit {
    crate::working_on(display_name(&args.input));
    let (bytes, input) = match read_input(&args.input, "bytecode") {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            return Exit::Usage;
        }
    };
    let mut env = Envelope::new("decompile", ctx.game);
    env.inputs.push(input);
    let dir = args
        .input
        .parent()
        .filter(|_| args.input.as_os_str() != "-")
        .map(|d| {
            if d.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                d.to_path_buf()
            }
        });
    let mut resolver = ctx.resolver(dir);
    let mut table_map = SourceMap::new();
    let table = match nwscript::engine::load_table(&mut resolver, &mut table_map) {
        Ok(Some((t, _))) => t,
        Ok(None) => {
            eprintln!(
                "error: decompiling needs nwscript.nss for the engine routines: pass -I DIR or --nwscript FILE"
            );
            return Exit::Usage;
        }
        Err(e) => {
            eprintln!("error: reading nwscript.nss: {e}");
            return Exit::Errors;
        }
    };
    let nwscript_path = nwscript::Resolver::inputs(&resolver)
        .iter()
        .find(|i| i.role == "nwscript")
        .map(|i| PathBuf::from(&i.path));
    env.inputs
        .extend(nwscript::Resolver::inputs(&resolver).iter().cloned());

    let mut verifier = None;
    if !args.no_verify {
        let Some(bin) = args.compiler.clone() else {
            eprintln!(
                "error: verification needs a compiler: pass --compiler PATH (nwnnsscomp) or set NWNNSSCOMP, or skip it with --no-verify"
            );
            return Exit::Usage;
        };
        let include = nwscript_path
            .as_deref()
            .and_then(Path::parent)
            .map(|p| {
                if p.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    p
                }
            })
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let include = std::fs::canonicalize(&include).unwrap_or(include);
        verifier = Some(Nwnnsscomp {
            bin,
            include,
            work: std::env::temp_dir().join(format!("nws-decompile-{}", std::process::id())),
            timeout: Duration::from_secs(args.timeout),
        });
    }

    let mut map = SourceMap::new();
    let name = display_name(&args.input);
    let made = nwscript::decompile::decompile(
        &name,
        &bytes,
        &table,
        verifier.as_mut().map(|v| v as &mut dyn Compiler),
        &mut map,
    );
    drop(verifier);
    let (d, warnings) = match made {
        Ok(x) => x,
        Err(diag) => return ctx.finish(env, "", vec![diag], &map, Exit::Ok),
    };
    let source = d.source();
    let functions: Vec<serde_json::Value> = d
        .functions
        .iter()
        .map(|f| {
            let mut v = serde_json::json!({
                "name": f.name,
                "status": f.status.name(),
                "offset": f.start,
                "end": f.end,
            });
            match &f.status {
                Status::Unverified { at, detail } => {
                    if let Some(at) = at {
                        v["differs_at"] = (*at).into();
                    }
                    if !detail.is_empty() {
                        v["detail"] = detail.clone().into();
                    }
                }
                Status::NotRecovered { reason } => v["reason"] = reason.clone().into(),
                _ => {}
            }
            v
        })
        .collect();
    let verified = d.verified();
    let all = verified == d.functions.len();
    env.result = serde_json::json!({ "functions": functions, "verified": verified });
    let mut shown = String::new();
    match &args.output {
        Some(p) if p.as_os_str() != "-" => match write_output(p, &source) {
            Ok(rec) => env.outputs.push(rec),
            Err(e) => {
                eprintln!("error: {e}");
                return Exit::Errors;
            }
        },
        _ if ctx.out.format == Format::Json => {
            env.result["source"] = source
                .iter()
                .map(|&b| char::from(b))
                .collect::<String>()
                .into()
        }
        _ => shown = source.iter().map(|&b| char::from(b)).collect(),
    }
    let ok = if args.strict && !all {
        Exit::Different
    } else {
        Exit::Ok
    };
    ctx.finish(env, &shown, warnings, &map, ok)
}
