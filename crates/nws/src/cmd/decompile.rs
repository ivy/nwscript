//! `nws decompile`: source from bytecode, each function verified by
//! recompiling it.
//!
//! Verification compiles the recovered source with the project's compiler
//! (`nwscript::decompile::ProjectCompiler`). `--compiler PATH` verifies with
//! nwnnsscomp instead, run as a black box behind the same `Compiler`
//! interface: the source is written to a scratch directory and compiled
//! there with `-v1.00` against the directory that holds `nwscript.nss`.

use super::Ctx;
use crate::Exit;
use crate::cli::Format;
use crate::io::{display_name, read_input, write_output};
use nwscript::decompile::{CompileError, Compiler, Status};
use nwscript::diag::SourceMap;
use nwscript::sema::EngineTable;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  nws --game k1 -I scripts decompile k_act_mispaz.ncs             print the source
  nws --game k1 -I scripts decompile k_act_mispaz.ncs -o out.nss  write it to a file
  nws --game k1 --declarations bioware decompile x.ncs            BioWare's own declarations
  nws --game k1 decompile --strict x.ncs                          exit 3 unless every function verifies
  nws -I scripts decompile --no-verify x.ncs                      skip recompiling
  nws -I oracle decompile --compiler ~/bin/nwnnsscomp x.ncs       verify with nwnnsscomp
  nws --game k1 -I scripts decompile modules/ --markdown r.md     a report by directory

Each function is recompiled and compared with the original bytes; a comment
above it says `verified`, `unverified` with the first offset that differs, or
`not recovered` with the reason, and each function that is not verified is
also reported as warning E0602. Verification uses the project's compiler for
--game, checking in the bioware mode (no lint stops it); --declarations
bioware compiles against the declarations BioWare's own scripts were
compiled against, which their bytecode needs (and which the decompiler then
reads the bytecode with too). nwscript.nss comes from -I or --nwscript.

--compiler PATH verifies with nwnnsscomp instead, run with -v1.00 against
the directory holding nwscript.nss; it never returns on KotOR 1's shipped
nwscript.nss, so point it at a copy with line 5145 corrected (the corpus's
oracle/ directory). In JSON mode result.functions[] has each function's
name, status and offset, and result.verified the count.

Given a directory, every .ncs file under it is decompiled and verified and
nothing is written but a report: one row per directory holding scripts, with
its scripts, the scripts verified whole, and its functions by status
(verified, unverified, not recovered, unchecked). A script with no entry
point the decompiler recognises counts as failed. --format json puts the
rows in result.directories and the sums in result.total; --markdown FILE
also writes them as a table; --strict exits 3 unless every function
verifies.")]
pub struct Args {
    /// The NCS file, a directory of them for a report by directory, or - for stdin.
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
    /// Verify with this nwnnsscomp executable instead of the project's compiler.
    #[arg(long, value_name = "PATH")]
    pub compiler: Option<PathBuf>,
    /// Seconds before an nwnnsscomp compilation is abandoned.
    #[arg(long, value_name = "SECS", default_value_t = 30)]
    pub timeout: u64,
    /// With a directory: also write the report as a markdown table.
    #[arg(long, value_name = "FILE")]
    pub markdown: Option<PathBuf>,
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

/// The engine table, read with the declarations the bytecode is verified
/// against.
fn engine_table(ctx: &Ctx, resolver: &mut crate::sources::Sources) -> Result<EngineTable, Exit> {
    let mut table_map = SourceMap::new();
    let mut table = match nwscript::engine::load_table(resolver, &mut table_map) {
        Ok(Some((t, _))) => t,
        Ok(None) => {
            if let Some(d) = resolver.missing_nwscript() {
                ctx.out
                    .emit(super::envelope("decompile", ctx.game), "", &[d], &table_map);
                return Err(Exit::Errors);
            }
            eprintln!(
                "error: decompiling needs nwscript.nss for the engine routines: pass -I DIR, --nwscript FILE or --game-dir DIR"
            );
            return Err(Exit::Usage);
        }
        Err(e) => {
            eprintln!("error: reading nwscript.nss: {e}");
            return Err(Exit::Errors);
        }
    };
    if let Some((name, text)) = ctx.game.and_then(|g| ctx.declarations().overlay(g)) {
        let file = table_map.add(name, text.as_bytes().to_vec());
        let _ = table.overlay(file, text.as_bytes());
    }
    let mut diags = Vec::new();
    for (name, bytes) in &ctx.overlays {
        let file = table_map.add(name.clone(), bytes.to_vec());
        diags.extend(table.overlay(file, bytes));
    }
    if nwscript::diag::has_errors(&diags) {
        ctx.out.emit(
            super::envelope("decompile", ctx.game),
            "",
            &diags,
            &table_map,
        );
        return Err(Exit::Errors);
    }
    Ok(table)
}

/// The compiler verification uses, or none with `--no-verify`. `work`
/// names nwnnsscomp's scratch directory, one per concurrent verifier.
fn verifier(
    ctx: &Ctx,
    args: &Args,
    dir: Option<PathBuf>,
    nwscript_path: Option<&Path>,
    work: &str,
) -> Result<Option<Box<dyn Compiler>>, Exit> {
    if args.no_verify {
        return Ok(None);
    }
    let Some(bin) = args.compiler.clone() else {
        let Some(game) = ctx.game else {
            eprintln!(
                "error: verification compiles for a game: pass --game k1 or --game k2, or skip it with --no-verify"
            );
            return Err(Exit::Usage);
        };
        return Ok(Some(Box::new(
            nwscript::decompile::ProjectCompiler::with_checker(
                ctx.resolver(dir),
                game,
                ctx.checker(game),
            ),
        )));
    };
    let include = nwscript_path
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
    Ok(Some(Box::new(Nwnnsscomp {
        bin,
        include,
        work: std::env::temp_dir().join(format!("nws-decompile-{}-{work}", std::process::id())),
        timeout: Duration::from_secs(args.timeout),
    })))
}

/// The `nwscript.nss` read from a directory, which an external compiler is
/// pointed at; one read from an install's archive is no file to point at.
fn nwscript_path(resolver: &crate::sources::Sources) -> Option<PathBuf> {
    resolver.nwscript_file()
}

pub fn run(ctx: &Ctx, args: &Args) -> Exit {
    if args.input.is_dir() {
        return report(ctx, args);
    }
    if args.markdown.is_some() {
        eprintln!("error: --markdown writes a report, which takes a directory");
        return Exit::Usage;
    }
    crate::working_on(display_name(&args.input));
    let (bytes, input) = match read_input(&args.input, "bytecode") {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            return Exit::Usage;
        }
    };
    let mut env = super::envelope("decompile", ctx.game);
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
    let mut resolver = ctx.resolver(dir.clone());
    let table = match engine_table(ctx, &mut resolver) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let nwscript_path = nwscript_path(&resolver);
    env.inputs
        .extend(nwscript::Resolver::inputs(&resolver).iter().cloned());
    let mut verifier = match verifier(ctx, args, dir, nwscript_path.as_deref(), "0") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let mut map = SourceMap::new();
    let name = display_name(&args.input);
    let made = nwscript::decompile::decompile(
        &name,
        &bytes,
        &table,
        verifier.as_mut().map(|v| &mut **v as &mut dyn Compiler),
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

/// One directory's functions by status.
#[derive(Clone, Default, Debug, serde::Serialize)]
struct Tally {
    scripts: usize,
    /// Scripts whose every function verified.
    whole: usize,
    /// Scripts with no entry point the decompiler recognises.
    failed: usize,
    functions: usize,
    verified: usize,
    unverified: usize,
    not_recovered: usize,
    unchecked: usize,
}

impl Tally {
    fn add(&mut self, o: &Tally) {
        self.scripts += o.scripts;
        self.whole += o.whole;
        self.failed += o.failed;
        self.functions += o.functions;
        self.verified += o.verified;
        self.unverified += o.unverified;
        self.not_recovered += o.not_recovered;
        self.unchecked += o.unchecked;
    }

    fn percent(&self) -> String {
        if self.functions == 0 {
            "-".into()
        } else {
            format!(
                "{:.2}%",
                self.verified as f64 * 100.0 / self.functions as f64
            )
        }
    }
}

/// Every `.ncs` under `root`, sorted, each with the directory it is
/// tallied under, relative to `root` and `/`-separated.
fn scripts(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ncs")) {
                let rel = d.strip_prefix(root).unwrap_or(&d);
                let group = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push((if group.is_empty() { ".".into() } else { group }, p));
            }
        }
    }
    out.sort();
    out
}

fn one(path: &Path, table: &EngineTable, verifier: Option<&mut dyn Compiler>) -> Tally {
    let mut t = Tally {
        scripts: 1,
        ..Tally::default()
    };
    let Ok(bytes) = std::fs::read(path) else {
        t.failed = 1;
        return t;
    };
    let mut map = SourceMap::new();
    let name = path.display().to_string();
    let Ok((d, _)) = nwscript::decompile::decompile(&name, &bytes, table, verifier, &mut map)
    else {
        t.failed = 1;
        return t;
    };
    for f in &d.functions {
        t.functions += 1;
        match f.status {
            Status::Verified => t.verified += 1,
            Status::Unverified { .. } => t.unverified += 1,
            Status::NotRecovered { .. } => t.not_recovered += 1,
            Status::Unchecked => t.unchecked += 1,
        }
    }
    if t.verified == t.functions {
        t.whole = 1;
    }
    t
}

fn markdown(rows: &BTreeMap<String, Tally>, total: &Tally) -> String {
    let mut s = String::from(
        "| Directory | Scripts | Verified whole | Failed | Functions | Verified | Unverified | Not recovered | Unchecked | Verified fraction |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    let mut row = |name: &str, t: &Tally| {
        s.push_str(&format!(
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            t.scripts,
            t.whole,
            t.failed,
            t.functions,
            t.verified,
            t.unverified,
            t.not_recovered,
            t.unchecked,
            t.percent()
        ));
    };
    for (g, t) in rows {
        row(&format!("`{g}`"), t);
    }
    row("**total**", total);
    s
}

/// A directory of bytecode: decompile and verify each script in parallel,
/// and report the functions by status for each directory.
fn report(ctx: &Ctx, args: &Args) -> Exit {
    if args.output.is_some() {
        eprintln!(
            "error: a directory is reported on, not decompiled to a file; -o takes a single NCS file"
        );
        return Exit::Usage;
    }
    let root = &args.input;
    let mut env = super::envelope("decompile", ctx.game);
    let mut resolver = ctx.resolver(None);
    let table = match engine_table(ctx, &mut resolver) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let nwscript_path = nwscript_path(&resolver);
    env.inputs
        .extend(nwscript::Resolver::inputs(&resolver).iter().cloned());
    // Fail on a bad command line before starting the workers.
    if let Err(e) = verifier(ctx, args, None, nwscript_path.as_deref(), "check") {
        return e;
    }
    let jobs = scripts(root);
    let rows: Mutex<BTreeMap<String, Tally>> = Mutex::new(BTreeMap::new());
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for w in 0..workers {
            let (rows, next, jobs, table, nwscript_path) =
                (&rows, &next, &jobs, &table, &nwscript_path);
            s.spawn(move || {
                let Ok(mut v) = verifier(ctx, args, None, nwscript_path.as_deref(), &w.to_string())
                else {
                    return;
                };
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((group, path)) = jobs.get(i) else {
                        break;
                    };
                    crate::working_on(display_name(path));
                    let t = one(
                        path,
                        table,
                        v.as_mut().map(|v| &mut **v as &mut dyn Compiler),
                    );
                    rows.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .entry(group.clone())
                        .or_default()
                        .add(&t);
                }
            });
        }
    });
    let rows = rows.into_inner().unwrap_or_else(|e| e.into_inner());
    let mut total = Tally::default();
    for t in rows.values() {
        total.add(t);
    }
    if let Some(p) = &args.markdown {
        match write_output(p, markdown(&rows, &total).as_bytes()) {
            Ok(rec) => env.outputs.push(rec),
            Err(e) => {
                eprintln!("error: {e}");
                return Exit::Errors;
            }
        }
    }
    env.result = serde_json::json!({
        "directories": rows
            .iter()
            .map(|(g, t)| {
                let mut v = serde_json::to_value(t).unwrap_or_default();
                v["directory"] = g.clone().into();
                v
            })
            .collect::<Vec<_>>(),
        "total": total,
    });
    let mut text = format!(
        "{:<28} {:>7} {:>7} {:>9} {:>9} {:>10}\n",
        "directory", "scripts", "whole", "functions", "verified", "fraction"
    );
    for (g, t) in rows
        .iter()
        .chain(std::iter::once((&"total".to_string(), &total)))
    {
        text.push_str(&format!(
            "{g:<28} {:>7} {:>7} {:>9} {:>9} {:>10}\n",
            t.scripts,
            t.whole,
            t.functions,
            t.verified,
            t.percent()
        ));
    }
    let ok = if args.strict && (total.verified != total.functions || total.failed > 0) {
        Exit::Different
    } else {
        Exit::Ok
    };
    ctx.finish(env, &text, Vec::new(), &SourceMap::new(), ok)
}
