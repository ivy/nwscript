//! `nws run`: run a script under the recording engine or the world model.

use super::Ctx;
use crate::Exit;
use crate::io::{display_name, read_input};
use nwscript::Envelope;
use nwscript::diag::SourceMap;
use nwscript::vm::{Limits, Recording, World};
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  nws --game k1 --nwscript nwscript.nss run k_con_canderpm.ncs
  nws --game k1 -I corpus run --trace k_pdan_mand04.ncs
  nws --game k1 -I corpus --format json run --engine world k_ai_master.ncs

The script runs from its entry, then the deferred actions it created
(DelayCommand, AssignCommand, ActionDoCommand) in order. Every engine call
is recorded with its arguments and reply; a reply no model produced is
marked unmodelled. KotOR scripts take no arguments, so there are none to pass.")]
pub struct Args {
    /// The NCS file, or - for stdin.
    pub input: PathBuf,
    /// Print every engine call.
    #[arg(long)]
    pub trace: bool,
    /// recording: every routine answers its type's default; world: an empty world model answers the routines it covers.
    #[arg(long, value_enum, default_value_t = EngineArg::Recording)]
    pub engine: EngineArg,
    /// Seed for the world model's random numbers.
    #[arg(long, default_value_t = 0)]
    pub seed: u64,
    /// Instructions before the run stops, deferred actions included.
    #[arg(long, default_value_t = 5_000_000)]
    pub budget: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum EngineArg {
    Recording,
    World,
}

pub fn run(ctx: &Ctx, args: &Args) -> Exit {
    let game = match ctx.require_game("run") {
        Ok(g) => g,
        Err(e) => return e,
    };
    crate::working_on(display_name(&args.input));
    if args
        .input
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("nss"))
    {
        eprintln!(
            "error: `nws run` takes compiled bytecode (.ncs) until the compiler lands; compile the script first"
        );
        return Exit::Usage;
    }
    let (bytes, input) = match read_input(&args.input, "bytecode") {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            return Exit::Usage;
        }
    };
    let dir = args
        .input
        .parent()
        .filter(|_| args.input.as_os_str() != "-")
        .map(|d| {
            if d.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                d.to_path_buf()
            }
        });
    let mut resolver = ctx.resolver(dir);
    let mut map = SourceMap::new();
    let table = match nwscript::engine::load_table(&mut resolver, &mut map) {
        Ok(Some((t, _))) => t,
        Ok(None) => {
            eprintln!(
                "error: `nws run` needs the game's nwscript.nss for its engine routines: pass --nwscript FILE or -I DIR"
            );
            return Exit::Usage;
        }
        Err(e) => {
            eprintln!("error: reading nwscript.nss: {e}");
            return Exit::Usage;
        }
    };
    let mut env = Envelope::new("run", Some(game));
    env.inputs.push(input);
    env.inputs
        .extend(nwscript::Resolver::inputs(&resolver).iter().cloned());
    let routines = nwscript::engine::routines(&table);
    let limits = Limits {
        steps: args.budget,
        ..Limits::default()
    };
    let mut recording = Recording;
    let mut world = World::new(args.seed);
    let engine: &mut dyn nwscript::vm::Engine = match args.engine {
        EngineArg::Recording => &mut recording,
        EngineArg::World => &mut world,
    };
    let (r, fault) = match nwscript::run::run_bytes(
        &display_name(&args.input),
        &bytes,
        &routines,
        engine,
        limits,
        &mut map,
    ) {
        Ok(x) => x,
        Err(d) => return ctx.finish(env, "", vec![d], &map, Exit::Ok),
    };
    let mut text = String::new();
    if args.trace {
        for c in &r.trace {
            text.push_str(&format!("{c}\n"));
        }
    }
    match &r.result {
        Some(v) => text.push_str(&format!("result: {v}\n")),
        None if r.fault.is_none() => text.push_str("finished\n"),
        None => {}
    }
    if !r.unmodelled.is_empty() && !args.trace && r.fault.is_none() {
        text.push_str(&format!(
            "{} engine calls; unmodelled: {}\n",
            r.trace.len(),
            r.unmodelled.join(", ")
        ));
    }
    env.result = serde_json::json!({
        "result": r.result.as_ref().map(|v| v.to_string()),
        "steps": r.steps,
        "unmodelled": r.unmodelled,
        "deferred_skipped": r.deferred_skipped,
        "trace": r.trace.iter().map(|c| serde_json::json!({
            "routine": c.routine,
            "name": c.name,
            "args": c.args.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
            "ret": c.ret.to_string(),
            "modelled": c.modelled,
            "deferred": c.deferred,
        })).collect::<Vec<_>>(),
    });
    ctx.finish(env, &text, fault.into_iter().collect(), &map, Exit::Ok)
}
