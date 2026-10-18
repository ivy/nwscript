//! `rakata diff A B`: two trees or two resources compared by field, row,
//! strref and dialogue node, through [`rakata::diff`].

use std::time::Instant;

use clap::Args;
use serde_json::json;

use super::Ctx;
use super::inspect::{LookupArgs, profile};
use crate::Exit;
use rakata::diag::SourceMap;

#[derive(Args, Debug)]
#[command(after_help = "\
A and B are two directories (trees) or two resources: files, archives, an
ARCHIVE!MEMBER, or a NAME.EXT resolved in --game as `rakata cat` resolves it.

Trees pair their files by path in any case, and a case-only rename is a
change of its own. A directory reached through a symbolic link is not
descended; where it points is compared. Two names of one file (a hard link)
are the same without reading. Archives (KEY, BIF, ERF, MOD, SAV, HAK, RIM)
are recursed: members pair by name, a BIF's by position as #N.ext, and each
shows as ARCHIVE!MEMBER. Each pair whose bytes differ is compared by format:
  GFF        by field path (fields by label, list elements by position)
  DLG        by node: added, removed and changed, each by its stable key,
             voice resref, then strref, then text; a node with none is keyed
             by index and marked fragile; links compare by target key
  2DA        by row label and column
  TLK        by strref
  LIP        by length and keyframe
  KEY        by resource: which BIF and index holds it
  others     by hash, with the header fields known (a TPC's size and
             encoding, a WAV's duration), marked contents not compared
A pair whose bytes differ while its data is equal (another tool's layout) is
counted, not listed.

An Override file one tree has and the other lacks is compared with the
copies the other tree's install loads in its place (patch.erf or the BIFs,
and each module holding it): a mod's changed dialogue in Override shows its
nodes, `over modules/x.rim!y.dlg`.

--only keeps the paths matching a glob, a path, or a directory or archive
they lie in, in any case: --only Override/ --only '*.dlg'. In the text, +
is added, - removed, ~ changed, R renamed; sites are indented beneath their
path. --format json gives the same entries with every site typed, and the
wall time. Exit 0 when the sides hold the same data, 3 when they differ.

Examples:
  rakata diff ckpt/PARENT/tree ckpt/CHILD/tree
  rakata diff ckpt/PARENT/tree ckpt/CHILD/tree --only Override/ --only '*.dlg'
  rakata diff ./danm14ac.mod ./danm14ac-edited.mod
  rakata diff dan14_juhani.dlg ./dan14_juhani.dlg --game DIR --module danm14ac
  rakata --format json diff A/tree B/tree")]
pub struct DiffArgs {
    /// The first side: a directory, or a resource.
    #[arg(value_name = "A")]
    pub a: String,
    /// The second side, of the same kind.
    #[arg(value_name = "B")]
    pub b: String,
    /// Keep only paths matching this glob, path, or directory or archive prefix; repeatable.
    #[arg(long, value_name = "PATH", num_args = 1..)]
    pub only: Vec<String>,
    #[command(flatten)]
    pub lookup: LookupArgs,
}

pub fn run(ctx: &Ctx, args: &DiffArgs) -> Exit {
    let mut env = ctx.envelope("diff");
    let map = SourceMap::new();
    crate::working_on(format!("{} {}", args.a, args.b));
    let options = rakata::diff::Options {
        only: args.only.clone(),
        profile: profile(args.lookup.profile),
        game: ctx.game.clone(),
        module: args.lookup.module.clone(),
    };
    let started = Instant::now();
    let d = match rakata::diff::diff(&args.a, &args.b, &options) {
        Ok(d) => d,
        Err(e) => return ctx.finish(env, "", vec![e], &map, Exit::Ok),
    };
    let seconds = started.elapsed().as_secs_f64();
    let diags = d.warnings.iter().map(|w| w.diagnostic()).collect();
    let mut text = d.text();
    text.push_str(&format!(" in {seconds:.2} s"));
    let exit = if d.is_same() {
        Exit::Ok
    } else {
        Exit::Different
    };
    let mut result = serde_json::to_value(&d).expect("the diff serializes");
    result["same"] = json!(d.is_same());
    result["seconds"] = json!(seconds);
    env.result = result;
    ctx.finish(env, &text, diags, &map, exit)
}
