//! `nws check`: the front end and semantic analysis, with no output file.

use super::Ctx;
use crate::Exit;
use crate::io::{display_name, read_input};
use nwscript::diag::SourceMap;
use nwscript::{Envelope, InputRecord, Resolver};
use std::path::{Path, PathBuf};

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  nws --game k1 check k_my_script.nss                      nwscript.nss and includes beside it
  nws --game k1 -I ~/kotor/scripts check a.nss b.nss       includes from a directory
  nws --game k1 --nwscript ~/kotor/nwscript.nss check *.nss
  nws --game k1 --format json check k_my_script.nss         one JSON document

Each file is checked on its own: includes are searched in its directory,
then each -I directory in order, ignoring case. A file with neither
`void main()` nor `int StartingConditional()` is checked as a library,
such as an include file. Exit 0 when no file has errors, 1 otherwise.")]
pub struct CheckArgs {
    /// The NWScript source files to check.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
}

/// The directory a file's includes are searched in first.
fn own_dir(path: &Path) -> PathBuf {
    match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

pub fn run(ctx: &Ctx, args: &CheckArgs) -> Exit {
    let game = match ctx.require_game("check") {
        Ok(g) => g,
        Err(e) => return e,
    };
    let mut env = Envelope::new("check", Some(game));
    let mut map = SourceMap::new();
    let mut checker = ctx.checker(game);
    let mut diags = Vec::new();
    let mut files = Vec::new();
    let mut failed = 0;
    for path in &args.files {
        let name = display_name(path);
        crate::working_on(name.clone());
        let (bytes, input) = match read_input(path, "source") {
            Ok(x) => x,
            Err(e) => {
                eprintln!("error: {e}");
                return Exit::Usage;
            }
        };
        push_input(&mut env.inputs, input);
        let mut resolver = ctx.resolver(Some(own_dir(path)));
        let out = checker.check(&mut resolver, &mut map, &name, &bytes);
        for i in resolver.inputs() {
            push_input(&mut env.inputs, i.clone());
        }
        // A file's status counts what remains after lints, directives and levels.
        let found = ctx.linted(out.analysis.as_ref(), out.diagnostics, &map);
        let errors = found.iter().filter(|d| d.is_error()).count();
        if errors > 0 {
            failed += 1;
        }
        let entry = out
            .analysis
            .as_ref()
            .and_then(|a| a.program.entry)
            .map(|e| e.kind.name());
        files.push(serde_json::json!({
            "path": name,
            "ok": errors == 0,
            "entry": entry,
            "errors": errors,
            "warnings": found.len() - errors,
        }));
        diags.extend(found);
    }
    let text = if failed == 0 {
        format!("checked {}: no errors", plural(args.files.len()))
    } else {
        format!("checked {}: {failed} with errors", plural(args.files.len()))
    };
    env.result = serde_json::json!({ "files": files });
    ctx.finish(env, &text, diags, &map, Exit::Ok)
}

fn plural(n: usize) -> String {
    if n == 1 {
        "1 file".into()
    } else {
        format!("{n} files")
    }
}

/// Record an input once, however many scripts read it.
fn push_input(inputs: &mut Vec<InputRecord>, i: InputRecord) {
    if !inputs.contains(&i) {
        inputs.push(i);
    }
}
