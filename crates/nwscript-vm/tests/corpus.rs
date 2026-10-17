//! Every corpus NCS under the recording engine: no machine faults, and the
//! same trace twice.

use nwscript_diag::FileId;
use nwscript_vm::{Limits, Recording, Routine, Ty, run};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn corpus() -> Option<PathBuf> {
    let dir = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".cache/nwscript/corpus/k1"),
    };
    dir.join("ncs").is_dir().then_some(dir)
}

/// The routine table from `nwscript.nss`, through the front end's reader.
fn routines(nwscript: &Path) -> Vec<Routine> {
    use nwscript_sema::Type;
    let ty = |t: &Type| match t {
        Type::Void => Ty::Void,
        Type::Int => Ty::Int,
        Type::Float => Ty::Float,
        Type::String => Ty::String,
        Type::Object => Ty::Object,
        Type::Vector => Ty::Vector,
        Type::Action => Ty::Action,
        Type::Engine(n) => Ty::Engine(*n),
        Type::Struct(_) => Ty::Int,
    };
    let (table, _) =
        nwscript_sema::EngineTable::from_nwscript(FileId(0), &std::fs::read(nwscript).unwrap());
    table
        .functions
        .iter()
        .map(|f| Routine {
            name: f.name.clone(),
            params: f.params.iter().map(|p| ty(&p.ty)).collect(),
            ret: ty(&f.ret),
        })
        .collect()
}

fn all_ncs(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.join("ncs"), root.join("modules"), root.join("rims")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "ncs") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn corpus_runs() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let table = routines(&root.join("nwscript.nss"));
    let files = all_ncs(&root);
    let mut faults: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let (mut clean, mut calls, mut deferred) = (0, 0usize, 0usize);
    for f in &files {
        let p = nwscript_ncs::decode(FileId(0), &std::fs::read(f).unwrap()).unwrap();
        let a = run(&p, &table, &mut Recording, Limits::default());
        let b = run(&p, &table, &mut Recording, Limits::default());
        assert_eq!(a, b, "{}: runs differ", f.display());
        calls += a.trace.len();
        deferred += a.trace.iter().filter(|c| c.deferred.is_some()).count();
        match &a.fault {
            None => clean += 1,
            Some(fault) => faults
                .entry(format!("{:?}", fault.kind))
                .or_default()
                .push(format!(
                    "{} {fault}",
                    f.strip_prefix(&root).unwrap().display()
                )),
        }
    }
    println!(
        "vm: {clean}/{} ran to completion, {calls} engine calls ({deferred} in deferred actions)",
        files.len()
    );
    let mut machine = Vec::new();
    for (k, v) in &faults {
        println!("  {k}: {}", v.len());
        for x in v.iter().take(if k.starts_with("ArgumentMismatch") {
            20
        } else {
            4
        }) {
            println!("    {x}");
        }
        if !(k.starts_with("DivisionByZero")
            || k.starts_with("Budget")
            || k.starts_with("ArgumentMismatch"))
        {
            machine.extend(v.iter().cloned());
        }
    }
    assert!(machine.is_empty(), "machine faults: {machine:#?}");
}

/// The world model covers at least 60% of the engine call sites in the
/// corpus, and running every script against an empty world faults no more
/// than the recording engine does.
#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn corpus_world_coverage() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let table = routines(&root.join("nwscript.nss"));
    let modelled: std::collections::HashSet<&str> =
        nwscript_vm::world::MODELS.iter().map(|m| m.0).collect();
    let (mut sites, mut covered, mut machine) = (0usize, 0usize, Vec::new());
    for f in all_ncs(&root) {
        let p = nwscript_ncs::decode(FileId(0), &std::fs::read(&f).unwrap()).unwrap();
        for i in p.instructions() {
            if let nwscript_ncs::Operands::Action { routine, .. } = i.operands {
                sites += 1;
                covered += usize::from(
                    table
                        .get(usize::from(routine))
                        .is_some_and(|r| modelled.contains(r.name.as_str())),
                );
            }
        }
        let r = run(
            &p,
            &table,
            &mut nwscript_vm::World::new(0),
            Limits::default(),
        );
        if let Some(fault) = &r.fault
            && !fault.kind.is_engine_dependent()
        {
            machine.push(format!("{} {fault}", f.display()));
        }
    }
    let pct = 100.0 * covered as f64 / sites as f64;
    println!("world: models {covered} of {sites} engine call sites ({pct:.1}%)");
    assert!(pct >= 60.0);
    assert!(machine.is_empty(), "{machine:#?}");
}
