//! `rakata lip show`: a lip-sync file's length and keyframes, over
//! `rakata-lip`.

use clap::{Args, Subcommand};
use serde_json::json;

use super::Ctx;
use super::inspect::{self, LookupArgs, resource_forms};
use crate::Exit;
use rakata::diag::SourceMap;
use rakata::inspect::diagnostic;
use rakata::lip::Lip;

const SHOW: &str = concat!(
    "\
Prints `length SECONDS, N keyframes`, then one `TIME<TAB>SHAPE` line per
keyframe, the shape by its label (rest ee eh ah oh oo y s f n th m t sh l k,
the file's bytes 0 to 15 in that order). --format json gives each keyframe's
time, shape byte and label.

",
    resource_forms!(),
    "

Examples:
  rakata lip show ./n_juhani_01.lip
  rakata lip show 'lips/danm14ac_loc.mod!n_juhani_01.lip' --game DIR
  rakata --format json lip show n_juhani_01.lip --game DIR --module danm14ac"
);

#[derive(Args, Debug)]
#[command(after_help = "\
Example:
  rakata lip show ./n_juhani_01.lip")]
pub struct LipArgs {
    #[command(subcommand)]
    pub action: LipAction,
}

#[derive(Subcommand, Debug)]
pub enum LipAction {
    /// Print a lip-sync file's length and keyframes.
    #[command(after_help = SHOW)]
    Show {
        /// The lip-sync file: NAME.lip, PATH or ARCHIVE!MEMBER.
        #[arg(value_name = "RESOURCE")]
        name: String,
        #[command(flatten)]
        lookup: LookupArgs,
    },
}

pub fn run(ctx: &Ctx, args: &LipArgs) -> Exit {
    let LipAction::Show { name, lookup } = &args.action;
    let mut env = ctx.envelope("lip show");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let loaded = match inspect::load(&mut env, "resource", name, &lookup.lookup(ctx)) {
        Ok(l) => l,
        Err(d) => return fail(env, d),
    };
    let lip = match Lip::parse(&loaded.bytes) {
        Ok(l) => l,
        Err(e) => {
            let d = diagnostic(e.code(), &e).note(format!("while reading `{}`", loaded.source));
            return fail(env, d);
        }
    };
    let mut text = format!("length {}, {} keyframes", lip.length, lip.keys.len());
    for k in &lip.keys {
        text.push_str(&format!("\n{}\t{}", k.time, k.shape));
    }
    env.result = json!({
        "source": loaded.source,
        "length": lip.length,
        "keys": lip.keys.iter().map(|k| json!({
            "time": k.time,
            "shape": k.shape.byte(),
            "label": k.shape.label(),
        })).collect::<Vec<_>>(),
    });
    ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
}
