//! `rakata install TSLPATCHDATA --game DIR [--option N] [--dry-run] [--report FILE]`:
//! read the installer's files, build the plan, and either print it or run
//! it against the install, every write read back. The report is the tree:
//! each operation, its target and the hash found there; exit 1 when any
//! operation failed or a write did not land.

use super::Ctx;
use crate::Exit;
use rakata::diag::{InputRecord, SourceMap};
use rakata::patch::{self, Executor, LoadError, Plan};
use std::path::Path;

pub fn run(
    ctx: &Ctx,
    tslpatchdata: &Path,
    option: Option<u32>,
    dry_run: bool,
    report_file: Option<&Path>,
) -> Exit {
    let Ok(game) = ctx.require_game("install") else {
        return Exit::Usage;
    };
    crate::working_on(tslpatchdata.display().to_string());
    let mut loaded = match patch::load(tslpatchdata, option) {
        Ok(l) => l,
        Err(LoadError::Io { path, source }) => {
            let mut env = ctx.envelope("install");
            env.result = serde_json::json!({ "dry_run": dry_run });
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
    let mut diags = patch::diagnostics(&loaded, &mut map);
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
    let mut result = serde_json::json!({
        "dry_run": dry_run,
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
    if dry_run {
        text.push_str(&format!(
            "plan for {} into {} ({} steps)\n",
            loaded.data_dir.display(),
            game.display(),
            plan.steps.len()
        ));
        text.push_str(&plan.to_string());
        env.result = result;
        return ctx.finish(env, &text, diags, &map, Exit::Ok);
    }

    let started = std::time::Instant::now();
    let report = match Executor::open(game, tslpatchdata, &loaded) {
        Ok(exec) => exec.run(&plan),
        Err(f) => patch::Report {
            failure: Some(f),
            ..Default::default()
        },
    };
    let elapsed = started.elapsed();
    let src = &loaded.sources[changes_file];
    let id = map.add(src.path.display().to_string(), src.bytes.clone());
    diags.extend(patch::report_diagnostics(&report, Some((id, &src.bytes))));
    result["report"] = serde_json::to_value(&report).expect("the report serializes");
    result["ok"] = serde_json::Value::Bool(report.ok());
    result["seconds"] = serde_json::json!(elapsed.as_secs_f64());
    text.push_str(&format!(
        "installed {} into {} in {:.2}s\n",
        loaded.data_dir.display(),
        game.display(),
        elapsed.as_secs_f64()
    ));
    text.push_str(&report.to_string());
    if let Some(file) = report_file {
        let doc = serde_json::to_string_pretty(&result).expect("the report serializes");
        if let Err(e) = std::fs::write(file, doc + "\n") {
            eprintln!(
                "error: cannot write the report to `{}`: {e}",
                file.display()
            );
            return Exit::Usage;
        }
    }
    env.result = result;
    let exit = if report.ok() { Exit::Ok } else { Exit::Errors };
    ctx.finish(env, &text, diags, &map, exit)
}

fn sha256_hex(bytes: &[u8]) -> String {
    rakata::res::ContentHash::of(bytes).to_string()
}
