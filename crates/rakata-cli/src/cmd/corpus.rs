//! `rakata corpus`: the script corpus through [`rakata::corpus`]: `extract`
//! writes every script of the install, `decompile` recovers every
//! bytecode-only script to source, `verify` checks a corpus against the
//! committed manifest.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use serde_json::json;

use super::Ctx;
use crate::Exit;
use crate::cli::ProfileArg;
use rakata::codes;
use rakata::corpus::{self, DecompileOptions, ExtractOptions};
use rakata::diag::{Diagnostic, SourceMap};
use rakata::game::Profile;

#[derive(Args, Debug)]
#[command(after_help = "\
The corpus is game data and lives under the user's cache: --out, else
$NWSCRIPT_CORPUS, else ~/.cache/nwscript/corpus/k1. Every run rewrites the
subtree it owns from scratch, so a rerun is byte-identical.

Examples:
  rakata corpus extract --game ~/games/swkotor
  rakata corpus decompile --game ~/games/swkotor
  rakata corpus verify
  rakata --format json corpus decompile --game ~/games/swkotor --jobs 8")]
pub struct CorpusArgs {
    #[command(subcommand)]
    pub action: CorpusAction,
}

#[derive(Subcommand, Debug)]
pub enum CorpusAction {
    /// Write every script the install holds: nss/ and ncs/ from chitin.key, nwscript.nss, oracle/, and each capsule's NCS under modules/STEM/ and rims/STEM/.
    #[command(after_help = "\
Layout written:
  nss/NAME.nss, ncs/NAME.ncs     every NSS and NCS chitin.key lists
  nwscript.nss                    as shipped
  oracle/                         nss/ with nwscript.nss's line 5145 typo fixed,
                                  the include directory for nwnnsscomp, which
                                  hangs on the shipped file
  modules/STEM/NAME.ncs           NCS from modules/*.rim|*.mod|*.erf
  rims/STEM/NAME.ncs              NCS from rims/*.rim

Examples:
  rakata corpus extract --game ~/games/swkotor
  rakata corpus extract --game ~/games/swkotor --manifest conformance/k1.toml --source \"Steam, 2024-05-01\"")]
    Extract {
        /// The corpus directory. Default: $NWSCRIPT_CORPUS, else ~/.cache/nwscript/corpus/k1.
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
        /// Also write the manifest here, and the module hashes beside it as FILE-modules.toml.
        #[arg(long, value_name = "FILE")]
        manifest: Option<PathBuf>,
        /// The manifest's `source` field: where the game came from.
        #[arg(long, value_name = "TEXT", default_value = "unknown")]
        source: String,
    },
    /// Recover every bytecode-only script to source under decompiled/, each with a header naming its origin, the copy the engine loads and its verified functions, with include libraries folded out, and the engine calls in decompiled/calls.jsonl.
    #[command(after_help = "\
Every .ncs in every module capsule, under rims/ and under Override/ is
decompiled to decompiled/CAPSULE/NAME.nss and verified by recompiling with
the project's compiler against the declarations BioWare's scripts were
compiled against. A recovered function that is byte for byte a function of
an include library the install holds is replaced by the #include line, when
the folded source still recompiles to the original bytes. calls.jsonl has
one line per engine call: script, capsule, routine, the literal arguments
(null where computed) and the instruction offset. The wall time is printed
last.

Examples:
  rakata corpus decompile --game ~/games/swkotor
  rakata corpus decompile --game ~/games/swkotor --out /tmp/corpus --jobs 4
  grep -rl k_pman_aioff ~/.cache/nwscript/corpus/k1/decompiled/")]
    Decompile {
        /// The corpus directory; the tree goes under decompiled/ in it. Default: $NWSCRIPT_CORPUS, else ~/.cache/nwscript/corpus/k1.
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
        /// The game the install belongs to.
        #[arg(long, value_enum, default_value_t = ProfileArg::K1)]
        profile: ProfileArg,
        /// Threads. Defaults to the number of CPUs.
        #[arg(long, value_name = "N", default_value_t = 0)]
        jobs: usize,
    },
    /// Check a corpus against the manifest and its modules file: every named file present with the recorded hash.
    #[command(after_help = "\
Exit 0 when every file matches, 3 when any is missing or differs.

Examples:
  rakata corpus verify
  rakata corpus verify --out /tmp/corpus --manifest conformance/k1.toml")]
    Verify {
        /// The corpus directory. Default: $NWSCRIPT_CORPUS, else ~/.cache/nwscript/corpus/k1.
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
        /// The manifest; its modules file is found beside it by name.
        #[arg(long, value_name = "FILE", default_value = "conformance/k1.toml")]
        manifest: PathBuf,
    },
}

fn error(e: &corpus::Error) -> Diagnostic {
    let code = codes::lookup(e.code())
        .unwrap_or_else(|| panic!("corpus code {} is not registered", e.code()));
    Diagnostic::new(code, e.to_string())
}

pub fn run(ctx: &Ctx, args: &CorpusArgs) -> Exit {
    let map = SourceMap::new();
    match &args.action {
        CorpusAction::Extract {
            out,
            manifest,
            source,
        } => {
            let game = match ctx.require_game("corpus extract") {
                Ok(g) => g.to_path_buf(),
                Err(e) => return e,
            };
            let mut env = ctx.envelope("corpus extract");
            crate::working_on(game.display().to_string());
            let o = ExtractOptions {
                game,
                out: out.clone().unwrap_or_else(corpus::default_out),
                manifest: manifest.clone(),
                source: source.clone(),
            };
            let x = match corpus::extract(&o) {
                Ok(x) => x,
                Err(e) => return ctx.finish(env, "", vec![error(&e)], &map, Exit::Ok),
            };
            let mut text = format!("corpus written to {}\n", x.out.display());
            for (bif, (nss, ncs)) in &x.per_bif {
                text.push_str(&format!("  {bif}: {nss} nss, {ncs} ncs\n"));
            }
            text.push_str(&format!(
                "  total: {} nss, {} ncs, {} pairs, {} module ncs in {} archives\n",
                x.counts.nss, x.counts.ncs, x.counts.pairs, x.counts.module_ncs, x.archives
            ));
            if let Some(m) = &x.manifest {
                text.push_str(&format!("manifest written to {}\n", m.display()));
            }
            env.result = serde_json::to_value(&x).expect("the summary serializes");
            ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
        }
        CorpusAction::Decompile { out, profile, jobs } => {
            let game = match ctx.require_game("corpus decompile") {
                Ok(g) => g.to_path_buf(),
                Err(e) => return e,
            };
            let mut env = ctx.envelope("corpus decompile");
            crate::working_on(game.display().to_string());
            let o = DecompileOptions {
                game,
                out: out.clone().unwrap_or_else(corpus::default_out),
                profile: match profile {
                    ProfileArg::K1 => Profile::K1,
                    ProfileArg::K2 => Profile::K2,
                },
                jobs: *jobs,
            };
            let (report, warnings) = match corpus::decompile(&o) {
                Ok(x) => x,
                Err(e) => return ctx.finish(env, "", vec![error(&e)], &map, Exit::Ok),
            };
            let text = report.text();
            env.result = serde_json::to_value(&report).expect("the report serializes");
            ctx.finish(env, &text, warnings, &map, Exit::Ok)
        }
        CorpusAction::Verify { out, manifest } => {
            let mut env = ctx.envelope("corpus verify");
            let out = out.clone().unwrap_or_else(corpus::default_out);
            crate::working_on(out.display().to_string());
            let v = match corpus::verify(&out, manifest) {
                Ok(v) => v,
                Err(e) => return ctx.finish(env, "", vec![error(&e)], &map, Exit::Ok),
            };
            let text = if v.is_ok() {
                format!("{} files match {}\n", v.files, manifest.display())
            } else {
                let mut t: String = v
                    .problems
                    .iter()
                    .take(20)
                    .map(|p| format!("  {p}\n"))
                    .collect();
                t.push_str(&format!(
                    "{} of {} files do not match {}\n",
                    v.problems.len(),
                    v.files,
                    manifest.display()
                ));
                t
            };
            env.result = json!({ "files": v.files, "problems": v.problems });
            let exit = if v.is_ok() { Exit::Ok } else { Exit::Different };
            ctx.finish(env, &text, Vec::new(), &map, exit)
        }
    }
}
