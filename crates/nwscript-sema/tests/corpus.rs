//! Every vanilla script checked against the shipped `nwscript.nss`.
//!
//! The scripts with errors are exactly the ones BioWare compiled against
//! declarations that did not ship, each with only the codes its cause
//! explains. Every other script checks clean, and its checked program agrees
//! with BioWare's bytecode on which subroutines exist.

use nwscript_diag::{FileId, SourceMap};
use nwscript_ncs::cfg::{self, SubKind};
use nwscript_sema::hir::{Arg, Callee, EntryKind, ExprKind, Program};
use nwscript_sema::{EngineTable, Includes, analyze, visit};
use nwscript_syntax::{Parse, parse};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

fn corpus() -> Option<PathBuf> {
    let dir = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".cache/nwscript/corpus/k1"),
    };
    dir.join("nwscript.nss").is_file().then_some(dir)
}

/// Includes from the corpus's `nss/`, each parsed once.
struct Dir<'a> {
    root: PathBuf,
    map: &'a mut SourceMap,
    cache: HashMap<String, Parse>,
}

impl Includes for Dir<'_> {
    fn include(&mut self, name: &str) -> Result<Option<(String, Parse)>, String> {
        let key = name.to_ascii_lowercase();
        let shown = format!("{key}.nss");
        if let Some(p) = self.cache.get(&key) {
            return Ok(Some((shown, p.clone())));
        }
        let Ok(bytes) = std::fs::read(self.root.join(&shown)) else {
            return Ok(None);
        };
        let file = self.map.add(shown.clone(), bytes.clone());
        let p = parse(file, &bytes);
        self.cache.insert(key, p.clone());
        Ok(Some((shown, p)))
    }
}

/// The codes each cause in `conformance/known.toml` accounts for.
fn cause_codes(cause: &str) -> &'static [&'static str] {
    match cause {
        // Calls against changed engine prototypes, and k_inc_ebonhawk's
        // forward declarations: all codes BioWare's compiler accepted.
        "stale-prototype" => &["E0308", "E0309", "E0311"],
        // Names no shipped file declares.
        "undeclared" => &["E0303", "E0304"],
        _ => unreachable!(),
    }
}

/// The scripts with errors and their causes: the 18 of
/// `conformance/known.toml`, `k_inc_ebonhawk` itself, and `e3_scripts`, a
/// library that calls Neverwinter Nights functions and has no NCS.
const EXPECTED: &[(&str, &str)] = &[
    ("e3_scripts", "undeclared"),
    ("k_act_bastadd", "stale-prototype"),
    ("k_act_bastrmv", "stale-prototype"),
    ("k_act_canderrmv", "stale-prototype"),
    ("k_act_carthrmv", "stale-prototype"),
    ("k_act_hki47rmv", "stale-prototype"),
    ("k_act_joleermv", "stale-prototype"),
    ("k_act_juhanirmv", "stale-prototype"),
    ("k_act_makeitem", "stale-prototype"),
    ("k_act_missionrmv", "stale-prototype"),
    ("k_act_t3m3rmv", "stale-prototype"),
    ("k_act_zaalrmv", "stale-prototype"),
    ("k_con_makeitem", "stale-prototype"),
    ("k_inc_ebonhawk", "stale-prototype"),
    ("k_pebn_galaxy", "stale-prototype"),
    ("k_pend_door04", "undeclared"),
    ("k_sup_galaxmap", "stale-prototype"),
    ("k_sup_galaxymap", "stale-prototype"),
    ("nw_g0_conversat", "undeclared"),
    ("nw_s0_lghtnbolt", "undeclared"),
];

/// Scripts whose shipped NCS was compiled from other source than shipped:
/// `k_qa.ncs` holds a subroutine that adds each NPC by template, which
/// `k_qa.nss` does not have.
const STALE_NCS: &[&str] = &["k_qa"];

/// What must hold of a program that checked without errors.
fn assert_complete(name: &str, p: &Program, table: &EngineTable) {
    for f in &p.functions {
        if f.reachable {
            assert!(
                f.body.is_some(),
                "{name}: reachable `{}` has no body",
                f.name
            );
        }
        let Some(body) = &f.body else { continue };
        visit::block_exprs(body, &mut |e| match &e.kind {
            ExprKind::Error => panic!("{name}: an error expression in `{}`", f.name),
            ExprKind::Call(c) => {
                let n = match c.callee {
                    Callee::Engine(r) => table.functions[usize::from(r)].params.len(),
                    Callee::User(u) => p.function(u).params.len(),
                };
                assert_eq!(
                    c.args.len(),
                    n,
                    "{name}: a call in `{}` without every argument",
                    f.name
                );
                for a in &c.args {
                    if let Arg::Action(x) = a {
                        assert!(
                            matches!(x.kind, ExprKind::Call(_)),
                            "{name}: an action that is no call"
                        );
                    }
                }
            }
            _ => {}
        });
    }
}

#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn k1_vanilla_scripts() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let mut map = SourceMap::new();
    let ns = std::fs::read(root.join("nwscript.nss")).unwrap();
    let nsf = map.add("nwscript.nss", ns.clone());
    let (table, _) = EngineTable::from_nwscript(nsf, &ns);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(root.join("nss"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap() != "nwscript.nss")
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 1773);
    let mut dir = Dir {
        root: root.join("nss"),
        map: &mut map,
        cache: HashMap::new(),
    };
    let mut failed: BTreeMap<String, Vec<(&'static str, String)>> = BTreeMap::new();
    let mut entries: BTreeMap<&str, usize> = BTreeMap::new();
    let mut subs_disagree = Vec::new();
    let mut compared = 0;
    for path in &paths {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let bytes = std::fs::read(path).unwrap();
        let shown = format!("{stem}.nss");
        let file = dir.map.add(shown.clone(), bytes.clone());
        let a = analyze(&shown, parse(file, &bytes), &table, &mut dir);
        *entries
            .entry(a.program.entry.map_or("none", |e| e.kind.name()))
            .or_default() += 1;
        let errs: Vec<(&'static str, String)> = a
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(|d| (d.code.id, d.message.clone()))
            .collect();
        if !errs.is_empty() {
            failed.insert(stem, errs);
            continue;
        }
        assert_complete(&stem, &a.program, &table);
        // BioWare's compiler emits a subroutine for each function the entry
        // point reaches, and one more that initialises the globals when the
        // program declares any.
        let Ok(ncs) = std::fs::read(root.join("ncs").join(format!("{stem}.ncs"))) else {
            continue;
        };
        if a.program.entry.is_none() {
            continue;
        }
        let prog = nwscript_ncs::decode(FileId(u32::MAX), &ncs).unwrap();
        let calls = cfg::build(&prog)
            .subs
            .iter()
            .filter(|s| s.kind == SubKind::Call)
            .count();
        let want =
            a.program.reachable_functions().len() + usize::from(!a.program.globals.is_empty());
        compared += 1;
        if calls != want {
            subs_disagree.push(stem);
        }
    }
    println!("entry points: {entries:?}");
    println!("{compared} scripts compared with their NCS");
    for (f, errs) in &failed {
        println!("{f}:");
        for (c, m) in errs {
            println!("    {c}: {m}");
        }
    }
    assert_eq!(entries.get(EntryKind::Main.name()), Some(&767));
    assert_eq!(
        entries.get(EntryKind::StartingConditional.name()),
        Some(&983)
    );
    assert_eq!(entries.get("none"), Some(&23));

    let got: Vec<&str> = failed.keys().map(String::as_str).collect();
    let want: Vec<&str> = EXPECTED.iter().map(|(n, _)| *n).collect();
    assert_eq!(got, want, "the scripts with errors changed");
    for (name, cause) in EXPECTED {
        for (code, msg) in &failed[*name] {
            assert!(
                cause_codes(cause).contains(code),
                "{name} ({cause}) has an error its cause does not explain: {code}: {msg}"
            );
        }
    }
    assert_eq!(
        subs_disagree, STALE_NCS,
        "reachable functions disagree with the NCS"
    );
    // 1,754 pairs less the 18 of known.toml and the five include files.
    assert_eq!(compared, 1731);
}
