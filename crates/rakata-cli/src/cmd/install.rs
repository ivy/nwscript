//! `rakata install TSLPATCHDATA --game DIR [--option N] --dry-run`: read the
//! installer's files and print the plan. The executor is the next piece;
//! until it lands, an install without `--dry-run` is refused.

use super::Ctx;
use crate::Exit;
use rakata::diag::{InputRecord, SourceMap};
use rakata::patch::{self, LoadError, Plan};
use std::path::Path;

pub fn run(ctx: &Ctx, tslpatchdata: &Path, option: Option<u32>, dry_run: bool) -> Exit {
    let Ok(game) = ctx.require_game("install") else {
        return Exit::Usage;
    };
    if !dry_run {
        eprintln!(
            "error: `rakata install` can only plan today; pass --dry-run to print what installing into `{}` would do",
            game.display()
        );
        return Exit::Usage;
    }
    crate::working_on(tslpatchdata.display().to_string());
    let mut loaded = match patch::load(tslpatchdata, option) {
        Ok(l) => l,
        Err(LoadError::Io { path, source }) => {
            let mut env = ctx.envelope("install");
            env.result = serde_json::json!({ "dry_run": true });
            let d = rakata::diag::Diagnostic::new(
                &rakata::codes::RE0001,
                format!("cannot read `{}`: {source}", path.display()),
            );
            return ctx.finish(env, "", vec![d], &SourceMap::new(), Exit::Ok);
        }
        Err(e) => {
            eprintln!("error: {e}");
            return Exit::Usage;
        }
    };
    // The changes file is the last source read; the plan's findings point into it.
    let changes_file = loaded.sources.len() - 1;
    let plan = Plan::build(&loaded.changes, changes_file, &mut loaded.issues);
    let mut map = SourceMap::new();
    let diags = patch::diagnostics(&loaded, &mut map);
    let mut env = ctx.envelope("install");
    env.inputs = loaded
        .sources
        .iter()
        .map(|s| InputRecord {
            role: "changes".to_string(),
            path: s.path.display().to_string(),
            sha256: sha256_hex(&s.bytes),
        })
        .collect();
    env.result = serde_json::json!({
        "dry_run": true,
        "tslpatchdata": tslpatchdata.display().to_string(),
        "data_dir": loaded.data_dir.display().to_string(),
        "option": loaded.option.as_ref().map(|o| serde_json::json!({
            "number": o.number,
            "section": o.section,
            "name": o.name,
            "ini_name": o.ini_name,
            "data_path": o.data_path,
        })),
        "settings": loaded.changes.settings,
        "plan": plan,
    });
    let mut text = String::new();
    if let Some(o) = &loaded.option {
        text.push_str(&format!(
            "option {}: {} ({})\n",
            o.number,
            o.name.as_deref().unwrap_or(&o.section),
            o.ini_name
        ));
    }
    text.push_str(&format!(
        "plan for {} into {} ({} steps)\n",
        loaded.data_dir.display(),
        game.display(),
        plan.steps.len()
    ));
    text.push_str(&plan.to_string());
    ctx.finish(env, &text, diags, &map, Exit::Ok)
}

fn sha256_hex(bytes: &[u8]) -> String {
    rakata::res::ContentHash::of(bytes).to_string()
}
