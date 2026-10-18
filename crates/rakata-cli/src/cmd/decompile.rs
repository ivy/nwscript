//! `rakata decompile`: one script, the copy the engine loads, decompiled
//! through [`rakata::show::decompile`] as `dialog show` decompiles a
//! dialogue's conditions.

use clap::Args;
use serde_json::json;

use super::Ctx;
use super::inspect::profile;
use crate::Exit;
use crate::cli::ProfileArg;
use rakata::codes;
use rakata::diag::{Diagnostic, SourceMap};
use rakata::dialog::Session;
use rakata::game::ModuleName;
use rakata::show::{self, ScriptStatus};

#[derive(Args, Debug)]
#[command(after_help = "\
NAME.ncs is looked up as `rakata which` finds it: Override/, the module's
capsules (with --module), patch.erf, the BIFs. Without --module, a script no
global layer holds is looked for in every module, and the one module holding
it is used; several are RE2002. The extension may be left out.

The script is decompiled with the decompiler `nws decompile` uses, against
the install's own nwscript.nss, with the include libraries of the install's
scripts folded out (`#include \"k_inc_utility\"` in place of the functions it
supplies), and verified by recompiling: [verified, 3 of 3 functions] means
the source recompiles to the original bytes. The first lines are comments:
where the copy lies, the status, and what folding did, so the output is a
source file as it stands. A function that did not verify is reported
(RW2001). --format json gives the source, status and counts.

Examples:
  rakata decompile k35_uth_hassword.ncs --module korr_m35aa --game DIR
  rakata decompile k_pdan_juhani31 --game DIR > k_pdan_juhani31.nss
  rakata --format json decompile k37_opensarc.ncs --module korr_m37aa --game DIR")]
pub struct DecompileArgs {
    /// The script, as NAME.ncs or NAME.
    #[arg(value_name = "NAME.ncs")]
    pub name: String,
    /// The module whose capsules to search: the stem of modules/STEM.mod, STEM.rim and STEM_s.rim.
    #[arg(long, short, value_name = "STEM")]
    pub module: Option<String>,
    /// The game the install belongs to.
    #[arg(long, value_enum, default_value_t = ProfileArg::K1)]
    pub profile: ProfileArg,
}

fn diagnostic(code: &str, message: String) -> Diagnostic {
    let code =
        codes::lookup(code).unwrap_or_else(|| panic!("diagnostic code {code} is not registered"));
    Diagnostic::new(code, message)
}

pub fn run(ctx: &Ctx, args: &DecompileArgs) -> Exit {
    let map = SourceMap::new();
    let game = match ctx.require_game("decompile") {
        Ok(g) => g.to_path_buf(),
        Err(e) => return e,
    };
    let mut env = ctx.envelope("decompile");
    crate::working_on(args.name.clone());
    let mut session = match Session::open(&game, profile(args.profile)) {
        Ok(s) => s,
        Err(e) => {
            let d = diagnostic(e.code(), e.message());
            return ctx.finish(env, "", vec![d], &map, Exit::Ok);
        }
    };
    session.set_module(args.module.as_deref().map(ModuleName::new));
    let done = match show::decompile(&mut session, &args.name) {
        Ok(d) => d,
        Err(d) => return ctx.finish(env, "", vec![d], &map, Exit::Ok),
    };
    // The include libraries' own warnings (RW1002) concern the install's
    // libraries, not this script, as in `dialog show`.
    let _ = session.take_warnings();
    let s = &done.script;
    let mut diags = Vec::new();
    if s.status == ScriptStatus::Unverified {
        diags.push(diagnostic(
            show::ids::UNVERIFIED,
            format!(
                "{}.ncs: {} of {} functions verified",
                s.name, s.verified, s.functions
            ),
        ));
    }
    let location = s.location.clone().unwrap_or_default();
    let mut text = match &done.module {
        Some(m) => format!("// {location} (module {m})\n"),
        None => format!("// {location}\n"),
    };
    text.push_str(&format!(
        "// {} [{}, {} of {} functions]\n",
        s.name,
        s.status.text(),
        s.verified,
        s.functions
    ));
    if let Some(note) = &s.note {
        text.push_str(&format!("// {note}\n"));
    }
    text.push_str(s.source.as_deref().unwrap_or_default());
    env.result = json!({
        "location": location,
        "module": done.module,
        "script": s,
    });
    ctx.finish(env, &text, diags, &map, Exit::Ok)
}
