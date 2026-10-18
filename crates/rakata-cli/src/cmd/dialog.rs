//! `rakata dialog`: conversations through [`rakata::dialog`]. `show` prints
//! a dialogue's transcript.

use std::time::Instant;

use clap::{Args, Subcommand};
use serde_json::json;

use super::Ctx;
use crate::Exit;
use crate::cli::ProfileArg;
use rakata::codes;
use rakata::diag::{Diagnostic, SourceMap};
use rakata::dialog::{Session, TranscriptOptions};
use rakata::game::{ModuleName, Profile};

#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  rakata dialog show banter.dlg --game ~/games/swkotor
  rakata dialog show dan14_cutjuh.dlg --module danm14ac --game DIR
  rakata dialog show path/to/edited.dlg --module danm14ac --game DIR --scripts
  rakata --format json dialog show banter.dlg --game DIR")]
pub struct DialogArgs {
    #[command(subcommand)]
    pub action: DialogAction,
}

#[derive(Subcommand, Debug)]
pub enum DialogAction {
    /// Print a conversation as a transcript: the starts in order, then each branch indented, every node as speaker to listener with its text, voice and duration, script, animations, camera and quest, and every condition decompiled to source and marked verified or not.
    #[command(after_help = "\
DIALOG is a .dlg file, or NAME.dlg resolved the way the engine loads it:
Override/, the module's capsules (with --module, or the one module holding it
when no global layer does), patch.erf, the BIFs. Scripts and conditions are
looked up the same way, so a module's dialogue needs its module.

In the text, E3 is EntryList[3] and R0 is ReplyList[0]. An entry with an
empty Speaker is spoken by the dialogue's owner, shown as (owner); replies
are the player's. A link to a node already shown prints `-> E3 (shown
above)`; nodes no start reaches follow the tree. Animation numbers and
camera angles are the engine's own, printed without names. A voice-over's
duration is read from the head of its file under streamwaves/. A Delay of
0xFFFFFFFF is left out of the text and kept in the JSON.

Each condition is decompiled with the decompiler `nws decompile` uses,
include libraries folded out, and verified by recompiling: [verified, 1 of 1
functions] means the source recompiles to the original bytes. Scripts are
cached by content hash, so a condition shared by many nodes is decompiled
once. --format json gives the same transcript as structured nodes, scripts
and tree rows, with the timings.

Examples:
  rakata dialog show banter.dlg --game ~/games/swkotor
  rakata dialog show dan14_cutjuh.dlg --module danm14ac --game DIR --scripts
  rakata --format json dialog show banter.dlg --game DIR")]
    Show {
        /// The dialogue: a .dlg file, or NAME.dlg in the install.
        #[arg(value_name = "DIALOG")]
        dialog: String,
        /// The module whose capsules to search: the stem of modules/STEM.mod, STEM.rim and STEM_s.rim.
        #[arg(long, short, value_name = "STEM")]
        module: Option<String>,
        /// Decompile each node's action script too, not only the conditions.
        #[arg(long)]
        scripts: bool,
        /// The game the install belongs to.
        #[arg(long, value_enum, default_value_t = ProfileArg::K1)]
        profile: ProfileArg,
    },
}

fn diagnostic(code: &str, message: String) -> Diagnostic {
    let code =
        codes::lookup(code).unwrap_or_else(|| panic!("diagnostic code {code} is not registered"));
    Diagnostic::new(code, message)
}

pub fn run(ctx: &Ctx, args: &DialogArgs) -> Exit {
    let map = SourceMap::new();
    match &args.action {
        DialogAction::Show {
            dialog,
            module,
            scripts,
            profile,
        } => {
            let game = match ctx.require_game("dialog show") {
                Ok(g) => g.to_path_buf(),
                Err(e) => return e,
            };
            let mut env = ctx.envelope("dialog show");
            crate::working_on(dialog.clone());
            let start = Instant::now();
            let profile = match profile {
                ProfileArg::K1 => Profile::K1,
                ProfileArg::K2 => Profile::K2,
            };
            let fail = |e: rakata::dialog::Error| vec![diagnostic(e.code(), e.message())];
            let mut session = match Session::open(&game, profile) {
                Ok(s) => s,
                Err(e) => return ctx.finish(env, "", fail(e), &map, Exit::Ok),
            };
            session.set_module(module.as_deref().map(ModuleName::new));
            let loaded = match session.load(dialog) {
                Ok(l) => l,
                Err(e) => return ctx.finish(env, "", fail(e), &map, Exit::Ok),
            };
            let opened = start.elapsed().as_secs_f64();
            let transcript =
                session.transcript(&loaded.dialog, TranscriptOptions { scripts: *scripts });
            let seconds = start.elapsed().as_secs_f64();
            let stats = session.stats().clone();
            // The include libraries' own warnings (RW1002) concern the
            // corpus's index, not this dialogue, and repeat on every run.
            let _ = session.take_warnings();
            let mut diags: Vec<Diagnostic> = Vec::new();
            diags.extend(
                transcript
                    .warnings
                    .iter()
                    .map(|w| diagnostic(w.code, w.message.clone())),
            );
            let mut text = match &loaded.module {
                Some(m) => format!("{} (module {m})\n", loaded.location),
                None => format!("{}\n", loaded.location),
            };
            text.push_str(&transcript.text());
            text.push_str(&format!(
                "\n{} scripts decompiled, {} verified, {} from the cache, {} missing; {:.2} s (install {:.2} s, include libraries {:.2} s, decompiling {:.2} s)\n",
                stats.decompiled,
                stats.verified,
                stats.cache_hits,
                stats.missing,
                seconds,
                opened,
                stats.setup_seconds,
                stats.decompile_seconds
            ));
            env.result = json!({
                "location": loaded.location,
                "module": loaded.module,
                "transcript": transcript,
                "stats": stats,
                "seconds": seconds,
            });
            ctx.finish(env, &text, diags, &map, Exit::Ok)
        }
    }
}
