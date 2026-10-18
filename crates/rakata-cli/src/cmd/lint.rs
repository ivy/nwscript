//! `rakata lint --game DIR`: the tree lints, through [`rakata::lint`].

use std::path::PathBuf;

use clap::Args;

use super::Ctx;
use super::inspect::profile;
use crate::Exit;
use crate::cli::ProfileArg;
use rakata::diag::SourceMap;
use rakata::lint::{Known, Options, codes};

#[derive(Args, Debug)]
#[command(after_help = "\
The rules, each with its code (`rakata explain CODE`):
  RW1901  a reference that resolves to nothing: a GFF's CResRef field, a
          2DA cell of a resref column, or a string a script passes to an
          engine routine's resref parameter, naming no resource of the type
          the site names where it is loaded (a module's members in that
          module, anything else in any)
  RW1902  a strref in a GFF or a 2DA strref column at or past the end of
          dialog.tlk
  RW1903  two Override files whose names differ only in case
  RE1901  a .tpc in Override beside a .tga or .dds of the same name, which
          crashes the game

--paths FILE checks only the files it lists, one per line relative to the
tree, as a modshop checkpoint's changed.txt lists them (`+ `, `~ ` and `- `
marks; removed paths are skipped); resolution still sees the whole install.
--db FILE reads the contents from an index of the tree (`rakata index
build`) instead of walking it. --known FILE accepts the findings it lists
(conformance/tree-known.toml holds the vanilla game's own); they are
counted, not listed. --write-known FILE writes every finding as a known
list. Exit 1 when a finding is not in the known list, else 0.

Examples:
  rakata lint --game ~/games/swkotor --known conformance/tree-known.toml
  rakata lint --game ckpt/CHILD/tree --paths ckpt/CHILD/changed.txt --known KNOWN
  rakata lint --game BUILD --db index.sqlite --format json")]
pub struct LintArgs {
    /// Check only the files this list names, relative to the tree.
    #[arg(long, value_name = "FILE")]
    pub paths: Option<PathBuf>,
    /// Read the contents from this index of the tree instead of walking it.
    #[arg(long, value_name = "FILE")]
    pub db: Option<PathBuf>,
    /// Accept the findings this known list holds.
    #[arg(long, value_name = "FILE")]
    pub known: Option<PathBuf>,
    /// Write every finding, known or not, as a known list to FILE.
    #[arg(long, value_name = "FILE")]
    pub write_known: Option<PathBuf>,
    /// The game the install belongs to.
    #[arg(long, value_enum, default_value_t = ProfileArg::K1)]
    pub profile: ProfileArg,
}

pub fn run(ctx: &Ctx, args: &LintArgs) -> Exit {
    let game = match ctx.require_game("lint") {
        Ok(g) => g,
        Err(exit) => return exit,
    };
    let mut env = ctx.envelope("lint");
    let map = SourceMap::new();
    crate::working_on(game.display().to_string());
    let input = |what: &str, p: &std::path::Path, e: String| {
        rakata::index::diagnostic(
            codes::INPUT,
            format!("cannot read the {what} {}: {e}", p.display()),
        )
    };
    let paths = match &args.paths {
        Some(p) => match std::fs::read_to_string(p) {
            Ok(text) => Some(rakata::index::parse_changed(&text)),
            Err(e) => {
                return ctx.finish(
                    env,
                    "",
                    vec![input("paths", p, e.to_string())],
                    &map,
                    Exit::Ok,
                );
            }
        },
        None => None,
    };
    let known = match &args.known {
        Some(p) => match std::fs::read_to_string(p)
            .map_err(|e| e.to_string())
            .and_then(|t| Known::parse(&t))
        {
            Ok(k) => Some(k),
            Err(e) => return ctx.finish(env, "", vec![input("known list", p, e)], &map, Exit::Ok),
        },
        None => None,
    };
    let report = match rakata::lint::run(&Options {
        game,
        profile: profile(args.profile),
        paths: paths.as_deref(),
        db: args.db.as_deref(),
        known: known.as_ref(),
        threads: 0,
    }) {
        Ok(r) => r,
        Err(d) => return ctx.finish(env, "", vec![d], &map, Exit::Ok),
    };
    let mut diags = report.warnings.clone();
    if let Some(p) = &args.write_known {
        let header = "Findings `rakata lint --known` accepts: a table per code, each path's sites.\nWritten by `rakata lint --write-known`.";
        if let Err(e) = std::fs::write(p, Known::render(header, &report.all)) {
            diags.push(rakata::index::diagnostic(
                "RE1306",
                format!("cannot write {}: {e}", p.display()),
            ));
        }
    }
    diags.extend(report.diagnostics());
    let exit = if report.findings.is_empty() {
        Exit::Ok
    } else {
        Exit::Errors
    };
    let text = report.summary();
    env.result = serde_json::to_value(&report).expect("the report serializes");
    ctx.finish(env, &text, diags, &map, exit)
}
