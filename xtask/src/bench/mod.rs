//! `cargo xtask bench`: Rakata's correctness and speed per format against
//! other tools run as black boxes, the script compiler against the others
//! in `conformance/compilers.toml`, and `rakata install` against the other
//! installers. The comparators are argv templates in
//! `conformance/bench.toml`; the report is JSON and Markdown under
//! `target/bench/`.
//!
//! The harness is an xtask, not a `rakata` command: it measures this
//! checkout against tools and drivers that live beside it in
//! `conformance/`, which an installed `rakata` has no use for.

mod compile;
mod driver;
mod extract;
mod formats;
mod host;
mod install;
mod readers;
mod report;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const USAGE: &str = "\
usage: cargo xtask bench extract --game DIR [--out target/bench/files]
       cargo xtask bench run [OPTIONS]
       target/release/xtask bench driver MODE FORMAT LIST OUT

`extract` writes every vanilla file of each benchmarked format once, with a
list per format. `run` measures the comparators of conformance/bench.toml
over those lists, the compilers over the corpus, and the installers over
the install cases, and writes the report. `driver` is Rakata's reader as a
comparator; build it with `cargo build --release -p xtask`.

run options:
  --manifest FILE     the comparators (default conformance/bench.toml)
  --compilers FILE    the compiler definitions (default conformance/compilers.toml)
  --files DIR         the extracted lists (default target/bench/files)
  --corpus DIR        the scripts corpus (default $NWSCRIPT_CORPUS, then ~/.cache/nwscript/corpus/k1)
  --section NAME      readers, compile or install; repeatable (default: all three)
  --reader NAME       only these readers; repeatable
  --format NAME       only these formats; repeatable
  --compiler NAME     only these compilers; repeatable
  --installer NAME    only these installers; repeatable
  --case NAME         only these install cases; repeatable
  --reps N            timed repetitions of each measurement (default 5)
  --install-reps N    timed repetitions of each install (default 3)
  --max-files N       only the first N files of each list
  --max-scripts N     only the first N scripts of the corpus
  --timeout SECS      a process running longer is killed (default 1800)
  --label TEXT        what the run is, shown at the top of the report
  --report FILE       JSON report (default target/bench/report.json)
  --markdown FILE     Markdown report (default target/bench/report.md)
  --compare FILE      an earlier JSON report: say for each timing whether
                      this run's median lies within that run's range";

pub fn main(args: &[String]) -> Result<ExitCode, String> {
    match args.first().map(String::as_str) {
        Some("extract") => extract::main(&args[1..]).map(|()| ExitCode::SUCCESS),
        Some("run") => run(&args[1..]).map(|()| ExitCode::SUCCESS),
        Some("driver") => driver::main(&args[1..]),
        Some("-h" | "--help") => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        _ => Err(USAGE.to_string()),
    }
}

/// `conformance/bench.toml`.
#[derive(Deserialize, Default)]
pub struct BenchFile {
    #[serde(default)]
    pub reader: BTreeMap<String, ReaderDef>,
    #[serde(default)]
    pub compile: CompileDef,
    #[serde(default)]
    pub installer: BTreeMap<String, InstallerDef>,
    #[serde(default)]
    pub install: BTreeMap<String, InstallCase>,
}

/// A tool that reads (and perhaps writes) files of the benchmarked formats.
#[derive(Deserialize, Clone)]
pub struct ReaderDef {
    #[serde(default)]
    pub description: String,
    /// `{mode} {format} {list} {out}` for a batch driver; `{file}` for a
    /// tool run once per file.
    pub command: Vec<String>,
    /// Run once per file, success read from the exit status.
    #[serde(default)]
    pub per_file: bool,
    /// For a per-file tool: the command that starts it and does nothing.
    #[serde(default)]
    pub start: Vec<String>,
    /// For a per-file tool: the formats it is asked to read.
    #[serde(default)]
    pub formats: Vec<String>,
    #[serde(default)]
    pub identity: Vec<String>,
}

#[derive(Deserialize, Default)]
pub struct CompileDef {
    /// Names in the compiler definitions file.
    #[serde(default)]
    pub compilers: Vec<String>,
    /// Per compiler, a command that starts it and compiles nothing.
    #[serde(default)]
    pub start: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize, Clone)]
pub struct InstallerDef {
    #[serde(default)]
    pub description: String,
    /// `{data}` the tslpatchdata directory, `{game}` the tree to patch,
    /// `{option}` the 0-based option, `{log}` a directory for logs,
    /// `{option_args}` replaced by `option_args` for a mod with options
    /// and dropped otherwise.
    pub command: Vec<String>,
    #[serde(default)]
    pub option_args: Vec<String>,
    #[serde(default)]
    pub identity: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct InstallCase {
    #[serde(default)]
    pub description: String,
    /// The mod's archive, unpacked once under `target/bench/install/mods/`.
    pub archive: String,
    /// The `tslpatchdata` directory inside the unpacked archive.
    pub data: String,
    /// The option, 0-based in `[Namespaces]` order.
    #[serde(default)]
    pub option: Option<u32>,
    /// The game tree the mod is installed onto, copied for each run.
    pub tree: String,
}

/// Placeholder expansion shared by every section.
pub struct Subst {
    pub repo: PathBuf,
    pub corpus: PathBuf,
}

impl Subst {
    /// Expand `{repo}`, `{corpus}` and `{env:VAR}`; an unset variable is an
    /// error naming it.
    pub fn fixed(&self, s: &str) -> Result<String, String> {
        let mut out = s
            .replace("{repo}", &self.repo.display().to_string())
            .replace("{corpus}", &self.corpus.display().to_string());
        while let Some(i) = out.find("{env:") {
            let j = out[i..]
                .find('}')
                .map(|j| i + j)
                .ok_or_else(|| format!("unterminated {{env: in `{s}`"))?;
            let var = out[i + 5..j].to_string();
            let val = std::env::var(&var)
                .ok()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| format!("the environment variable {var} is unset"))?;
            out.replace_range(i..=j, &val);
        }
        Ok(out)
    }

    pub fn all(&self, argv: &[String]) -> Result<Vec<String>, String> {
        argv.iter().map(|a| self.fixed(a)).collect()
    }
}

/// A file that identifies a tool: its path as written and its hash.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Identity {
    pub path: String,
    pub sha256: Option<String>,
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

pub fn sha256_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(
        Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

/// The identity of a command's first element and of the files named,
/// recorded as written so a report names no machine.
pub fn identities(subst: &Subst, template: &[String], extra: &[String]) -> Vec<Identity> {
    let mut out = Vec::new();
    for written in template.iter().take(1).chain(extra) {
        let resolved = subst.fixed(written).ok().and_then(|p| which(&p));
        out.push(Identity {
            path: written.clone(),
            sha256: resolved.as_deref().and_then(sha256_file),
        });
    }
    out
}

pub struct RunOpts {
    pub manifest: PathBuf,
    pub compilers: PathBuf,
    pub files: PathBuf,
    pub corpus: PathBuf,
    pub sections: Vec<String>,
    pub readers: Vec<String>,
    pub formats: Vec<String>,
    pub compiler: Vec<String>,
    pub installers: Vec<String>,
    pub cases: Vec<String>,
    pub reps: usize,
    pub install_reps: usize,
    pub max_files: Option<usize>,
    pub max_scripts: Option<usize>,
    pub timeout: Duration,
    pub label: String,
    pub report: PathBuf,
    pub markdown: PathBuf,
    pub compare: Option<PathBuf>,
    /// GNU time, for peak memory.
    pub time: Option<PathBuf>,
    /// Where processes write their scratch files.
    pub work: PathBuf,
    /// The tree the files were extracted from, to show their paths within it.
    pub tree: Option<PathBuf>,
}

impl RunOpts {
    fn wants(list: &[String], name: &str) -> bool {
        list.is_empty() || list.iter().any(|n| n == name)
    }
}

fn default_corpus() -> PathBuf {
    if let Some(p) = std::env::var_os("NWSCRIPT_CORPUS") {
        return p.into();
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache/nwscript/corpus/k1")
}

fn parse(args: &[String]) -> Result<RunOpts, String> {
    let mut o = RunOpts {
        manifest: "conformance/bench.toml".into(),
        compilers: "conformance/compilers.toml".into(),
        files: "target/bench/files".into(),
        corpus: default_corpus(),
        sections: Vec::new(),
        readers: Vec::new(),
        formats: Vec::new(),
        compiler: Vec::new(),
        installers: Vec::new(),
        cases: Vec::new(),
        reps: 5,
        install_reps: 3,
        max_files: None,
        max_scripts: None,
        timeout: Duration::from_secs(1800),
        label: String::new(),
        report: "target/bench/report.json".into(),
        markdown: "target/bench/report.md".into(),
        compare: None,
        time: None,
        work: "target/bench/work".into(),
        tree: None,
    };
    let mut it = args.iter();
    let value = |it: &mut std::slice::Iter<'_, String>, flag: &str| {
        it.next()
            .cloned()
            .ok_or_else(|| format!("{flag} needs a value"))
    };
    while let Some(a) = it.next() {
        let num = |v: String| {
            v.parse::<usize>()
                .map_err(|_| format!("{a} needs a number, not `{v}`"))
        };
        match a.as_str() {
            "--manifest" => o.manifest = value(&mut it, a)?.into(),
            "--compilers" => o.compilers = value(&mut it, a)?.into(),
            "--files" => o.files = value(&mut it, a)?.into(),
            "--corpus" => o.corpus = value(&mut it, a)?.into(),
            "--section" => o.sections.push(value(&mut it, a)?),
            "--reader" => o.readers.push(value(&mut it, a)?),
            "--format" => o.formats.push(value(&mut it, a)?),
            "--compiler" => o.compiler.push(value(&mut it, a)?),
            "--installer" => o.installers.push(value(&mut it, a)?),
            "--case" => o.cases.push(value(&mut it, a)?),
            "--reps" => o.reps = num(value(&mut it, a)?)?.max(1),
            "--install-reps" => o.install_reps = num(value(&mut it, a)?)?.max(1),
            "--max-files" => o.max_files = Some(num(value(&mut it, a)?)?),
            "--max-scripts" => o.max_scripts = Some(num(value(&mut it, a)?)?),
            "--timeout" => o.timeout = Duration::from_secs(num(value(&mut it, a)?)? as u64),
            "--label" => o.label = value(&mut it, a)?,
            "--report" => o.report = value(&mut it, a)?.into(),
            "--markdown" => o.markdown = value(&mut it, a)?.into(),
            "--compare" => o.compare = Some(value(&mut it, a)?.into()),
            "-h" | "--help" => return Err(USAGE.to_string()),
            other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
        }
    }
    for s in &o.sections {
        if !["readers", "compile", "install"].contains(&s.as_str()) {
            return Err(format!(
                "no section `{s}`; the sections are readers, compile, install"
            ));
        }
    }
    for f in &o.formats {
        if formats::by_name(f).is_none() {
            let names: Vec<&str> = formats::ALL.iter().map(|f| f.name).collect();
            return Err(format!(
                "no format `{f}`; the formats are {}",
                names.join(", ")
            ));
        }
    }
    Ok(o)
}

fn run(args: &[String]) -> Result<(), String> {
    let mut opts = parse(args)?;
    let text = std::fs::read_to_string(&opts.manifest)
        .map_err(|e| format!("{}: {e}", opts.manifest.display()))?;
    let bench: BenchFile =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", opts.manifest.display()))?;
    let repo = std::env::current_dir().map_err(|e| e.to_string())?;
    let subst = Subst {
        repo: repo.clone(),
        corpus: opts.corpus.clone(),
    };
    opts.time = host::gnu_time();
    opts.work = repo.join(&opts.work);
    let files: Option<extract::Manifest> =
        std::fs::read_to_string(opts.files.join("manifest.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok());
    opts.tree = files.as_ref().map(|m| PathBuf::from(&m.tree));
    let mut report = report::Report {
        label: opts.label.clone(),
        manifest_sha256: Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        host: host::host(),
        method: report::Method {
            reps: opts.reps,
            install_reps: opts.install_reps,
            warmup: 1,
            max_files: opts.max_files,
            max_scripts: opts.max_scripts,
            peak_memory: match &opts.time {
                Some(_) => "GNU time %M (maximum resident set size) of each process".into(),
                None => "not measured: GNU time (/usr/bin/time) is not installed".into(),
            },
        },
        files,
        readers: Vec::new(),
        compile: Vec::new(),
        install: Vec::new(),
        compared: Vec::new(),
    };
    let section = |s: &str| RunOpts::wants(&opts.sections, s);
    if section("readers") {
        report.readers = readers::run(&bench, &subst, &opts);
    }
    if section("compile") {
        report.compile = compile::run(&bench, &subst, &opts);
    }
    if section("install") {
        report.install = install::run(&bench, &subst, &opts);
    }
    report.host.after = host::state();
    if let Some(old) = &opts.compare {
        let text = std::fs::read_to_string(old).map_err(|e| format!("{}: {e}", old.display()))?;
        let old: report::Report =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", old.display()))?;
        report.compared = report::compare(&old, &report);
    }
    for p in [&opts.report, &opts.markdown] {
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
    }
    std::fs::write(
        &opts.report,
        serde_json::to_string_pretty(&report).expect("report serializes") + "\n",
    )
    .map_err(|e| format!("{}: {e}", opts.report.display()))?;
    std::fs::write(&opts.markdown, report::markdown(&report))
        .map_err(|e| format!("{}: {e}", opts.markdown.display()))?;
    eprintln!(
        "report: {} and {}",
        opts.report.display(),
        opts.markdown.display()
    );
    Ok(())
}

/// A progress line on stderr.
pub fn progress(what: impl AsRef<str>) {
    eprintln!("bench: {}", what.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_expand_and_an_unset_variable_is_named() {
        let s = Subst {
            repo: "/r".into(),
            corpus: "/c".into(),
        };
        assert_eq!(s.fixed("{repo}/x {corpus}").unwrap(), "/r/x /c");
        let e = s
            .fixed("{env:BENCH_SURELY_UNSET_VARIABLE}/bin")
            .unwrap_err();
        assert!(e.contains("BENCH_SURELY_UNSET_VARIABLE"), "{e}");
    }

    #[test]
    fn the_manifest_parses() {
        let text = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../conformance/bench.toml"),
        )
        .unwrap();
        let b: BenchFile = toml::from_str(&text).unwrap();
        assert!(b.reader.contains_key("rakata"));
        for (name, r) in &b.reader {
            assert!(
                !r.per_file || !r.start.is_empty(),
                "{name}: a per-file reader needs `start`"
            );
            for f in &r.formats {
                assert!(formats::by_name(f).is_some(), "{name}: no format `{f}`");
            }
        }
        for c in &b.compile.compilers {
            assert!(b.compile.start.contains_key(c), "no start command for {c}");
        }
        assert!(b.installer.contains_key("rakata"));
    }
}
