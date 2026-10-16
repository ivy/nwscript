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

/// The routine table from `nwscript.nss`'s prototypes, read naively for this
/// test; the front end's function table is the real one.
fn routines(nwscript: &Path) -> Vec<Routine> {
    let text = String::from_utf8_lossy(&std::fs::read(nwscript).unwrap()).into_owned();
    let ty = |t: &str| match t.trim() {
        "void" => Ty::Void,
        "int" | "INT" => Ty::Int,
        "float" => Ty::Float,
        "string" => Ty::String,
        "object" | "OBJECT_ID" => Ty::Object,
        "vector" => Ty::Vector,
        "action" => Ty::Action,
        "effect" => Ty::Engine(0),
        "event" => Ty::Engine(1),
        "location" => Ty::Engine(2),
        "talent" => Ty::Engine(3),
        other => panic!("unknown type {other}"),
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(open) = line.find('(') else { continue };
        let head: Vec<&str> = line[..open].split_whitespace().collect();
        if head.len() != 2 || !line.ends_with(");") || line.starts_with("//") {
            continue;
        }
        let params = &line[open + 1..line.rfind(')').unwrap()];
        let mut parts = Vec::new();
        let (mut depth, mut start) = (0, 0);
        for (i, c) in params.char_indices() {
            match c {
                '[' => depth += 1,
                ']' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(&params[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        parts.push(&params[start..]);
        let params = parts
            .into_iter()
            .filter(|p| !p.trim().is_empty())
            .map(|p| ty(p.split_whitespace().next().unwrap()))
            .collect();
        out.push(Routine {
            name: head[1].to_string(),
            params,
            ret: ty(head[0]),
        });
    }
    out
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
        for x in v.iter().take(4) {
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
