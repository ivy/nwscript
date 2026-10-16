//! Run one NCS and print its trace or its last instructions before a fault:
//! `cargo run -p nwscript-vm --example vmtrace -- FILE.ncs NWSCRIPT.NSS`.
use nwscript_diag::FileId;
use nwscript_vm::{Limits, Recording, Routine, Ty, run};
use std::path::Path;

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

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = nwscript_ncs::decode(FileId(0), &std::fs::read(&a[1]).unwrap()).unwrap();
    let r = run(
        &p,
        &routines(Path::new(&a[2])),
        &mut Recording,
        Limits {
            history: 40,
            ..Limits::default()
        },
    );
    for c in &r.trace {
        println!("{c}");
    }
    if let Some(f) = &r.fault {
        println!("FAULT {f}");
        for h in &r.history {
            println!("{h}");
        }
    }
    println!("result: {:?}", r.result);
}
