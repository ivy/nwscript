//! The decompiler's headline number: over the vanilla pairs, the fraction
//! of functions whose recovered source recompiles (with nwnnsscomp
//! `-v1.00`, the interim verifier) to BioWare's bytes.
//!
//! ```bash
//! cargo test -p nwscript-decomp --release --test corpus -- --ignored --nocapture
//! ```
//!
//! `NWSCRIPT_DECOMP_ONLY=k_act_*` limits the run to matching scripts, and
//! `NWSCRIPT_DECOMP_DUMP=DIR` writes each recovered source there.

mod common;

use nwscript_decomp::{Status, decompile};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The verified fraction must not fall below this.
const FLOOR: f64 = 0.0;

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

#[derive(Default)]
struct Tally {
    scripts: usize,
    whole: usize,
    functions: usize,
    verified: usize,
    unverified: usize,
    not_recovered: usize,
    /// Functions in scripts nwnnsscomp could not reproduce from BioWare's source.
    oracle_limited: usize,
    oracle_limited_verified: usize,
    failed: usize,
    reasons: BTreeMap<String, usize>,
    unverified_scripts: Vec<String>,
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
    let table = common::table(&corpus);
    let include = corpus.join("oracle");
    let only = std::env::var("NWSCRIPT_DECOMP_ONLY").ok();
    let dump = std::env::var_os("NWSCRIPT_DECOMP_DUMP").map(std::path::PathBuf::from);
    let classes = oracle_classes();
    let mut names: Vec<String> = std::fs::read_dir(corpus.join("ncs"))
        .expect("ncs/")
        .filter_map(|e| {
            let p = e.ok()?.path();
            let stem = p.file_stem()?.to_str()?.to_string();
            (p.extension()? == "ncs" && corpus.join("nss").join(format!("{stem}.nss")).is_file())
                .then_some(stem)
        })
        .filter(|n| only.as_deref().is_none_or(|g| glob(g, n)))
        .collect();
    names.sort();
    let tally = Mutex::new(Tally::default());
    let next = AtomicUsize::new(0);
    let work_root = std::env::temp_dir().join(format!("nwscript-decomp-{}", std::process::id()));
    let jobs = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for w in 0..jobs {
            let (names, tally, next, table, bin, include, corpus) =
                (&names, &tally, &next, &table, &bin, &include, &corpus);
            let (classes, dump) = (&classes, &dump);
            let work = work_root.join(w.to_string());
            s.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(name) = names.get(i) else { break };
                    let bytes =
                        std::fs::read(corpus.join("ncs").join(format!("{name}.ncs"))).unwrap();
                    let mut map = nwscript_diag::SourceMap::new();
                    let program =
                        nwscript_ncs::decode_named(name.clone(), &bytes, &mut map).unwrap();
                    let limited = classes.get(name).is_some_and(|c| c != "identical");
                    let mut d = match decompile(&program, table) {
                        Ok(d) => d,
                        Err(e) => {
                            let mut t = tally.lock().unwrap();
                            t.scripts += 1;
                            t.failed += 1;
                            *t.reasons.entry(format!("(script) {e}")).or_default() += 1;
                            continue;
                        }
                    };
                    let mut compiler =
                        |src: &[u8]| common::oracle_compile(bin, include, &work, src);
                    d.verify(&program, &mut compiler);
                    let src = d.source();
                    if let Some(dir) = dump {
                        let _ = std::fs::create_dir_all(dir);
                        let _ = std::fs::write(dir.join(format!("{name}.nss")), &src);
                    }
                    let parsed = nwscript_syntax::parse(nwscript_diag::FileId(0), &src);
                    assert!(
                        parsed.diagnostics().is_empty(),
                        "{name}: the recovered source does not parse: {:?}",
                        parsed.diagnostics().first().map(|d| &d.message)
                    );
                    let mut t = tally.lock().unwrap();
                    t.scripts += 1;
                    let all = d.functions.iter().all(|f| f.status == Status::Verified);
                    if all {
                        t.whole += 1;
                    } else {
                        t.unverified_scripts.push(name.clone());
                    }
                    for f in &d.functions {
                        t.functions += 1;
                        if limited {
                            t.oracle_limited += 1;
                        }
                        match &f.status {
                            Status::Verified => {
                                t.verified += 1;
                                if limited {
                                    t.oracle_limited_verified += 1;
                                }
                            }
                            Status::Unverified { .. } => t.unverified += 1,
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
                }
            });
        }
    });
    let _ = std::fs::remove_dir_all(&work_root);
    let t = tally.into_inner().unwrap();
    let frac = t.verified as f64 / t.functions.max(1) as f64;
    println!(
        "decompiled {} scripts: {} functions, {} verified ({:.2}%), {} unverified, {} not recovered; {} scripts verified whole; {} scripts with no entry point",
        t.scripts,
        t.functions,
        t.verified,
        frac * 100.0,
        t.unverified,
        t.not_recovered,
        t.whole,
        t.failed
    );
    println!(
        "in the scripts nwnnsscomp does not reproduce from BioWare's source: {} functions, {} verified",
        t.oracle_limited, t.oracle_limited_verified
    );
    let mut reasons: Vec<(&String, &usize)> = t.reasons.iter().collect();
    reasons.sort_by(|a, b| b.1.cmp(a.1));
    for (r, n) in reasons.iter().take(30) {
        println!("{n:6}  {r}");
    }
    if std::env::var_os("NWSCRIPT_DECOMP_LIST").is_some() {
        println!("not verified whole: {}", t.unverified_scripts.join(" "));
    }
    assert!(
        frac >= FLOOR,
        "verified fraction {frac:.4} fell below the floor {FLOOR}"
    );
}
