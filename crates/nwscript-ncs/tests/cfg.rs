//! The control-flow view over the corpus: every instruction in one block,
//! and a consistent stack depth at every `RETN`.

use nwscript_diag::FileId;
use nwscript_ncs::cfg::{build, stack_depths};
use nwscript_ncs::{Opcode, decode};
use std::path::{Path, PathBuf};

fn corpus() -> Option<PathBuf> {
    let dir = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".cache/nwscript/corpus/k1"),
    };
    dir.join("ncs").is_dir().then_some(dir)
}

/// Each engine routine's parameter sizes and return size, from the
/// prototypes in `nwscript.nss`, in order. A deliberately small reading for
/// this test; the engine function table proper belongs to the front end.
fn routines(nwscript: &Path) -> Vec<(Vec<i32>, i32)> {
    let text = String::from_utf8_lossy(&std::fs::read(nwscript).unwrap()).into_owned();
    let size = |t: &str| match t.trim() {
        "void" | "action" => 0,
        "vector" => 12,
        _ => 4,
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(open) = line.find('(') else { continue };
        let head: Vec<&str> = line[..open].split_whitespace().collect();
        if head.len() != 2 || !line.trim_end().ends_with(");") || line.starts_with("//") {
            continue;
        }
        let params = &line[open + 1..line.rfind(')').unwrap()];
        let sizes = params
            .split(',')
            .filter(|p| !p.trim().is_empty())
            .map(|p| size(p.split_whitespace().next().unwrap_or("")))
            .collect();
        out.push((sizes, size(head[0])));
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
fn corpus_cfg_and_depths() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let table = routines(&root.join("nwscript.nss"));
    assert!(table.len() > 700, "read {} prototypes", table.len());
    let action = |r: u16, argc: u8| {
        let (params, ret) = table.get(usize::from(r))?;
        let args: i32 = params.iter().take(usize::from(argc)).sum();
        Some(ret - args)
    };
    let files = all_ncs(&root);
    let (mut ok, mut dead, mut problems) = (0, 0, Vec::new());
    for f in &files {
        let p = decode(FileId(0), &std::fs::read(f).unwrap()).unwrap();
        let cfg = build(&p);
        // Every instruction is in exactly one block: blocks tile the program.
        let mut covered = 0;
        for (b, blk) in cfg.blocks.iter().enumerate() {
            assert_eq!(blk.start, covered, "{}: blocks do not tile", f.display());
            assert!(blk.end > blk.start);
            assert!(cfg.block_of[blk.start..blk.end].iter().all(|&x| x == b));
            covered = blk.end;
        }
        assert_eq!(covered, p.len());
        let d = stack_depths(&p, &cfg, &action);
        let unreachable = cfg.blocks.iter().filter(|b| b.sub.is_none()).count();
        let unknown_retn = p
            .instructions()
            .iter()
            .enumerate()
            .filter(|(i, x)| {
                x.opcode == Opcode::Retn
                    && d.before[*i].is_none()
                    && cfg.blocks[cfg.block_of[*i]].sub.is_some()
            })
            .count();
        let mut why = Vec::new();
        if !d.conflicts.is_empty() {
            why.push(format!("{} depth conflicts", d.conflicts.len()));
        }
        if unknown_retn > 0 {
            why.push(format!("{unknown_retn} RETN of unknown depth"));
        }
        dead += usize::from(unreachable > 0);
        if !cfg.shared.is_empty() {
            why.push(format!("{} blocks shared by subroutines", cfg.shared.len()));
        }
        if why.is_empty() {
            ok += 1;
        } else {
            problems.push((
                f.strip_prefix(&root)
                    .unwrap()
                    .with_extension("")
                    .display()
                    .to_string(),
                why.join(", "),
            ));
        }
    }
    println!(
        "cfg: {ok}/{} with consistent depths at every RETN; {dead} have dead code a return leaves behind",
        files.len()
    );
    for (f, why) in &problems {
        println!("  {f}: {why}");
    }
    // The conflicts that remain are in BioWare's bytecode itself:
    // - a `break` out of a `case` that declared a local jumps past the
    //   local's cleanup (the default user-defined-event template);
    // - nw_s0_lghtnbolt, a Neverwinter Nights script compiled against NWN's
    //   engine function table, whose `ACTION`s do not match KotOR's.
    const KNOWN: &[&str] = &[
        "modules/lev_m40ac_s/k_plev_fcpush_ud",
        "modules/lev_m40ac_s/k_plev_fpush2_ud",
        "modules/tar_m02ab_s/k_ptar_drunk_ud",
        "modules/tat_m18ac_s/k_pkas_banthwalk",
        "modules/tat_m18ac_s/k_ptat_dewbuser",
        "ncs/k_pkas_randwalk",
        "ncs/nw_s0_lghtnbolt",
    ];
    let unexpected: Vec<_> = problems
        .iter()
        .filter(|(f, _)| !KNOWN.contains(&f.as_str()))
        .collect();
    assert!(unexpected.is_empty(), "new depth problems: {unexpected:?}");
}
