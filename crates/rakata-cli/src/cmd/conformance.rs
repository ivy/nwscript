//! `rakata conformance`: the round-trip suite over an install, through
//! [`rakata::conformance::run`].

use std::path::PathBuf;

use clap::Args;

use super::Ctx;
use crate::Exit;
use rakata::conformance::{Known, Registry};
use rakata::diag::SourceMap;

#[derive(Args, Debug)]
#[command(after_help = "\
Walks Override/, modules/, rims/, lips/, the BIFs behind chitin.key and
data/*.bif, patch.erf, TexturePacks/*.erf, streamwaves/**/*.lip and
dialog.tlk, into every archive's members. A file with a registered format
is read, written again and classified: identical, equal after re-read (a
layout another writer chose, named in --known), or different. Files with
no registered format are counted as \"no reader\".

Examples:
  rakata conformance --game ~/games/swkotor
  rakata conformance --game ~/games/swkotor --only archive --report target/rakata-conformance.json
  rakata --format json conformance --game ~/games/swkotor

Exit 0 when every file is identical or equal after re-read; 1 when a file
failed to read; 3 when any file is different.")]
pub struct ConformanceArgs {
    /// Run only these formats, comma-separated (`archive`). Default: all.
    #[arg(long, value_name = "NAME,...", value_delimiter = ',')]
    pub only: Vec<String>,
    /// Write the report as JSON.
    #[arg(long, value_name = "FILE")]
    pub report: Option<PathBuf>,
    /// Write the summary as markdown; it is also the text output.
    #[arg(long, value_name = "FILE")]
    pub markdown: Option<PathBuf>,
    /// Named layout variants other writers choose, shown beside each file
    /// that is equal after re-read.
    #[arg(
        long,
        value_name = "FILE",
        default_value = "conformance/rakata-known.toml"
    )]
    pub known: PathBuf,
    /// Threads. Defaults to the number of CPUs.
    #[arg(long, value_name = "N", default_value_t = 0)]
    pub jobs: usize,
}

pub fn run(ctx: &Ctx, args: &ConformanceArgs) -> Exit {
    let game = match ctx.require_game("conformance") {
        Ok(g) => g,
        Err(e) => return e,
    };
    let registry = if args.only.is_empty() {
        Registry::builtin()
    } else {
        match Registry::builtin().only(&args.only) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error: --only: {e}");
                return Exit::Usage;
            }
        }
    };
    let known = match Known::load(&args.known) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("error: --known {e}");
            return Exit::Usage;
        }
    };
    crate::working_on(game.display().to_string());
    let mut env = ctx.envelope("conformance");
    let report = match rakata::conformance::run(game, &registry, &known, args.jobs) {
        Ok(r) => r,
        Err(d) => {
            return ctx.finish(env, "", vec![d], &SourceMap::new(), Exit::Errors);
        }
    };
    let md = report.markdown();
    if let Some(p) = &args.report
        && let Err(e) = std::fs::write(
            p,
            serde_json::to_string_pretty(&report).expect("the report serializes") + "\n",
        )
    {
        eprintln!("error: writing {}: {e}", p.display());
        return Exit::Errors;
    }
    if let Some(p) = &args.markdown
        && let Err(e) = std::fs::write(p, &md)
    {
        eprintln!("error: writing {}: {e}", p.display());
        return Exit::Errors;
    }
    let exit = report.exit();
    let diags = report.diagnostics();
    env.result = serde_json::to_value(&report).expect("the report serializes");
    ctx.out.emit(env, &md, &diags, &SourceMap::new());
    exit
}
