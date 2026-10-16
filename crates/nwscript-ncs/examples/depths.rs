//! Print stack-depth conflicts for one NCS file: `cargo run --example depths -- FILE NWSCRIPT.NSS`.
use nwscript_diag::FileId;
use nwscript_ncs::cfg::{build, stack_depths};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let p = nwscript_ncs::decode(FileId(0), &std::fs::read(&args[1]).unwrap()).unwrap();
    let text = String::from_utf8_lossy(&std::fs::read(&args[2]).unwrap()).into_owned();
    let size = |t: &str| match t.trim() {
        "void" | "action" => 0,
        "vector" => 12,
        _ => 4,
    };
    let mut table: Vec<(Vec<i32>, i32, String)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(open) = line.find('(') else { continue };
        let head: Vec<&str> = line[..open].split_whitespace().collect();
        if head.len() != 2 || !line.ends_with(");") || line.starts_with("//") {
            continue;
        }
        let params = &line[open + 1..line.rfind(')').unwrap()];
        let sizes = params
            .split(',')
            .filter(|p| !p.trim().is_empty())
            .map(|p| size(p.split_whitespace().next().unwrap_or("")))
            .collect();
        table.push((sizes, size(head[0]), head[1].to_string()));
    }
    let action = |r: u16, argc: u8| {
        let (params, ret, _) = table.get(usize::from(r))?;
        Some(ret - params.iter().take(usize::from(argc)).sum::<i32>())
    };
    let cfg = build(&p);
    let d = stack_depths(&p, &cfg, &action);
    let offs = p.offsets();
    let ins = p.instructions();
    for (i, a, b) in d.conflicts.iter().take(5) {
        println!("conflict at {} ({}): {a} vs {b}", offs[*i], ins[*i]);
        for j in i.saturating_sub(12)..(*i + 2).min(ins.len()) {
            let name = match &ins[j].operands {
                nwscript_ncs::Operands::Action { routine, .. } => table
                    .get(usize::from(*routine))
                    .map(|t| t.2.clone())
                    .unwrap_or_default(),
                _ => String::new(),
            };
            println!("  {:>6} [{:?}] {} {}", offs[j], d.before[j], ins[j], name);
        }
    }
}
