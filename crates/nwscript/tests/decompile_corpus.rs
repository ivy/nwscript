//! The decompiler's headline numbers: the fraction of functions whose
//! recovered source recompiles to the original bytes, over the vanilla
//! pairs and over the module scripts. The verifier is the project's
//! compiler, checking in the bioware mode, once against the shipped
//! declarations and once against BioWare's own (`Declarations::BioWare`);
//! `NWSCRIPT_DECOMP_VERIFIER=nwnnsscomp` verifies with nwnnsscomp `-v1.00`
//! instead (`$NWNNSSCOMP`, else modshop's build), for comparison.
//!
//! ```bash
//! cargo test -p nwscript --release --test decompile_corpus -- --ignored --nocapture
//! ```
//!
//! `NWSCRIPT_DECOMP_ONLY=k_act_*` limits a run to matching scripts,
//! `NWSCRIPT_DECOMP_DUMP=DIR` writes each recovered source there (as
//! verified against BioWare's declarations), and `NWSCRIPT_DECOMP_LIST=1`
//! names the scripts not verified whole.

#[path = "decompile/oracle.rs"]
mod common;

use nwscript::Declarations;
use nwscript::decompile::{Compiler, ProjectCompiler, Status};
use nwscript::ncs::Program;
use nwscript_decomp::decompile;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
/// The floors hold the project's compiler to its measurement; the
/// nwnnsscomp comparison is reported, not held.
///
/// The vanilla verified fraction, against BioWare's declarations, must not
/// fall below this. Measured: 4,023 of 4,029 functions; the others call
/// engine routines with arguments neither declaration set has (an older
/// ActionStartConversation, the Neverwinter Nights leftovers).
const VANILLA_FLOOR: f64 = 0.998;
/// The module verified fraction, against BioWare's declarations, must not
/// fall below this. Measured: 46,475 of 46,511 functions; the others call
/// engine routines with arguments neither declaration set has, or the
/// routine whose shipped declaration does not read (SetAvailableNPCId).
const MODULE_FLOOR: f64 = 0.999;

fn glob(pat: &str, s: &str) -> bool {
    fn go(p: &[u8], s: &[u8]) -> bool {
        match (p.first(), s.first()) {
            (None, None) => true,
            (Some(b'*'), _) => go(&p[1..], s) || (!s.is_empty() && go(p, &s[1..])),
            (Some(b'?'), Some(_)) => go(&p[1..], &s[1..]),
            (Some(a), Some(b)) if a == b => go(&p[1..], &s[1..]),
            _ => false,
        }
    }
    go(pat.as_bytes(), s.as_bytes())
}

/// nwnnsscomp's class per script from the committed conformance report:
/// where it could not reproduce BioWare's bytes from BioWare's own source.
fn oracle_classes() -> BTreeMap<String, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/report.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    // A dependency-free reading of `"NAME": {"class": "CLASS"` pairs under the first compiler.
    let mut out = BTreeMap::new();
    let start = text.find("\"nwnnsscomp-v100\"").unwrap_or(0);
    let section = &text[start..];
    let section = section[1..]
        .find("\"name\":")
        .map_or(section, |e| &section[..e]);
    let mut rest = section;
    while let Some(i) = rest.find("\"class\": \"") {
        let before = &rest[..i];
        let name = before
            .rfind("\": {")
            .and_then(|e| before[..e].rfind('"').map(|s| before[s + 1..e].to_string()));
        let after = &rest[i + 10..];
        let class = after.split('"').next().unwrap_or("").to_string();
        if let Some(n) = name {
            out.insert(n, class);
        }
        rest = after;
    }
    out
}

#[derive(Default, Clone)]
struct Tally {
    scripts: usize,
    whole: usize,
    functions: usize,
    verified: usize,
    unverified: usize,
    not_recovered: usize,
    /// Unverified functions whose bytecode calls an engine routine with
    /// other arguments than the shipped nwscript.nss declares.
    stale: usize,
    /// Functions in scripts nwnnsscomp could not reproduce from BioWare's source.
    oracle_limited: usize,
    oracle_limited_verified: usize,
    /// Scripts with no entry point the decompiler recognises.
    failed: usize,
    reasons: BTreeMap<String, usize>,
    unverified_scripts: Vec<String>,
}

impl Tally {
    fn fraction(&self) -> f64 {
        self.verified as f64 / self.functions.max(1) as f64
    }

    fn add(&mut self, o: &Tally) {
        self.scripts += o.scripts;
        self.whole += o.whole;
        self.functions += o.functions;
        self.verified += o.verified;
        self.unverified += o.unverified;
        self.not_recovered += o.not_recovered;
        self.stale += o.stale;
        self.oracle_limited += o.oracle_limited;
        self.oracle_limited_verified += o.oracle_limited_verified;
        self.failed += o.failed;
        for (r, n) in &o.reasons {
            *self.reasons.entry(r.clone()).or_default() += n;
        }
        self.unverified_scripts
            .extend(o.unverified_scripts.iter().cloned());
    }

    fn report(&self, what: &str) {
        println!(
            "{what}: {} scripts, {} functions, {} verified ({:.2}%), {} unverified, {} not recovered; {} scripts verified whole; {} scripts with no entry point",
            self.scripts,
            self.functions,
            self.verified,
            self.fraction() * 100.0,
            self.unverified,
            self.not_recovered,
            self.whole,
            self.failed
        );
        println!(
            "  unverified because the bytecode was compiled against older engine declarations: {}",
            self.stale
        );
        let mut reasons: Vec<(&String, &usize)> = self.reasons.iter().collect();
        reasons.sort_by(|a, b| b.1.cmp(a.1));
        for (r, n) in reasons.iter().take(30) {
            println!("  {n:6}  {r}");
        }
        if std::env::var_os("NWSCRIPT_DECOMP_LIST").is_some() {
            println!(
                "  not verified whole: {}",
                self.unverified_scripts.join(" ")
            );
        }
    }
}

/// One script to decompile: a name for reports and dumps, its bytes' path,
/// a group to tally it under, and whether nwnnsscomp fails to reproduce it.
struct Job {
    name: String,
    path: PathBuf,
    group: String,
    oracle_limited: bool,
}

/// How a run verifies: which declarations, and with what compiler.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Setup {
    Project(Declarations),
    Nwnnsscomp,
}

impl Setup {
    fn name(self) -> &'static str {
        match self {
            Setup::Project(Declarations::Shipped) => "project compiler, shipped declarations",
            Setup::Project(Declarations::BioWare) => "project compiler, BioWare's declarations",
            Setup::Nwnnsscomp => "nwnnsscomp -v1.00, shipped declarations",
        }
    }

    fn declarations(self) -> Declarations {
        match self {
            Setup::Project(d) => d,
            Setup::Nwnnsscomp => Declarations::Shipped,
        }
    }
}

/// The verifier runs use, from `NWSCRIPT_DECOMP_VERIFIER`.
fn setups() -> Option<(Vec<Setup>, Option<PathBuf>)> {
    if std::env::var("NWSCRIPT_DECOMP_VERIFIER").is_ok_and(|v| v == "nwnnsscomp") {
        let bin = common::nwnnsscomp()?;
        return Some((vec![Setup::Nwnnsscomp], Some(bin)));
    }
    Some((
        vec![
            Setup::Project(Declarations::Shipped),
            Setup::Project(Declarations::BioWare),
        ],
        None,
    ))
}

/// Decompile and verify every job in parallel under each setup. The
/// tallies are by group, one per setup and a last for functions verified
/// under any of them.
/// Each group's tallies: one per setup, then the union.
type Tallies = BTreeMap<String, Vec<Tally>>;

fn run(jobs: &[Job], corpus: &Path) -> Option<(Vec<Setup>, Tallies)> {
    let (setups, bin) = setups()?;
    let tables: Vec<_> = setups
        .iter()
        .map(|s| common::table(corpus, s.declarations()))
        .collect();
    let include = corpus.join("oracle");
    let dump = std::env::var_os("NWSCRIPT_DECOMP_DUMP").map(PathBuf::from);
    let tallies: Mutex<Tallies> = Mutex::new(BTreeMap::new());
    let next = AtomicUsize::new(0);
    // Tests run in parallel in one process: each run gets its own directory.
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    let work_root = std::env::temp_dir().join(format!(
        "nwscript-decomp-{}-{}",
        std::process::id(),
        RUNS.fetch_add(1, Ordering::Relaxed)
    ));
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for w in 0..workers {
            let (tallies, next, tables, include, dump, setups, bin) =
                (&tallies, &next, &tables, &include, &dump, &setups, &bin);
            let work = work_root.join(w.to_string());
            s.spawn(move || {
                let mut compilers: Vec<Box<dyn Compiler>> = setups
                    .iter()
                    .map(|s| -> Box<dyn Compiler> {
                        match *s {
                            Setup::Project(d) => Box::new(ProjectCompiler::new(
                                nwscript::FsResolver::new(vec![corpus.to_path_buf()], None),
                                nwscript::Game::K1,
                                d,
                            )),
                            Setup::Nwnnsscomp => {
                                let (bin, include, work) =
                                    (bin.clone().unwrap(), include.clone(), work.clone());
                                Box::new(move |src: &[u8]| {
                                    common::oracle_compile(&bin, &include, &work, src)
                                })
                            }
                        }
                    })
                    .collect();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = jobs.get(i) else { break };
                    let t = one(job, tables, &mut compilers, dump.as_deref());
                    let mut all = tallies.lock().unwrap();
                    let slot = all
                        .entry(job.group.clone())
                        .or_insert_with(|| vec![Tally::default(); t.len()]);
                    for (a, b) in slot.iter_mut().zip(&t) {
                        a.add(b);
                    }
                }
            });
        }
    });
    let _ = std::fs::remove_dir_all(&work_root);
    Some((setups, tallies.into_inner().unwrap()))
}

fn one(
    job: &Job,
    tables: &[nwscript::sema::EngineTable],
    compilers: &mut [Box<dyn Compiler>],
    dump: Option<&Path>,
) -> Vec<Tally> {
    let bytes = std::fs::read(&job.path).unwrap();
    let mut map = nwscript::diag::SourceMap::new();
    let program: Program = nwscript::ncs::decode_named(job.name.clone(), &bytes, &mut map).unwrap();
    let mut statuses: Vec<Vec<Status>> = Vec::new();
    for (k, (table, compiler)) in tables.iter().zip(compilers.iter_mut()).enumerate() {
        let mut d = match decompile(&program, table) {
            Ok(d) => d,
            Err(e) => {
                let mut t = Tally {
                    scripts: 1,
                    failed: 1,
                    ..Tally::default()
                };
                t.reasons.insert(format!("(script) {e}"), 1);
                t.unverified_scripts.push(job.name.clone());
                return vec![t; tables.len() + 1];
            }
        };
        d.verify(&program, &mut **compiler);
        let src = d.source();
        if k + 1 == tables.len()
            && let Some(dir) = dump
        {
            let path = dir.join(format!("{}.nss", job.name));
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            let _ = std::fs::write(path, &src);
        }
        let parsed = nwscript_syntax::parse(nwscript::diag::FileId(0), &src);
        assert!(
            parsed.diagnostics().is_empty(),
            "{}: the recovered source does not parse: {:?}",
            job.name,
            parsed.diagnostics().first().map(|d| &d.message)
        );
        statuses.push(d.functions.iter().map(|f| f.status.clone()).collect());
    }
    let mut out: Vec<Tally> = statuses.iter().map(|s| tally(job, s)).collect();
    // A function verified under any declaration set.
    let any: Vec<Status> = (0..statuses[0].len())
        .map(|i| {
            statuses
                .iter()
                .map(|s| &s[i])
                .find(|s| **s == Status::Verified)
                .unwrap_or(&statuses[statuses.len() - 1][i])
                .clone()
        })
        .collect();
    out.push(tally(job, &any));
    out
}

fn tally(job: &Job, statuses: &[Status]) -> Tally {
    let mut t = Tally {
        scripts: 1,
        ..Tally::default()
    };
    if statuses.iter().all(|s| *s == Status::Verified) {
        t.whole += 1;
    } else {
        t.unverified_scripts.push(job.name.clone());
    }
    for status in statuses {
        t.functions += 1;
        if job.oracle_limited {
            t.oracle_limited += 1;
        }
        match status {
            Status::Verified => {
                t.verified += 1;
                if job.oracle_limited {
                    t.oracle_limited_verified += 1;
                }
            }
            Status::Unverified { detail, .. } => {
                t.unverified += 1;
                if detail.contains("older declaration") {
                    t.stale += 1;
                }
                let key = if detail.is_empty() {
                    "(unverified) recompiles differently".to_string()
                } else {
                    format!(
                        "(unverified) {}",
                        detail.split(" at offset").next().unwrap_or(detail)
                    )
                };
                *t.reasons.entry(key).or_default() += 1;
            }
            Status::NotRecovered { reason } => {
                t.not_recovered += 1;
                let key = reason
                    .split(" at offset")
                    .next()
                    .unwrap_or(reason)
                    .to_string();
                *t.reasons.entry(key).or_default() += 1;
            }
            Status::Unchecked => {}
        }
    }
    t
}

fn only() -> Option<String> {
    std::env::var("NWSCRIPT_DECOMP_ONLY").ok()
}

/// Print each setup's tally and the union; return the last setup's.
fn report(what: &str, setups: &[Setup], tallies: &[Tally]) {
    for (s, t) in setups.iter().zip(tallies) {
        t.report(&format!("{what}, {}", s.name()));
    }
    if setups.len() > 1 {
        tallies[setups.len()].report(&format!("{what}, verified under either declaration set"));
    }
}

#[test]
#[ignore = "needs the corpus; run with --ignored"]
fn vanilla_verified_fraction() {
    let Some(corpus) = common::corpus() else {
        return;
    };
    let classes = oracle_classes();
    let only = only();
    let mut jobs: Vec<Job> = std::fs::read_dir(corpus.join("ncs"))
        .expect("ncs/")
        .filter_map(|e| {
            let p = e.ok()?.path();
            let stem = p.file_stem()?.to_str()?.to_string();
            (p.extension()? == "ncs" && corpus.join("nss").join(format!("{stem}.nss")).is_file())
                .then(|| Job {
                    oracle_limited: classes.get(&stem).is_some_and(|c| c != "identical"),
                    name: stem,
                    path: p,
                    group: "vanilla".into(),
                })
        })
        .filter(|j| only.as_deref().is_none_or(|g| glob(g, &j.name)))
        .collect();
    jobs.sort_by(|a, b| a.name.cmp(&b.name));
    let Some((setups, tallies)) = run(&jobs, &corpus) else {
        return;
    };
    let t = tallies.get("vanilla").cloned().unwrap_or_default();
    report("vanilla pairs", &setups, &t);
    let head = &t[setups.len() - 1];
    println!(
        "  in the scripts nwnnsscomp does not reproduce from BioWare's source: {} functions, {} verified",
        head.oracle_limited, head.oracle_limited_verified
    );
    if only.is_none() && setups.len() > 1 {
        assert!(
            head.fraction() >= VANILLA_FLOOR,
            "verified fraction {:.4} fell below the floor {VANILLA_FLOOR}",
            head.fraction()
        );
    }
}

#[test]
#[ignore = "needs the corpus; run with --ignored"]
fn module_verified_fraction() {
    let Some(corpus) = common::corpus() else {
        return;
    };
    let only = only();
    let mut jobs = Vec::new();
    for kind in ["modules", "rims"] {
        let Ok(dirs) = std::fs::read_dir(corpus.join(kind)) else {
            continue;
        };
        for d in dirs.filter_map(Result::ok) {
            let group = format!("{kind}/{}", d.file_name().to_string_lossy());
            for f in std::fs::read_dir(d.path())
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
            {
                let p = f.path();
                if p.extension().is_none_or(|e| e != "ncs") {
                    continue;
                }
                let stem = p.file_stem().unwrap().to_string_lossy().to_string();
                let name = format!("{group}/{stem}");
                if only.as_deref().is_some_and(|g| !glob(g, &name)) {
                    continue;
                }
                jobs.push(Job {
                    name,
                    path: p,
                    group: group.clone(),
                    oracle_limited: false,
                });
            }
        }
    }
    jobs.sort_by(|a, b| a.name.cmp(&b.name));
    let Some((setups, tallies)) = run(&jobs, &corpus) else {
        return;
    };
    let n = setups.len();
    let mut all = vec![Tally::default(); n + 1];
    for (g, t) in &tallies {
        let head = &t[n - 1];
        println!(
            "{g:24} {:5} scripts {:6} functions {:6} verified ({:.2}%)",
            head.scripts,
            head.functions,
            head.verified,
            head.fraction() * 100.0
        );
        for (a, b) in all.iter_mut().zip(t) {
            a.add(b);
        }
    }
    report("module scripts", &setups, &all);
    if only.is_none() && n > 1 {
        assert!(
            all[n - 1].fraction() >= MODULE_FLOOR,
            "verified fraction {:.4} fell below the floor {MODULE_FLOOR}",
            all[n - 1].fraction()
        );
    }
}

/// Small, structure-preserving damage to every vanilla script (an operand
/// nudged, two instructions swapped) is reported per function, never a panic.
#[test]
#[ignore = "needs the corpus; run with --ignored"]
fn damaged_bytecode_never_panics() {
    use nwscript::ncs::Operands as O;
    let Some(corpus) = common::corpus() else {
        return;
    };
    let table = common::table(&corpus, Declarations::Shipped);
    // xorshift: the same damage on every run.
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(corpus.join("ncs"))
        .expect("ncs/")
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut panics = Vec::new();
    for path in &files {
        let bytes = std::fs::read(path).unwrap();
        let program = nwscript::ncs::decode(nwscript::diag::FileId(0), &bytes).unwrap();
        for _ in 0..20 {
            let mut ins = program.instructions().to_vec();
            let n = ins.len();
            for _ in 0..1 + rnd() % 3 {
                let k = rnd() as usize % n;
                let step = |r: u64, unit: i32| unit * ((r % 5) as i32 - 2);
                match &mut ins[k].operands {
                    O::Stack { offset, size } => {
                        if rnd() % 2 == 0 {
                            *offset += step(rnd(), 4);
                        } else {
                            *size = (rnd() % 4) as u16 * 4;
                        }
                    }
                    O::Offset(o) => *o += step(rnd(), 4),
                    O::Jump(j) => *j += step(rnd(), 6),
                    O::Action { routine, argc } => {
                        if rnd() % 2 == 0 {
                            *routine = (rnd() % 800) as u16;
                        } else {
                            *argc = (rnd() % 6) as u8;
                        }
                    }
                    O::StoreState { locals, .. } => *locals = (rnd() % 10) as u32 * 4,
                    O::Destruct {
                        size,
                        keep_offset,
                        keep_size,
                    } => {
                        *size = (rnd() % 5) as u16 * 4;
                        *keep_offset = (rnd() % 5) as u16 * 4;
                        *keep_size = (rnd() % 3) as u16 * 4;
                    }
                    _ => ins.swap(k, rnd() as usize % n),
                }
            }
            let damaged = Program::new(ins);
            let ok = std::panic::catch_unwind(|| {
                if let Ok(d) = decompile(&damaged, &table) {
                    let _ = d.source();
                }
            });
            if ok.is_err() {
                panics.push(path.display().to_string());
            }
        }
    }
    assert!(panics.is_empty(), "panicked on damaged {panics:?}");
}
