//! `rakata lua`: a Lua script with the façade loaded, hermetic, over
//! `rakata-lua`.

use clap::Args;
use serde_json::json;
use std::path::PathBuf;

use super::Ctx;
use crate::Exit;
use crate::cli::{Format, ProfileArg};
use rakata::diag::{Diagnostic, SourceMap};
use rakata::game::Profile;
use rakata_lua::{Options, Print};

#[derive(Args, Debug)]
#[command(after_help = "\
The script gets the global `rakata`: Install and Mod, the formats (twoda, gff,
lip, tlk), dialog with stable node keys, the authoring operations (author), and
errors as rakata.Error values carrying their codes. rakata.args holds the
arguments after the script's name; put `--` before one that begins with `-`.
`rakata lua --reference` prints the whole API.

The run is hermetic: no io, os, package or debug library; require loads only
modules from the script's directory; rakata.fs reads only below --game, the
script's directory and the paths the arguments name, and writes only below
--game and the arguments' paths (RE1801 otherwise). An error the script does
not catch exits 1 with its code.

Examples:
  rakata lua build.lua OUT --game ~/games/swkotor
  rakata lua survey.lua --game DIR --profile k2 -- --verbose
  rakata --format json lua build.lua OUT --game DIR
  rakata lua --reference")]
pub struct LuaArgs {
    /// The Lua file to run.
    #[arg(value_name = "SCRIPT", required_unless_present = "reference")]
    pub script: Option<PathBuf>,
    /// Arguments passed to the script as rakata.args.
    #[arg(value_name = "ARGS")]
    pub args: Vec<String>,
    /// The game the install belongs to: the profile Install.open takes when the script names none.
    #[arg(long, value_enum, default_value_t = ProfileArg::K1)]
    pub profile: ProfileArg,
    /// Print the reference for scripts, every table and function, and exit.
    #[arg(long)]
    pub reference: bool,
}

pub fn run(ctx: &Ctx, args: &LuaArgs) -> Exit {
    if args.reference {
        print!("{}", rakata_lua::REFERENCE);
        return Exit::Ok;
    }
    let Some(script) = &args.script else {
        eprintln!("error: `rakata lua` needs a script");
        return Exit::Usage;
    };
    crate::working_on(script.display().to_string());
    let json = ctx.out.format == Format::Json;
    let opts = Options {
        script: script.clone(),
        args: args.args.clone(),
        game: ctx.game.clone(),
        profile: match args.profile {
            ProfileArg::K1 => Profile::K1,
            ProfileArg::K2 => Profile::K2,
        },
        print: if json { Print::Capture } else { Print::Stdout },
    };
    let mut env = ctx.envelope("lua");
    let map = SourceMap::new();
    let (output, diags) = match rakata_lua::run(&opts) {
        Ok(o) => (o.output, Vec::new()),
        Err(f) => {
            let code = rakata::codes::lookup(f.code)
                .or_else(|| rakata::codes::lookup(rakata_lua::codes::SCRIPT_FAILED))
                .expect("the binding's codes are registered");
            let mut d = Diagnostic::new(code, f.message);
            if let Some(t) = f.traceback {
                d = d.note(t);
            }
            (f.output, vec![d])
        }
    };
    env.result = json!({
        "script": script.display().to_string(),
        "args": args.args,
        "output": output,
    });
    // In text mode the script printed as it ran.
    ctx.finish(env, "", diags, &map, Exit::Ok)
}
