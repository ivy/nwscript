//! The compile section: each compiler named in `conformance/bench.toml`
//! over the corpus scripts that ship as both source and bytecode, one
//! process per script as the compilers work, timed per pass.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use super::host::{self, Stats};
use super::report::CompileRow;
use super::{BenchFile, RunOpts, Subst, identities, progress};

#[derive(Deserialize)]
struct CompilersFile {
    compiler: BTreeMap<String, CompilerDef>,
}

#[derive(Deserialize, Clone)]
struct CompilerDef {
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

#[derive(Deserialize)]
struct Manifest {
    scripts: BTreeMap<String, Hashes>,
}

#[derive(Deserialize)]
struct Hashes {
    nss: Option<String>,
    ncs: Option<String>,
}

/// A script compiles in its own folder, as `nws conformance` runs it.
const SCRIPT_TIMEOUT: Duration = Duration::from_secs(120);

pub fn run(bench: &BenchFile, subst: &Subst, opts: &RunOpts) -> Vec<CompileRow> {
    let mut rows = Vec::new();
    let skipped_all = |why: String| -> Vec<CompileRow> {
        bench
            .compile
            .compilers
            .iter()
            .filter(|c| RunOpts::wants(&opts.compiler, c))
            .map(|c| CompileRow {
                compiler: c.clone(),
                skipped: Some(why.clone()),
                ..Default::default()
            })
            .collect()
    };
    let defs: CompilersFile = match fs::read_to_string(&opts.compilers)
        .map_err(|e| e.to_string())
        .and_then(|t| toml::from_str(&t).map_err(|e| e.to_string()))
    {
        Ok(d) => d,
        Err(e) => return skipped_all(format!("{}: {e}", opts.compilers.display())),
    };
    let manifest_path = Path::new("conformance/k1.toml");
    let manifest: Manifest = match fs::read_to_string(manifest_path)
        .map_err(|e| e.to_string())
        .and_then(|t| toml::from_str(&t).map_err(|e| e.to_string()))
    {
        Ok(m) => m,
        Err(e) => return skipped_all(format!("{}: {e}", manifest_path.display())),
    };
    if !opts.corpus.join("nss").is_dir() {
        return skipped_all(format!(
            "no corpus at {} (set NWSCRIPT_CORPUS; `rakata corpus extract`)",
            opts.corpus.display()
        ));
    }
    let scripts: Vec<(String, Vec<u8>)> = manifest
        .scripts
        .iter()
        .filter(|(_, h)| h.nss.is_some() && h.ncs.is_some())
        .take(opts.max_scripts.unwrap_or(usize::MAX))
        .filter_map(|(n, _)| {
            let ncs = fs::read(opts.corpus.join("ncs").join(format!("{n}.ncs"))).ok()?;
            Some((n.clone(), ncs))
        })
        .collect();

    for name in &bench.compile.compilers {
        if !RunOpts::wants(&opts.compiler, name) {
            continue;
        }
        let mut row = CompileRow {
            compiler: name.clone(),
            scripts: scripts.len(),
            ..Default::default()
        };
        let Some(def) = defs.compiler.get(name) else {
            row.skipped = Some(format!(
                "no compiler `{name}` in {}",
                opts.compilers.display()
            ));
            rows.push(row);
            continue;
        };
        row.description = def.description.clone();
        row.command = def.command.clone();
        row.identity = identities(subst, &def.command, &def.identity);
        let template = match subst.all(&def.command) {
            Ok(t) => t,
            Err(e) => {
                row.skipped = Some(e);
                rows.push(row);
                continue;
            }
        };
        if !Path::new(&template[0]).is_file() {
            row.skipped = Some(format!("`{}` does not exist", def.command[0]));
            rows.push(row);
            continue;
        }
        let start = bench
            .compile
            .start
            .get(name)
            .map(|s| subst.all(s))
            .transpose();
        let start = match start {
            Ok(s) => s,
            Err(e) => {
                row.skipped = Some(e);
                rows.push(row);
                continue;
            }
        };
        let include = opts.corpus.join(&def.include);
        let root = opts.work.join("compile").join(name);
        let _ = fs::remove_dir_all(&root);
        let mut jobs: Vec<(PathBuf, Vec<String>, PathBuf, &[u8])> = Vec::new();
        for (script, ncs) in &scripts {
            let dir = root.join(script);
            let src = format!("{script}.nss");
            let out = format!("{script}.ncs");
            if fs::create_dir_all(&dir).is_err()
                || fs::copy(opts.corpus.join("nss").join(&src), dir.join(&src)).is_err()
            {
                continue;
            }
            let argv: Vec<String> = template
                .iter()
                .map(|a| {
                    a.replace("{in}", &src)
                        .replace("{out}", &out)
                        .replace("{name}", script)
                        .replace("{include}", &include.display().to_string())
                })
                .collect();
            jobs.push((dir.clone(), argv, dir.join(out), ncs));
        }
        progress(format!("compiler {name} ({} scripts)", jobs.len()));
        let go = |dir: &Path, argv: &[String]| {
            host::run(
                argv,
                Some(dir),
                opts.time.as_deref(),
                &opts.work,
                SCRIPT_TIMEOUT,
            )
        };
        // Correctness, which also warms the cache.
        let mut rss = None::<u64>;
        for (dir, argv, out, ncs) in &jobs {
            let _ = fs::remove_file(out);
            let m = go(dir, argv);
            rss = rss.max(m.rss_kb);
            match fs::read(out) {
                Ok(b) if m.code == Some(0) && b == *ncs => row.identical += 1,
                Ok(_) if m.code == Some(0) => row.different += 1,
                _ => row.failed += 1,
            }
        }
        let (mut s, mut t) = (Vec::new(), Vec::new());
        for rep in 0..opts.reps {
            progress(format!("  {name} repetition {}/{}", rep + 1, opts.reps));
            if let Some(start) = &start {
                s.push(go(&root, start).wall);
            }
            let mut total = 0.0;
            for (dir, argv, _, _) in &jobs {
                total += go(dir, argv).wall;
            }
            t.push(total);
        }
        row.start = Stats::of(&s);
        row.total = Stats::of(&t);
        row.rss_mb = rss.map(|k| k as f64 / 1024.0);
        let _ = fs::remove_dir_all(&root);
        rows.push(row);
    }
    rows
}
