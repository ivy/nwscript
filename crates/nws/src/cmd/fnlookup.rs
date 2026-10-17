//! `nws fn`: an engine routine's prototype and comment from nwscript.nss.

use super::Ctx;
use crate::Exit;
use nwscript::Envelope;
use nwscript::diag::SourceMap;
use nwscript::sema::{EngineTable, Function, Type};
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  nws -I corpus fn GetGlobalNumber
  nws -I corpus fn --grep partymember
  nws -I corpus --format json fn ActionStartConversation

nwscript.nss comes from --nwscript or an -I directory (or the current
directory). Matching is case-insensitive; --grep also searches the comments.")]
pub struct Args {
    /// The routine's name.
    #[arg(required_unless_present = "grep")]
    pub name: Option<String>,
    /// List every routine whose name or comment contains this text.
    #[arg(long, value_name = "TEXT", conflicts_with = "name")]
    pub grep: Option<String>,
}

fn type_name(t: &Type, table: &EngineTable) -> String {
    match t {
        Type::Engine(n) => table
            .structures
            .get(usize::from(*n))
            .cloned()
            .unwrap_or_else(|| t.to_string()),
        other => other.to_string(),
    }
}

/// The prototype as nwscript.nss would write it.
pub fn prototype(f: &Function, table: &EngineTable) -> String {
    let params: Vec<String> = f
        .params
        .iter()
        .map(|p| match &p.default {
            Some(d) => format!("{} {}={d}", type_name(&p.ty, table), p.name),
            None => format!("{} {}", type_name(&p.ty, table), p.name),
        })
        .collect();
    format!(
        "{} {}({});",
        type_name(&f.ret, table),
        f.name,
        params.join(", ")
    )
}

fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (
        a.to_lowercase().chars().collect(),
        b.to_lowercase().chars().collect(),
    );
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=b.len() {
            let cur = row[j];
            row[j] = (row[j] + 1)
                .min(row[j - 1] + 1)
                .min(prev + usize::from(a[i - 1] != b[j - 1]));
            prev = cur;
        }
    }
    row[b.len()]
}

pub fn run(ctx: &Ctx, args: &Args) -> Exit {
    let mut resolver = ctx.resolver(Some(PathBuf::from(".")));
    let mut map = SourceMap::new();
    let table = match nwscript::engine::load_table(&mut resolver, &mut map) {
        Ok(Some((t, _))) => t,
        Ok(None) => {
            eprintln!("error: no nwscript.nss: pass --nwscript FILE or -I DIR");
            return Exit::Usage;
        }
        Err(e) => {
            eprintln!("error: reading nwscript.nss: {e}");
            return Exit::Usage;
        }
    };
    let mut env = Envelope::new("fn", ctx.game);
    env.inputs
        .extend(nwscript::Resolver::inputs(&resolver).iter().cloned());
    let entry = |n: usize, f: &Function| {
        serde_json::json!({
            "routine": n,
            "name": f.name,
            "prototype": prototype(f, &table),
            "doc": f.doc,
            "recovered": f.recovered,
        })
    };
    let matched: Vec<(usize, &Function)> = match (&args.name, &args.grep) {
        (Some(name), _) => table
            .functions
            .iter()
            .enumerate()
            .filter(|(_, f)| f.name.eq_ignore_ascii_case(name))
            .collect(),
        (None, Some(g)) => {
            let g = g.to_lowercase();
            table
                .functions
                .iter()
                .enumerate()
                .filter(|(_, f)| {
                    f.name.to_lowercase().contains(&g) || f.doc.to_lowercase().contains(&g)
                })
                .collect()
        }
        (None, None) => unreachable!("clap requires a name or --grep"),
    };
    if matched.is_empty() {
        let wanted = args
            .name
            .clone()
            .or_else(|| args.grep.clone())
            .unwrap_or_default();
        let mut close: Vec<(usize, &str)> = table
            .functions
            .iter()
            .map(|f| (distance(&f.name, &wanted), f.name.as_str()))
            .collect();
        close.sort();
        let close: Vec<&str> = close
            .into_iter()
            .take(3)
            .filter(|(d, _)| *d <= wanted.len() / 2 + 1)
            .map(|(_, n)| n)
            .collect();
        let hint = if close.is_empty() {
            String::new()
        } else {
            format!("; did you mean {}?", close.join(", "))
        };
        eprintln!("error: no engine routine matches `{wanted}`{hint}");
        env.result = serde_json::json!({ "matches": [], "suggestions": close });
        ctx.finish(env, "", Vec::new(), &map, Exit::Ok);
        return Exit::Errors;
    }
    let mut text = String::new();
    for (n, f) in &matched {
        if args.name.is_some() {
            for line in f.doc.lines() {
                text.push_str(&format!("// {line}\n"));
            }
        }
        text.push_str(&format!(
            "{}    // routine {n}{}\n",
            prototype(f, &table),
            if f.recovered {
                ", signature recovered"
            } else {
                ""
            }
        ));
    }
    env.result = serde_json::json!({ "matches": matched.iter().map(|(n, f)| entry(*n, f)).collect::<Vec<_>>() });
    ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
}
