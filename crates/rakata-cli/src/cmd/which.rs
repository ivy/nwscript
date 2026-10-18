//! `rakata which NAME.EXT`: the copy of a resource the engine loads, every
//! other copy the install holds, and why each lost.

use super::Ctx;
use crate::Exit;
use crate::cli::ProfileArg;
use rakata::codes;
use rakata::diag::{Diagnostic, SourceMap, codes::Code};
use rakata::game::{Candidate, Install, Location, ModuleName, Profile, Resolution, Verdict};
use rakata_res::ResId;
use serde_json::{Value, json};
use std::path::Path;

/// The arguments of `which`, as `cli.rs` parses them.
pub struct Args<'a> {
    pub name: Option<&'a str>,
    pub module: Option<&'a str>,
    pub profile: ProfileArg,
    pub duplicates: bool,
}

/// The registered code for an error this crate's dependencies report. An
/// unregistered one is a bug in the registry, not a user error.
fn code_of(id: &str) -> &'static Code {
    codes::lookup(id).unwrap_or_else(|| panic!("diagnostic code {id} is not registered"))
}

fn error(e: &rakata::game::Error) -> Diagnostic {
    Diagnostic::new(code_of(e.code()), e.message())
}

pub fn run(ctx: &Ctx, args: Args<'_>) -> Exit {
    let game = match ctx.require_game("which") {
        Ok(g) => g,
        Err(exit) => return exit,
    };
    let profile = match args.profile {
        ProfileArg::K1 => Profile::K1,
        ProfileArg::K2 => Profile::K2,
    };
    let mut env = ctx.envelope("which");
    let map = SourceMap::new();
    crate::working_on(game.display().to_string());
    let install = match Install::open(game, profile) {
        Ok(i) => i,
        Err(e) => return ctx.finish(env, "", vec![error(&e)], &map, Exit::Ok),
    };

    if args.duplicates {
        let groups = install.override_case_variants();
        let diags: Vec<Diagnostic> = groups.iter().map(|g| case_variants(g)).collect();
        let text = groups
            .iter()
            .map(|g| {
                g.iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("  ")
            })
            .collect::<Vec<_>>()
            .join("\n");
        env.result = json!({
            "duplicates": groups.iter().map(|g| g.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()).collect::<Vec<_>>(),
        });
        let text = if groups.is_empty() {
            "Override holds no case-variant duplicates".to_owned()
        } else {
            text
        };
        return ctx.finish(env, &text, diags, &map, Exit::Ok);
    }

    let Some(name) = args.name else {
        eprintln!(
            "error: `rakata which` needs NAME.EXT, or --duplicates to list Override's case variants"
        );
        return Exit::Usage;
    };
    crate::working_on(name);
    let id = match ResId::parse(name) {
        Ok(id) => id,
        Err(e) => {
            let d = Diagnostic::new(code_of(e.code()), e.to_string());
            return ctx.finish(env, "", vec![d], &map, Exit::Ok);
        }
    };
    let module = args.module.map(ModuleName::new);
    let resolution = match install.resolve(id, module.as_ref()) {
        Ok(r) => r,
        Err(e) => return ctx.finish(env, "", vec![error(&e)], &map, Exit::Ok),
    };

    let mut diags = Vec::new();
    if let Some(e) = resolution.not_found() {
        diags.push(error(&e));
    }
    if resolution.is_ambiguous() {
        let paths: Vec<&Path> = resolution
            .case_variants()
            .iter()
            .map(|c| c.location.path())
            .collect();
        diags.push(case_variants(&paths));
    }
    env.result = to_json(&resolution);
    let text = render(&resolution, game);
    ctx.finish(env, &text, diags, &map, Exit::Ok)
}

/// The warning for one group of Override files differing only in case.
fn case_variants<P: AsRef<Path>>(paths: &[P]) -> Diagnostic {
    let names: Vec<String> = paths
        .iter()
        .map(|p| p.as_ref().display().to_string())
        .collect();
    Diagnostic::new(
        code_of(rakata::game::codes::CASE_VARIANTS),
        format!(
            "Override holds {} files that differ only in case: {}",
            names.len(),
            names.join(", ")
        ),
    )
    .help("remove all but one; which the engine opens is not established")
}

fn location_json(l: &Location) -> Value {
    match l {
        Location::File { path } => json!({ "kind": "file", "path": path }),
        Location::Capsule { path, member } => {
            json!({ "kind": "capsule", "path": path, "member": member.to_string() })
        }
        Location::Bif { path, key, index } => {
            json!({ "kind": "bif", "path": path, "key": key, "index": index })
        }
    }
}

fn candidate_json(c: &Candidate) -> Value {
    let verdict = match &c.verdict {
        Verdict::Wins => json!({ "status": "wins" }),
        Verdict::Ambiguous => json!({ "status": "ambiguous" }),
        Verdict::Shadowed { by, reason } => {
            json!({ "status": "shadowed", "by": by, "reason": reason })
        }
    };
    json!({
        "layer": c.layer.name(),
        "location": location_json(&c.location),
        "len": c.len,
        "sha256": c.hash.to_hex(),
        "verdict": verdict,
    })
}

fn to_json(r: &Resolution) -> Value {
    json!({
        "id": r.id.to_string(),
        "module": r.module.as_ref().map(ToString::to_string),
        "ambiguous": r.is_ambiguous(),
        "winner": r.winner().map(candidate_json),
        "candidates": r.candidates.iter().map(candidate_json).collect::<Vec<_>>(),
    })
}

/// The text report: the id, then one line per candidate, the winner first.
fn render(r: &Resolution, game: &Path) -> String {
    let mut out = match &r.module {
        Some(m) => format!("{} (module {m})\n", r.id),
        None => format!("{}\n", r.id),
    };
    if r.candidates.is_empty() {
        out.push_str("  not in the install\n");
        return out;
    }
    for c in &r.candidates {
        let (word, why) = match &c.verdict {
            Verdict::Wins if r.is_ambiguous() => {
                ("wins?", String::from("a case variant stands beside it"))
            }
            Verdict::Wins => ("wins", String::new()),
            Verdict::Ambiguous => (
                "rival",
                String::from("differs only in case from the winner"),
            ),
            Verdict::Shadowed { reason, .. } => ("loses", reason.clone()),
        };
        let shown = relative(&c.location, game);
        out.push_str(&format!(
            "  {word:<6}{shown}  {} bytes  sha256 {}",
            c.len, c.hash
        ));
        if !why.is_empty() {
            out.push_str(&format!("  ({why})"));
        }
        out.push('\n');
    }
    out
}

/// The location with the install's path dropped from the front.
fn relative(l: &Location, game: &Path) -> String {
    let path = l.path().strip_prefix(game).unwrap_or(l.path());
    match l {
        Location::File { .. } => path.display().to_string(),
        Location::Capsule { member, .. } => format!("{}:{member}", path.display()),
        Location::Bif { index, .. } => format!("{}[{index}]", path.display()),
    }
}
