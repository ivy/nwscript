//! Count static `ACTION` call sites per routine over a corpus:
//! `cargo run -p nwscript-vm --example callsites -- CORPUS_DIR`.
use nwscript_diag::FileId;
use nwscript_vm::{Routine, Ty};
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
    let root = std::env::args().nth(1).expect("corpus dir");
    let root = Path::new(&root);
    let table = routines(&root.join("nwscript.nss"));
    let mut counts = vec![0usize; table.len()];
    let mut stack = vec![root.join("ncs"), root.join("modules"), root.join("rims")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let prog = nwscript_ncs::decode(FileId(0), &std::fs::read(&p).unwrap()).unwrap();
            for i in prog.instructions() {
                if let nwscript_ncs::Operands::Action { routine, .. } = i.operands {
                    counts[usize::from(routine)] += 1;
                }
            }
        }
    }
    let total: usize = counts.iter().sum();
    let mut order: Vec<usize> = (0..counts.len()).collect();
    order.sort_by(|a, b| counts[*b].cmp(&counts[*a]));
    let mut acc = 0;
    for (rank, r) in order.iter().enumerate() {
        acc += counts[*r];
        println!(
            "{:>4} {:>6} {:>5.1}% {:>3} {}",
            rank + 1,
            counts[*r],
            100.0 * acc as f64 / total as f64,
            r,
            table[*r].name
        );
        if counts[*r] == 0 {
            break;
        }
    }
    println!("total {total}");
}
