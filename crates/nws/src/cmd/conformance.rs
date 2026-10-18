//! `nws conformance`: run compilers over the vanilla corpus and classify each
//! script's output against BioWare's bytecode.
//!
//! Each compiler is an argv template (see `conformance/compilers.toml`). Each
//! script is copied into its own work directory, the command runs there with
//! stdin closed and a timeout, and the output is classed as identical, the
//! same length with different bytes, a different length, or failed.

use super::Ctx;
use crate::Exit;
use nwscript::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  NWNNSSCOMP=~/bin/nwnnsscomp nws conformance --compiler nwnnsscomp-v100
  nws conformance --compiler mine='mycc -o {out} {in}' --script 'k_act_*'
  nws conformance --compiler nwnnsscomp-v100 --report r.json --markdown r.md --baseline old.json

Compilers come from --compilers (default conformance/compilers.toml) by
name, or are defined inline as NAME=TEMPLATE (whitespace-separated argv).
The corpus is extracted with `cargo xtask corpus extract`.
Exit 0 when the run completes; 3 when --baseline is given and a compiler
reproduces fewer scripts than it did there.")]
pub struct Args {
    /// The corpus root. Defaults to $NWSCRIPT_CORPUS, then ~/.cache/nwscript/corpus/k1.
    #[arg(long, value_name = "DIR")]
    pub corpus: Option<PathBuf>,
    /// The manifest naming the scripts and their expected hashes.
    #[arg(long, value_name = "FILE", default_value = "conformance/k1.toml")]
    pub manifest: PathBuf,
    /// The compiler definitions file.
    #[arg(
        long,
        value_name = "FILE",
        default_value = "conformance/compilers.toml"
    )]
    pub compilers: PathBuf,
    /// A compiler to run: a NAME from --compilers, or NAME=TEMPLATE. Repeatable.
    #[arg(long = "compiler", value_name = "NAME[=TEMPLATE]", required = true)]
    pub compiler: Vec<String>,
    /// Only scripts whose names match this glob (`*` and `?`).
    #[arg(long, value_name = "GLOB")]
    pub script: Option<String>,
    /// Write the report as JSON.
    #[arg(long, value_name = "FILE")]
    pub report: Option<PathBuf>,
    /// Write the report as markdown.
    #[arg(long, value_name = "FILE")]
    pub markdown: Option<PathBuf>,
    /// A previous JSON report; exit 3 if any compiler in both reproduces fewer scripts now.
    #[arg(long, value_name = "FILE")]
    pub baseline: Option<PathBuf>,
    /// Known causes of failures, shown beside each one.
    #[arg(long, value_name = "FILE", default_value = "conformance/known.toml")]
    pub known: PathBuf,
    /// Parallel compilations. Defaults to the number of CPUs.
    #[arg(long, value_name = "N")]
    pub jobs: Option<usize>,
    /// Skip running non-identical outputs to compare their traces.
    #[arg(long)]
    pub no_trace: bool,
    /// Seconds before a compilation is killed and counted as failed.
    #[arg(long, value_name = "SECS", default_value_t = 60)]
    pub timeout: u64,
}

#[derive(Deserialize)]
struct CompilersFile {
    compiler: BTreeMap<String, CompilerDef>,
}

#[derive(Deserialize, Clone)]
struct CompilerDef {
    /// The command as written, placeholders unexpanded, for the report.
    #[serde(skip)]
    template: Vec<String>,
    #[serde(default)]
    description: String,
    command: Vec<String>,
    #[serde(default = "default_include")]
    include: String,
    #[serde(default)]
    identity: Vec<String>,
}

fn default_include() -> String {
    "oracle".into()
}

#[derive(Deserialize, Default)]
struct KnownFile {
    #[serde(default)]
    cause: BTreeMap<String, Cause>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Cause {
    pub summary: String,
    pub scripts: Vec<String>,
}

#[derive(Deserialize)]
struct Manifest {
    scripts: BTreeMap<String, ManifestHashes>,
}

#[derive(Deserialize)]
struct ManifestHashes {
    nss: Option<String>,
    ncs: Option<String>,
}

/// How one script's output compares with BioWare's.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Class {
    Identical,
    Samelen,
    Difflen,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScriptResult {
    pub class: Class,
    /// Output size against the expected size, for non-identical output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<i64>,
    /// Why a compilation failed: the first error line, or the timeout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The established cause, from the known-causes file, for output that
    /// is not identical.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    /// Behavioural comparison under the interpreter, for non-identical output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Counts {
    pub identical: usize,
    pub samelen: usize,
    pub difflen: usize,
    pub failed: usize,
    /// Of the outputs that are not identical: same calls, result and fault as BioWare's.
    #[serde(default)]
    pub trace_equal: usize,
    #[serde(default)]
    pub trace_different: usize,
    #[serde(default)]
    pub not_runnable: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Identity {
    pub path: String,
    pub sha256: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompilerReport {
    pub name: String,
    pub description: String,
    pub command: Vec<String>,
    pub identity: Vec<Identity>,
    pub counts: Counts,
    pub scripts: BTreeMap<String, ScriptResult>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Report {
    pub manifest: String,
    pub manifest_sha256: String,
    pub scripts: usize,
    pub causes: BTreeMap<String, Cause>,
    pub compilers: Vec<CompilerReport>,
}

fn default_corpus() -> PathBuf {
    if let Some(p) = std::env::var_os("NWSCRIPT_CORPUS") {
        return p.into();
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache/nwscript/corpus/k1")
}

fn glob_match(pat: &str, s: &str) -> bool {
    let (p, s): (Vec<char>, Vec<char>) = (pat.chars().collect(), s.chars().collect());
    fn go(p: &[char], s: &[char]) -> bool {
        match p.first() {
            None => s.is_empty(),
            Some('*') => go(&p[1..], s) || (!s.is_empty() && go(p, &s[1..])),
            Some('?') => !s.is_empty() && go(&p[1..], &s[1..]),
            Some(c) => s.first() == Some(c) && go(&p[1..], &s[1..]),
        }
    }
    go(&p, &s)
}

struct Subst<'a> {
    repo: &'a Path,
    corpus: &'a Path,
}

impl Subst<'_> {
    /// Expand `{env:VAR}`, `{repo}` and `{corpus}`; script placeholders are left.
    fn fixed(&self, s: &str) -> Result<String, String> {
        let mut out = s
            .replace("{repo}", &self.repo.display().to_string())
            .replace("{corpus}", &self.corpus.display().to_string());
        while let Some(i) = out.find("{env:") {
            let j = out[i..]
                .find('}')
                .map(|j| i + j)
                .ok_or_else(|| format!("unterminated {{env: in `{s}`"))?;
            let var = &out[i + 5..j];
            let val = std::env::var(var)
                .map_err(|_| format!("`{s}` needs the environment variable {var}"))?;
            out.replace_range(i..=j, &val);
        }
        Ok(out)
    }
}

fn which(cmd: &str) -> Option<PathBuf> {
    let p = Path::new(cmd);
    if p.components().count() > 1 {
        return p.is_file().then(|| p.to_path_buf());
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join(cmd))
        .find(|c| c.is_file())
}

fn identity(written: &str, path: &str) -> Identity {
    let resolved = which(path);
    Identity {
        path: written.to_string(),
        sha256: resolved
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| sha256_hex(&b)),
    }
}

/// The first line of compiler output that reads like an error, with the
/// work directory removed so reports compare across runs.
fn error_line(output: &str, work: &Path) -> Option<String> {
    let work = work.display().to_string();
    let lines: Vec<&str> = output
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let pick = lines
        .iter()
        .find(|l| l.to_ascii_lowercase().contains("error"))
        .or_else(|| lines.first())?;
    Some(pick.replace(&format!("{work}/"), "").replace(&work, "."))
}

struct Job<'a> {
    name: &'a str,
    expected: &'a [u8],
}

/// How a non-identical output behaves next to BioWare's: both run under the
/// recording engine and their calls, results and faults are compared.
fn trace_class(expected: &[u8], produced: &[u8], routines: &[nwscript::vm::Routine]) -> String {
    use nwscript::snapshot::CallRecord;
    use nwscript::vm::{Limits, Recording, run};
    // Behaviour does not depend on the program size record, and some compilers
    // write it differently, so it is set to the file's length before decoding.
    let decode = |b: &[u8]| {
        let mut b = b.to_vec();
        if b.len() >= 13 {
            let len = (b.len() as u32).to_be_bytes();
            b[9..13].copy_from_slice(&len);
        }
        nwscript::ncs::decode(nwscript::diag::FileId(0), &b)
    };
    let (Ok(want), Ok(have)) = (decode(expected), decode(produced)) else {
        return "not-runnable: the output does not decode".into();
    };
    let a = run(&want, routines, &mut Recording, Limits::default());
    let b = run(&have, routines, &mut Recording, Limits::default());
    if let Some(f) = b.fault.as_ref().filter(|f| !f.kind.is_engine_dependent()) {
        return format!("not-runnable: {f}");
    }
    let calls = |r: &nwscript::vm::Run| r.trace.iter().map(CallRecord::from).collect::<Vec<_>>();
    let (ca, cb) = (calls(&a), calls(&b));
    if let Some(i) = ca.iter().zip(&cb).position(|(x, y)| x != y) {
        return format!(
            "different: call {}: BioWare's {}, this {}",
            i + 1,
            ca[i],
            cb[i]
        );
    }
    if ca.len() != cb.len() {
        return format!("different: {} calls, BioWare's {}", cb.len(), ca.len());
    }
    if a.result != b.result {
        return format!(
            "different: result {:?}, BioWare's {:?}",
            b.result.map(|v| v.to_string()),
            a.result.map(|v| v.to_string())
        );
    }
    if a.fault.map(|f| f.to_string()) != b.fault.map(|f| f.to_string()) {
        return "different: the runs stop differently".into();
    }
    "equal".into()
}

fn run_one(
    def: &[String],
    include: &Path,
    corpus: &Path,
    job: &Job,
    work_root: &Path,
    timeout: Duration,
    routines: Option<&[nwscript::vm::Routine]>,
) -> ScriptResult {
    let failed = |detail: String| ScriptResult {
        class: Class::Failed,
        delta: None,
        detail: Some(detail),
        cause: None,
        trace: None,
    };
    let work = work_root.join(job.name);
    let _ = std::fs::remove_dir_all(&work);
    if let Err(e) = std::fs::create_dir_all(&work) {
        return failed(format!("work directory: {e}"));
    }
    let src_name = format!("{}.nss", job.name);
    let out_name = format!("{}.ncs", job.name);
    if let Err(e) = std::fs::copy(corpus.join("nss").join(&src_name), work.join(&src_name)) {
        return failed(format!("copying source: {e}"));
    }
    let argv: Vec<String> = def
        .iter()
        .map(|a| {
            a.replace("{in}", &src_name)
                .replace("{out}", &out_name)
                .replace("{name}", job.name)
                .replace("{include}", &include.display().to_string())
        })
        .collect();
    let log_path = work.join("log");
    let log = match std::fs::File::create(&log_path) {
        Ok(f) => f,
        Err(e) => return failed(format!("log file: {e}")),
    };
    let child = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(&work)
        .stdin(Stdio::null())
        .stdout(log.try_clone().expect("clone log handle"))
        .stderr(log)
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return failed(format!("could not start `{}`: {e}", argv[0])),
    };
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => return failed(format!("waiting: {e}")),
        }
    };
    let output = std::fs::read(&log_path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let produced = std::fs::read(work.join(&out_name)).ok();
    let _ = std::fs::remove_dir_all(&work);
    let Some(status) = status else {
        return failed(format!("timed out after {}s", timeout.as_secs()));
    };
    match produced {
        Some(bytes) if status.success() => {
            let delta = bytes.len() as i64 - job.expected.len() as i64;
            let class = if bytes == job.expected {
                Class::Identical
            } else if delta == 0 {
                Class::Samelen
            } else {
                Class::Difflen
            };
            let trace = routines
                .filter(|_| class != Class::Identical)
                .map(|r| trace_class(job.expected, &bytes, r));
            ScriptResult {
                class,
                delta: (class != Class::Identical).then_some(delta),
                detail: None,
                cause: None,
                trace,
            }
        }
        _ => failed(
            error_line(&output, &work).unwrap_or_else(|| match status.code() {
                Some(0) => format!("exited 0 without writing {out_name}"),
                Some(c) => format!("exited {c} without writing {out_name}"),
                None => format!("{status} without writing {out_name}"),
            }),
        ),
    }
}

fn load_compilers(args: &Args, subst: &Subst) -> Result<Vec<(String, CompilerDef)>, String> {
    let file: Option<CompilersFile> = match std::fs::read_to_string(&args.compilers) {
        Ok(t) => {
            Some(toml::from_str(&t).map_err(|e| format!("{}: {e}", args.compilers.display()))?)
        }
        Err(_) => None,
    };
    let mut out = Vec::new();
    for spec in &args.compiler {
        let (name, mut def) = match spec.split_once('=') {
            Some((name, template)) => (
                name.to_string(),
                CompilerDef {
                    template: Vec::new(),
                    description: format!("inline: {template}"),
                    command: template.split_whitespace().map(String::from).collect(),
                    include: default_include(),
                    identity: Vec::new(),
                },
            ),
            None => {
                let def = file
                    .as_ref()
                    .and_then(|f| f.compiler.get(spec))
                    .ok_or_else(|| {
                        format!(
                            "no compiler `{spec}` in {}; define one inline as NAME=TEMPLATE",
                            args.compilers.display()
                        )
                    })?;
                (spec.clone(), def.clone())
            }
        };
        if def.command.is_empty() {
            return Err(format!("compiler `{name}` has an empty command"));
        }
        def.template = def.command.clone();
        def.command = def
            .command
            .iter()
            .map(|a| subst.fixed(a))
            .collect::<Result<_, _>>()?;
        out.push((name, def));
    }
    Ok(out)
}

pub fn run(ctx: &Ctx, args: &Args) -> Exit {
    match measure(args) {
        Ok(report) => {
            let md = markdown(&report);
            if let Some(p) = &args.report
                && let Err(e) = std::fs::write(
                    p,
                    serde_json::to_string_pretty(&report).expect("report serializes") + "\n",
                )
            {
                eprintln!("error: writing {}: {e}", p.display());
                return Exit::Errors;
            }
            if let Some(p) = &args.markdown
                && let Err(e) = std::fs::write(p, &md)
            {
                eprintln!("error: writing {}: {e}", p.display());
                return Exit::Errors;
            }
            let mut exit = Exit::Ok;
            if let Some(b) = &args.baseline {
                match regressions(&report, b) {
                    Ok(r) if r.is_empty() => {}
                    Ok(r) => {
                        for line in r {
                            eprintln!("regression: {line}");
                        }
                        exit = Exit::Different;
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        return Exit::Usage;
                    }
                }
            }
            let mut env = super::envelope("conformance", ctx.game);
            env.result = serde_json::to_value(&report).expect("report serializes");
            ctx.finish(
                env,
                &md,
                Vec::new(),
                &nwscript::diag::SourceMap::new(),
                exit,
            )
        }
        Err(e) => {
            eprintln!("error: {e}");
            Exit::Usage
        }
    }
}

fn measure(args: &Args) -> Result<Report, String> {
    let corpus = args.corpus.clone().unwrap_or_else(default_corpus);
    let corpus = corpus.canonicalize().map_err(|e| {
        format!(
            "corpus {}: {e}; extract it with `cargo xtask corpus extract`",
            corpus.display()
        )
    })?;
    let manifest_text = std::fs::read_to_string(&args.manifest)
        .map_err(|e| format!("manifest {}: {e}", args.manifest.display()))?;
    let manifest: Manifest = toml::from_str(&manifest_text)
        .map_err(|e| format!("manifest {}: {e}", args.manifest.display()))?;
    let repo = std::env::current_dir().map_err(|e| e.to_string())?;
    let subst = Subst {
        repo: &repo,
        corpus: &corpus,
    };
    let compilers = load_compilers(args, &subst)?;
    let known: KnownFile = match std::fs::read_to_string(&args.known) {
        Ok(t) => toml::from_str(&t).map_err(|e| format!("{}: {e}", args.known.display()))?,
        Err(_) => KnownFile::default(),
    };
    let cause_of = |script: &str| {
        known
            .cause
            .iter()
            .find(|(_, c)| c.scripts.iter().any(|s| s == script))
            .map(|(k, _)| k.clone())
    };

    let mut expected: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, h) in &manifest.scripts {
        let (Some(_), Some(ncs_hash)) = (&h.nss, &h.ncs) else {
            continue;
        };
        if args.script.as_deref().is_some_and(|g| !glob_match(g, name)) {
            continue;
        }
        let bytes = std::fs::read(corpus.join("ncs").join(format!("{name}.ncs")))
            .map_err(|e| format!("corpus ncs/{name}.ncs: {e}"))?;
        if &sha256_hex(&bytes) != ncs_hash {
            return Err(format!(
                "corpus ncs/{name}.ncs does not match the manifest; re-extract the corpus"
            ));
        }
        expected.push((name.clone(), bytes));
    }
    let jobs = args
        .jobs
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let work_root = std::env::temp_dir().join(format!("nws-conformance-{}", std::process::id()));
    let timeout = Duration::from_secs(args.timeout);

    // The engine table for trace comparison: the corpus's own nwscript.nss.
    let routines: Option<Vec<nwscript::vm::Routine>> = if args.no_trace {
        None
    } else {
        // A corpus without nwscript.nss is compared by bytes alone.
        std::fs::read(corpus.join("nwscript.nss"))
            .ok()
            .map(|bytes| {
                let (table, _) =
                    nwscript::sema::EngineTable::from_nwscript(nwscript::diag::FileId(0), &bytes);
                nwscript::engine::routines(&table)
            })
    };
    let mut reports = Vec::new();
    for (name, def) in compilers {
        crate::working_on(format!("compiler {name}"));
        let include = corpus.join(&def.include);
        let next = Mutex::new(expected.iter());
        let results = Mutex::new(BTreeMap::new());
        let root = work_root.join(&name);
        std::thread::scope(|s| {
            for _ in 0..jobs.max(1) {
                s.spawn(|| {
                    loop {
                        let Some((script, bytes)) = next.lock().expect("queue").next() else {
                            break;
                        };
                        let r = run_one(
                            &def.command,
                            &include,
                            &corpus,
                            &Job {
                                name: script,
                                expected: bytes,
                            },
                            &root,
                            timeout,
                            routines.as_deref(),
                        );
                        results.lock().expect("results").insert(script.clone(), r);
                    }
                });
            }
        });
        let _ = std::fs::remove_dir_all(&root);
        let mut scripts: BTreeMap<String, ScriptResult> = results.into_inner().expect("results");
        for (name, r) in scripts.iter_mut() {
            if r.class != Class::Identical {
                r.cause = cause_of(name);
            }
        }
        let mut counts = Counts::default();
        for r in scripts.values() {
            match r.trace.as_deref() {
                Some("equal") => counts.trace_equal += 1,
                Some(t) if t.starts_with("different") => counts.trace_different += 1,
                Some(_) => counts.not_runnable += 1,
                None => {}
            }
            match r.class {
                Class::Identical => counts.identical += 1,
                Class::Samelen => counts.samelen += 1,
                Class::Difflen => counts.difflen += 1,
                Class::Failed => counts.failed += 1,
            }
        }
        // Paths are recorded as written, so a committed report names no machine.
        let mut ids = vec![identity(&def.template[0], &def.command[0])];
        for p in &def.identity {
            ids.push(identity(p, &subst.fixed(p)?));
        }
        reports.push(CompilerReport {
            name,
            description: def.description,
            command: def.template,
            identity: ids,
            counts,
            scripts,
        });
    }
    let _ = std::fs::remove_dir_all(&work_root);
    Ok(Report {
        manifest: args.manifest.display().to_string(),
        manifest_sha256: sha256_hex(manifest_text.as_bytes()),
        scripts: expected.len(),
        causes: known.cause,
        compilers: reports,
    })
}

fn regressions(now: &Report, baseline: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(baseline)
        .map_err(|e| format!("baseline {}: {e}", baseline.display()))?;
    let then: Report =
        serde_json::from_str(&text).map_err(|e| format!("baseline {}: {e}", baseline.display()))?;
    let mut out = Vec::new();
    for c in &now.compilers {
        if let Some(b) = then.compilers.iter().find(|b| b.name == c.name) {
            if c.counts.identical < b.counts.identical {
                out.push(format!(
                    "{}: {} identical, {} in the baseline",
                    c.name, c.counts.identical, b.counts.identical
                ));
            }
            for (script, r) in &c.scripts {
                if r.class != Class::Identical
                    && b.scripts
                        .get(script)
                        .is_some_and(|x| x.class == Class::Identical)
                {
                    out.push(format!("{}: {script} was identical", c.name));
                }
            }
        }
    }
    Ok(out)
}

/// The report as markdown: a summary table, then each compiler's failures
/// and the size deltas of its different-length outputs.
pub fn markdown(r: &Report) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# Conformance\n\n{} scripts with both source and bytecode in `{}`.\n\n",
        r.scripts, r.manifest
    ));
    s.push_str("| Compiler | Identical | Same length, different | Different length | Failed |\n| --- | --- | --- | --- | --- |\n");
    for c in &r.compilers {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            c.name, c.counts.identical, c.counts.samelen, c.counts.difflen, c.counts.failed
        ));
    }
    for c in &r.compilers {
        s.push_str(&format!("\n## {}\n\n{}\n\n", c.name, c.description));
        let traced = c.counts.trace_equal + c.counts.trace_different + c.counts.not_runnable;
        if traced > 0 {
            s.push_str(&format!(
                "Of the {traced} outputs that differ from BioWare's bytes, {} make the same engine calls with the same arguments and return the same result under the recording engine, {} behave differently, and {} do not run.\n\n",
                c.counts.trace_equal, c.counts.trace_different, c.counts.not_runnable
            ));
        }
        let failed: Vec<_> = c
            .scripts
            .iter()
            .filter(|(_, r)| r.class == Class::Failed)
            .collect();
        if !failed.is_empty() {
            s.push_str(&format!("Failed ({}):\n\n", failed.len()));
            let mut by_cause: BTreeMap<Option<&str>, Vec<(&String, &ScriptResult)>> =
                BTreeMap::new();
            for (name, res) in failed {
                by_cause
                    .entry(res.cause.as_deref())
                    .or_default()
                    .push((name, res));
            }
            for (cause, list) in by_cause.iter().rev() {
                match cause.and_then(|c| r.causes.get(c).map(|k| (c, k))) {
                    Some((c, k)) => {
                        s.push_str(&format!("- **{c}** ({}): {}\n", list.len(), k.summary))
                    }
                    None => s.push_str(&format!("- **no known cause** ({})\n", list.len())),
                }
                for (name, res) in list {
                    s.push_str(&format!(
                        "  - `{name}`: {}\n",
                        res.detail.as_deref().unwrap_or("")
                    ));
                }
            }
            s.push('\n');
        }
        let differ: Vec<_> = c
            .scripts
            .iter()
            .filter(|(_, r)| matches!(r.class, Class::Difflen | Class::Samelen))
            .collect();
        if !differ.is_empty() {
            s.push_str(&format!(
                "Different output ({}), by cause:\n\n",
                differ.len()
            ));
            let mut by_cause: BTreeMap<Option<&str>, Vec<&str>> = BTreeMap::new();
            for (name, res) in differ {
                by_cause.entry(res.cause.as_deref()).or_default().push(name);
            }
            for (cause, list) in by_cause.iter().rev() {
                let names: Vec<String> = list.iter().map(|n| format!("`{n}`")).collect();
                match cause.and_then(|c| r.causes.get(c).map(|k| (c, k))) {
                    Some((c, k)) => s.push_str(&format!(
                        "- **{c}** ({}): {}\n  {}\n",
                        list.len(),
                        k.summary,
                        names.join(", ")
                    )),
                    None => s.push_str(&format!(
                        "- **no known cause** ({}): {}\n",
                        list.len(),
                        names.join(", ")
                    )),
                }
            }
            s.push('\n');
        }
        let mut deltas: BTreeMap<i64, Vec<&str>> = BTreeMap::new();
        for (name, r) in &c.scripts {
            if r.class == Class::Difflen || r.class == Class::Samelen {
                deltas.entry(r.delta.unwrap_or(0)).or_default().push(name);
            }
        }
        if !deltas.is_empty() {
            let mut rows: Vec<_> = deltas.into_iter().collect();
            rows.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
            s.push_str("Size differences, largest clusters first:\n\n| Bytes | Scripts | Examples |\n| --- | --- | --- |\n");
            for (delta, names) in rows {
                let ex: Vec<String> = names.iter().take(3).map(|n| format!("`{n}`")).collect();
                s.push_str(&format!(
                    "| {delta:+} | {} | {} |\n",
                    names.len(),
                    ex.join(", ")
                ));
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob_match("k_act_*", "k_act_com1"));
        assert!(glob_match("k_?ct*", "k_act"));
        assert!(!glob_match("k_act_*", "k_ai_master"));
    }

    #[test]
    fn error_lines_drop_the_work_directory() {
        let w = Path::new("/tmp/x/k_a");
        assert_eq!(
            error_line("Compiling\n/tmp/x/k_a/k_a.nss(3): Error: bad\n", w).as_deref(),
            Some("k_a.nss(3): Error: bad")
        );
    }
}
