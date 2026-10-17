//! The decompiler's headline numbers: the fraction of functions whose
//! recovered source recompiles to the original bytes, with nwnnsscomp
//! `-v1.00` as the interim verifier, over the vanilla pairs and over the
//! module scripts.
//!
//! ```bash
//! cargo test -p nwscript-decomp --release --test corpus -- --ignored --nocapture
//! ```
//!
//! `NWSCRIPT_DECOMP_ONLY=k_act_*` limits a run to matching scripts,
//! `NWSCRIPT_DECOMP_DUMP=DIR` writes each recovered source there, and
//! `NWSCRIPT_DECOMP_LIST=1` names the scripts not verified whole.

mod common;

use nwscript_decomp::{Status, decompile};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The vanilla verified fraction must not fall below this.
const VANILLA_FLOOR: f64 = 0.0;
/// The module verified fraction must not fall below this.
const MODULE_FLOOR: f64 = 0.0;

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

/// Decompile and verify every job in parallel; tallies by group.
fn run(jobs: &[Job], corpus: &Path, bin: &Path) -> BTreeMap<String, Tally> {
    let table = common::table(corpus);
    let include = corpus.join("oracle");
    let dump = std::env::var_os("NWSCRIPT_DECOMP_DUMP").map(PathBuf::from);
    let tallies: Mutex<BTreeMap<String, Tally>> = Mutex::new(BTreeMap::new());
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
            let (tallies, next, table, include, dump) = (&tallies, &next, &table, &include, &dump);
            let work = work_root.join(w.to_string());
            s.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = jobs.get(i) else { break };
                    let t = one(job, table, bin, include, &work, dump.as_deref());
                    tallies
                        .lock()
                        .unwrap()
                        .entry(job.group.clone())
                        .or_default()
                        .add(&t);
                }
            });
        }
    });
    let _ = std::fs::remove_dir_all(&work_root);
    tallies.into_inner().unwrap()
}

fn one(
    job: &Job,
    table: &nwscript_sema::EngineTable,
    bin: &Path,
    include: &Path,
    work: &Path,
    dump: Option<&Path>,
) -> Tally {
    let mut t = Tally {
        scripts: 1,
        ..Tally::default()
    };
    let bytes = std::fs::read(&job.path).unwrap();
    let mut map = nwscript_diag::SourceMap::new();
    let program = nwscript_ncs::decode_named(job.name.clone(), &bytes, &mut map).unwrap();
    let mut d = match decompile(&program, table) {
        Ok(d) => d,
        Err(e) => {
            t.failed += 1;
            t.reasons.insert(format!("(script) {e}"), 1);
            t.unverified_scripts.push(job.name.clone());
            return t;
        }
    };
    let mut compiler = |src: &[u8]| common::oracle_compile(bin, include, work, src);
    d.verify(&program, &mut compiler);
    let src = d.source();
    if let Some(dir) = dump {
        let path = dir.join(format!("{}.nss", job.name));
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let _ = std::fs::write(path, &src);
    }
    let parsed = nwscript_syntax::parse(nwscript_diag::FileId(0), &src);
    assert!(
        parsed.diagnostics().is_empty(),
        "{}: the recovered source does not parse: {:?}",
        job.name,
        parsed.diagnostics().first().map(|d| &d.message)
    );
    if d.functions.iter().all(|f| f.status == Status::Verified) {
        t.whole += 1;
    } else {
        t.unverified_scripts.push(job.name.clone());
    }
    for f in &d.functions {
        t.functions += 1;
        if job.oracle_limited {
            t.oracle_limited += 1;
        }
        match &f.status {
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

#[test]
#[ignore = "needs the corpus and nwnnsscomp; run with --ignored"]
fn vanilla_verified_fraction() {
    let Some(corpus) = common::corpus() else {
        return;
    };
    let Some(bin) = common::nwnnsscomp() else {
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
    let tallies = run(&jobs, &corpus, &bin);
    let t = tallies.get("vanilla").cloned().unwrap_or_default();
    t.report("vanilla pairs");
    println!(
        "  in the scripts nwnnsscomp does not reproduce from BioWare's source: {} functions, {} verified",
        t.oracle_limited, t.oracle_limited_verified
    );
    assert!(
        t.fraction() >= VANILLA_FLOOR,
        "verified fraction {:.4} fell below the floor {VANILLA_FLOOR}",
        t.fraction()
    );
}

#[test]
#[ignore = "needs the corpus and nwnnsscomp; run with --ignored"]
fn module_verified_fraction() {
    let Some(corpus) = common::corpus() else {
        return;
    };
    let Some(bin) = common::nwnnsscomp() else {
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
    let tallies = run(&jobs, &corpus, &bin);
    let mut all = Tally::default();
    for (g, t) in &tallies {
        println!(
            "{g:24} {:5} scripts {:6} functions {:6} verified ({:.2}%)",
            t.scripts,
            t.functions,
            t.verified,
            t.fraction() * 100.0
        );
        all.add(t);
    }
    all.report("module scripts");
    assert!(
        all.fraction() >= MODULE_FLOOR,
        "verified fraction {:.4} fell below the floor {MODULE_FLOOR}",
        all.fraction()
    );
}
