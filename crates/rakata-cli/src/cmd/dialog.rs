//! `rakata dialog`: conversations through [`rakata::dialog`]. `show` prints
//! a dialogue's transcript; `lint` reports its faults.

use std::path::PathBuf;
use std::time::Instant;

use clap::{Args, Subcommand};
use serde_json::json;

use super::Ctx;
use crate::Exit;
use crate::cli::ProfileArg;
use rakata::codes;
use rakata::diag::{Diagnostic, SourceMap};
use rakata::dialog::lint::{self, Entry, Known, Linter};
use rakata::dialog::{NodeKey, Session, TranscriptOptions};
use rakata::game::{ModuleName, Profile};

#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  rakata dialog show banter.dlg --game ~/games/swkotor
  rakata dialog show dan14_cutjuh.dlg --module danm14ac --game DIR
  rakata dialog show path/to/edited.dlg --module danm14ac --game DIR --scripts
  rakata --format json dialog show banter.dlg --game DIR
  rakata dialog lint --all --game DIR --known conformance/dialog-known.toml
  rakata dialog lint dan14_juhani.dlg --module danm14ac --game DIR")]
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
when no global layer does or only Override/ does), patch.erf, the BIFs.
Scripts and conditions are looked up the same way, so a module's dialogue
needs its module; when several modules hold it, name one (RE1105).

In the text, E3 is EntryList[3] and R0 is ReplyList[0], and each node's
line carries in brackets its stable key, the one an edit names it by:
voice:RESREF when its voice-over names it alone, else strref:N, else
text:\"TEXT\" for text it embeds, else its index, marked (fragile) since
indices move when a list is reordered. An entry with an
empty Speaker is spoken by the dialogue's owner, shown as (owner); replies
are the player's. A link to a node already shown prints `-> E3 (shown
above)`; nodes no start reaches follow the tree. Animation numbers and
camera angles are the engine's own, printed without names. A voice-over's
duration is read from the head of its file under streamwaves/; a voiced line
without one says `no file under streamwaves/`, and a line nobody voiced (a
player reply, an entry whose SoundExists is 0 or that plays a Sound instead)
prints no voice line unless the file exists. A Delay of 0xFFFFFFFF is left
out of the text and kept in the JSON.

Each condition is decompiled with the decompiler `nws decompile` uses,
include libraries folded out (with their globals, named in the source), and
verified by recompiling: [verified, 1 of 1 functions] means the source
recompiles to the original bytes. A function that does not is marked in the
source with E0602 and the reason. The starts list prints each condition's
source; the tree refers back to it. --brief prints each node's speaker and
text and the names of its conditions and script, and decompiles nothing;
it cannot be combined with --scripts. Scripts are
cached by content hash, so a condition shared by many nodes is decompiled
once. --format json gives the same transcript as structured nodes, scripts
and tree rows, with the timings; each node has its key and whether it is
fragile.

--node KEY shows only that node, with each of its links: the link's
condition, then the node it leads to with that node's key. KEY is a stable
key as the transcript prints it (voice:RESREF, strref:N, text:TEXT) or an
index (E3, R0); repeat --node for several. A key that names no node fails
with RE1102, one that names several with RE1103, one that does not parse
with RE1104. With --format json, the result holds those nodes and the
scripts they name instead of the whole transcript.

Examples:
  rakata dialog show banter.dlg --game ~/games/swkotor
  rakata dialog show dan14_cutjuh.dlg --module danm14ac --game DIR --scripts
  rakata dialog show banter.dlg --node voice:nglobebant00089_ --node R2 --game DIR
  rakata dialog show kor37_freedonnad.dlg --module korr_m37aa --game DIR --brief
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
        /// Each node's speaker and text, and the names of its conditions and script, without their source.
        #[arg(long, conflicts_with = "scripts")]
        brief: bool,
        /// Show only the node this key names, with its links and their conditions; repeatable.
        #[arg(long = "node", value_name = "KEY")]
        nodes: Vec<String>,
        /// The game the install belongs to.
        #[arg(long, value_enum, default_value_t = ProfileArg::K1)]
        profile: ProfileArg,
    },
    /// Report a dialogue's faults, each under its code: a script, condition or end script that resolves to nothing, a strref past the talk table, a voiced node without its voice-over or lip sync, a dangling link, a node no start reaches, a global a condition reads that globalcat.2da does not declare or whose name is computed, and a SoundExists that disagrees with the node's audio.
    #[command(after_help = "\
DIALOG is a .dlg file or NAME.dlg, loaded as `dialog show` loads it; several
may be named. --all lints every dialogue of the install: each one a module's
capsules hold, in that module, and each one of the global layers (Override/,
the BIFs) against the install alone, a script or lip file no global layer
holds counting as present when any module holds it, since a global dialogue
plays in whichever module starts it. With --module, --all lints that module's
dialogues only.

A node is voiced when it names a VO_ResRef and its SoundExists is not 0;
BioWare's tool writes 0 on a line it has no recording for, and the engine
shows such a line silent. Reachability is structural: from the starts along
every link, whatever the conditions answer.

--known FILE names findings already accepted, as the TOML --write-known
writes: [[finding]] tables of dialog, module, at (E3, R0, E3>R4, start>E0,
end and abort for the dialogue's EndConversation and EndConverAbort) and
code. In the JSON, a finding's key is its node's stable key, null for an end
script. Known findings are counted and not reported. The exit status is 1
when a finding is not in the known list, else 0.

Codes: rakata explain RW1101, RW1102, RW1104 to RW1110.

Examples:
  rakata dialog lint --all --game DIR --known conformance/dialog-known.toml
  rakata dialog lint dan14_cutjuh.dlg dan14_juhani.dlg --module danm14ac --game DIR
  rakata --format json dialog lint --all --module danm14ac --game DIR")]
    Lint {
        /// The dialogues: .dlg files, or NAME.dlg in the install.
        #[arg(
            value_name = "DIALOG",
            required_unless_present = "all",
            conflicts_with = "all"
        )]
        dialogs: Vec<String>,
        /// Every dialogue of the install, or of --module.
        #[arg(long)]
        all: bool,
        /// The module whose capsules to search: the stem of modules/STEM.mod, STEM.rim and STEM_s.rim.
        #[arg(long, short, value_name = "STEM")]
        module: Option<String>,
        /// Findings already accepted, which are not reported.
        #[arg(long, value_name = "FILE")]
        known: Option<PathBuf>,
        /// Write every finding of this run as a known list.
        #[arg(long, value_name = "FILE")]
        write_known: Option<PathBuf>,
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
        DialogAction::Lint {
            dialogs,
            all,
            module,
            known,
            write_known,
            profile,
        } => run_lint(
            ctx,
            LintArgs {
                dialogs,
                all: *all,
                module: module.as_deref(),
                known: known.as_deref(),
                write_known: write_known.as_deref(),
                profile: *profile,
            },
        ),
        DialogAction::Show {
            dialog,
            module,
            scripts,
            brief,
            nodes,
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
            let mut picked: Vec<String> = Vec::new();
            for k in nodes {
                let at = k
                    .parse::<NodeKey>()
                    .map_err(rakata::dialog::Error::from)
                    .and_then(|key| session.resolve(&loaded.dialog, &key));
                match at {
                    Ok(at) => {
                        let id = at.to_string();
                        if !picked.contains(&id) {
                            picked.push(id);
                        }
                    }
                    Err(e) => return ctx.finish(env, "", fail(e), &map, Exit::Ok),
                }
            }
            let opened = start.elapsed().as_secs_f64();
            let transcript = session.transcript(
                &loaded.dialog,
                TranscriptOptions {
                    scripts: *scripts,
                    brief: *brief,
                },
            );
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
            if picked.is_empty() {
                text.push_str(&transcript.text());
            } else {
                let ids: Vec<&str> = picked.iter().map(String::as_str).collect();
                text.push_str(&transcript.nodes_text(&ids));
            }
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
            env.result = if picked.is_empty() {
                json!({
                    "location": loaded.location,
                    "module": loaded.module,
                    "transcript": transcript,
                    "stats": stats,
                    "seconds": seconds,
                })
            } else {
                let shown: Vec<_> = picked.iter().filter_map(|id| transcript.node(id)).collect();
                let named: Vec<&str> = shown
                    .iter()
                    .flat_map(|n| {
                        n.script
                            .iter()
                            .chain(n.links.iter().filter_map(|l| l.condition.as_ref()))
                    })
                    .map(String::as_str)
                    .collect();
                let scripts: Vec<_> = transcript
                    .scripts
                    .iter()
                    .filter(|s| named.iter().any(|n| n.eq_ignore_ascii_case(&s.name)))
                    .collect();
                json!({
                    "location": loaded.location,
                    "module": loaded.module,
                    "dialog": transcript.dialog,
                    "nodes": shown,
                    "scripts": scripts,
                    "stats": stats,
                    "seconds": seconds,
                })
            };
            ctx.finish(env, &text, diags, &map, Exit::Ok)
        }
    }
}

struct LintArgs<'a> {
    dialogs: &'a [String],
    all: bool,
    module: Option<&'a str>,
    known: Option<&'a std::path::Path>,
    write_known: Option<&'a std::path::Path>,
    profile: ProfileArg,
}

fn run_lint(ctx: &Ctx, a: LintArgs<'_>) -> Exit {
    let map = SourceMap::new();
    let game = match ctx.require_game("dialog lint") {
        Ok(g) => g.to_path_buf(),
        Err(e) => return e,
    };
    let mut env = ctx.envelope("dialog lint");
    let start = Instant::now();
    let fail = |code: &str, message: String| vec![diagnostic(code, message)];
    let known = match a.known.map(Known::read).transpose() {
        Ok(k) => k.unwrap_or_default(),
        Err(m) => return ctx.finish(env, "", fail(lint::BAD_KNOWN, m), &map, Exit::Ok),
    };
    let profile = match a.profile {
        ProfileArg::K1 => Profile::K1,
        ProfileArg::K2 => Profile::K2,
    };
    let mut linter = match Linter::open(&game, profile) {
        Ok(l) => l,
        Err(e) => return ctx.finish(env, "", fail(e.code(), e.message()), &map, Exit::Ok),
    };
    let module = a.module.map(ModuleName::new);
    let mut diags: Vec<Diagnostic> = Vec::new();
    let mut runs = Vec::new();
    if a.all {
        let targets = match linter.targets(module.as_ref()) {
            Ok(t) => t,
            Err(e) => return ctx.finish(env, "", fail(e.code(), e.message()), &map, Exit::Ok),
        };
        for t in &targets {
            runs.push((t.id.to_string(), linter.lint_target(t)));
        }
    } else {
        for d in a.dialogs {
            crate::working_on(d.clone());
            runs.push((d.clone(), linter.lint_named(d, module.clone())));
        }
    }
    let mut found = Known::default();
    let (mut dialogues, mut new, mut accepted) = (0usize, 0usize, 0usize);
    let mut results = Vec::new();
    let mut per_code: std::collections::BTreeMap<&str, usize> = Default::default();
    for (name, run) in runs {
        let linted = match run {
            Ok(l) => l,
            Err(e) => {
                diags.push(diagnostic(e.code(), format!("{name}: {}", e.message())));
                continue;
            }
        };
        dialogues += 1;
        let dialog = linted.loaded.dialog.name().to_string();
        let module = linted.loaded.module.clone();
        let mut items = Vec::new();
        for f in &linted.findings {
            let entry = Entry::of(&dialog, module.as_deref(), f);
            let is_known = known.contains(&entry);
            *per_code.entry(f.code).or_default() += 1;
            let message = match &module {
                Some(m) => format!("{} (module {m})", f.message(&dialog)),
                None => f.message(&dialog),
            };
            if is_known {
                accepted += 1;
            } else {
                new += 1;
                diags.push(diagnostic(f.code, message.clone()));
            }
            items.push(json!({
                "code": f.code,
                "at": entry.at,
                "key": f.key.as_ref().map(ToString::to_string),
                "subject": f.subject,
                "via": f.via,
                "known": is_known,
                "message": message,
            }));
            found.findings.insert(entry);
        }
        if !items.is_empty() {
            results.push(json!({
                "dialog": dialog,
                "module": module,
                "location": linted.loaded.location,
                "findings": items,
            }));
        }
    }
    if let Some(path) = a.write_known
        && let Err(e) = std::fs::write(path, found.to_toml())
    {
        diags.push(diagnostic(
            "RE1306",
            format!("cannot write {}: {e}", path.display()),
        ));
    }
    let unmatched: Vec<&Entry> = if a.all && a.module.is_none() {
        known
            .findings
            .iter()
            .filter(|e| !found.contains(e))
            .collect()
    } else {
        Vec::new()
    };
    let seconds = start.elapsed().as_secs_f64();
    let mut text = format!(
        "{dialogues} dialogues, {} findings: {new} new, {accepted} known",
        new + accepted
    );
    if !unmatched.is_empty() {
        text.push_str(&format!(
            "; {} known findings no longer found",
            unmatched.len()
        ));
    }
    text.push_str(&format!("; {seconds:.2} s\n"));
    for (code, n) in &per_code {
        text.push_str(&format!("  {code}  {n}\n"));
    }
    env.result = json!({
        "dialogues": dialogues,
        "new": new,
        "known": accepted,
        "by_code": per_code,
        "stale_known": unmatched,
        "results": results,
        "seconds": seconds,
    });
    let exit = if new > 0 { Exit::Errors } else { Exit::Ok };
    ctx.finish(env, &text, diags, &map, exit)
}
