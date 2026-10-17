//! `nws diff`: two scripts compared by bytes, instructions and behaviour.

use super::Ctx;
use crate::Exit;
use crate::io::{display_name, read_input};
use nwscript::Envelope;
use nwscript::compare::{self, Level};
use nwscript::diag::SourceMap;
use nwscript::vm::{Engine, Recording, World};
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  nws --game k1 -I corpus diff old.ncs new.ncs
  nws diff --level bytes old.ncs new.ncs
  nws --game k1 -I corpus diff --level trace --engine world old.ncs new.ncs

Without --level all three are compared: bytes; instructions, with every jump
resolved to the instruction it reaches; and the trace of engine calls under
the recording engine (which needs nwscript.nss through --nwscript or -I).
Exit 0 when the scripts are the same at every level compared, 3 otherwise.")]
pub struct Args {
    pub first: PathBuf,
    pub second: PathBuf,
    /// Compare at this level only. Repeatable.
    #[arg(long, value_enum)]
    pub level: Vec<LevelArg>,
    /// The engine for the trace comparison.
    #[arg(long, value_enum, default_value_t = EngineArg::Recording)]
    pub engine: EngineArg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum LevelArg {
    Bytes,
    Asm,
    Trace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum EngineArg {
    Recording,
    World,
}

pub fn run(ctx: &Ctx, args: &Args) -> Exit {
    let levels = if args.level.is_empty() {
        vec![LevelArg::Bytes, LevelArg::Asm, LevelArg::Trace]
    } else {
        args.level.clone()
    };
    let mut env = Envelope::new("diff", ctx.game);
    let mut map = SourceMap::new();
    let mut load = |p: &PathBuf| -> Result<(Vec<u8>, nwscript::ncs::Program), Exit> {
        crate::working_on(display_name(p));
        if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("nss")) {
            eprintln!(
                "error: `nws diff` compares compiled bytecode (.ncs) until the compiler lands"
            );
            return Err(Exit::Usage);
        }
        let (bytes, rec) = read_input(p, "bytecode").map_err(|e| {
            eprintln!("error: {e}");
            Exit::Usage
        })?;
        env.inputs.push(rec);
        let program =
            nwscript::ncs::decode_named(display_name(p), &bytes, &mut map).map_err(|d| {
                eprintln!("error: {}: {}", display_name(p), d.message);
                Exit::Errors
            })?;
        Ok((bytes, program))
    };
    let (a, pa) = match load(&args.first) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let (b, pb) = match load(&args.second) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let mut results: Vec<(&str, Level)> = Vec::new();
    for l in levels {
        let r = match l {
            LevelArg::Bytes => ("bytes", compare::bytes(&a, &b)),
            LevelArg::Asm => ("instructions", compare::instructions(&pa, &pb)),
            LevelArg::Trace => {
                let dir = args.first.parent().map(|d| {
                    if d.as_os_str().is_empty() {
                        PathBuf::from(".")
                    } else {
                        d.to_path_buf()
                    }
                });
                let mut resolver = ctx.resolver(dir);
                let mut tmap = SourceMap::new();
                match nwscript::engine::load_table(&mut resolver, &mut tmap) {
                    Ok(Some((table, _))) => {
                        env.inputs
                            .extend(nwscript::Resolver::inputs(&resolver).iter().cloned());
                        let routines = nwscript::engine::routines(&table);
                        let world = args.engine == EngineArg::World;
                        let mut make = move || -> Box<dyn Engine> {
                            if world {
                                Box::new(World::new(0))
                            } else {
                                Box::new(Recording)
                            }
                        };
                        ("trace", compare::trace(&pa, &pb, &routines, &mut make))
                    }
                    _ => (
                        "trace",
                        Level::Skipped {
                            why: "no nwscript.nss: pass --nwscript FILE or -I DIR".into(),
                        },
                    ),
                }
            }
        };
        results.push(r);
    }
    let mut text = String::new();
    for (name, l) in &results {
        match l {
            Level::Same => text.push_str(&format!("{name}: same\n")),
            Level::Different { at } => text.push_str(&format!("{name}: different at {at}\n")),
            Level::Skipped { why } => text.push_str(&format!("{name}: skipped: {why}\n")),
        }
    }
    let different = results.iter().any(|(_, l)| l.is_different());
    env.result = serde_json::json!(
        results
            .iter()
            .map(|(n, l)| (n.to_string(), serde_json::to_value(l).expect("serializes")))
            .collect::<serde_json::Map<_, _>>()
    );
    ctx.finish(
        env,
        &text,
        Vec::new(),
        &map,
        if different { Exit::Different } else { Exit::Ok },
    )
}
