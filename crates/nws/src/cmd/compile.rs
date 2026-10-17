//! `nws compile`: NWScript source to NCS bytecode.

use super::Ctx;
use crate::Exit;
use crate::cli::Format;
use crate::io::{display_name, read_input, write_output};
use nwscript::diag::{Diagnostic, SourceMap, has_errors};
use nwscript::ncs::asm::{PrintOptions, print};
use nwscript::{Envelope, InputRecord, Resolver};
use std::path::{Path, PathBuf};

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  nws --game k1 compile k_my_script.nss                  writes k_my_script.ncs beside it
  nws --game k1 -I ~/kotor/scripts compile a.nss -o a.ncs
  nws --game k1 compile *.nss --out-dir build --emit ncs,asm
  nws --game k1 --format json compile k_my_script.nss     one JSON document

Each file is checked, then compiled when no error remains after lint levels
apply. Includes and nwscript.nss are searched in the file's directory, then
each -I directory in order, ignoring case; each is read and parsed once. A
file with errors writes nothing, and the command goes on to the next; it
exits 1 at the end if any file had errors.")]
pub struct CompileArgs {
    /// The NWScript source files to compile.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// The output file, for a single input.
    #[arg(short, long, value_name = "FILE", conflicts_with = "out_dir")]
    pub output: Option<PathBuf>,
    /// Write each output into this directory instead of beside its input.
    #[arg(long, value_name = "DIR")]
    pub out_dir: Option<PathBuf>,
    /// What to write: `ncs` (the bytecode), `asm` (its assembly, beside it), or both.
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "ncs",
        value_name = "KINDS"
    )]
    pub emit: Vec<Emit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Emit {
    Ncs,
    Asm,
}

fn own_dir(path: &Path) -> PathBuf {
    match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// Where a file's bytecode goes.
fn ncs_path(args: &CompileArgs, input: &Path) -> PathBuf {
    if let Some(o) = &args.output {
        return o.clone();
    }
    let name = input.with_extension("ncs");
    match &args.out_dir {
        Some(d) => d.join(name.file_name().unwrap_or_default()),
        None => name,
    }
}

pub fn run(ctx: &Ctx, args: &CompileArgs) -> Exit {
    let game = match ctx.require_game("compile") {
        Ok(g) => g,
        Err(e) => return e,
    };
    if args.output.is_some() && args.files.len() > 1 {
        eprintln!("error: -o names one output; with several inputs, use --out-dir DIR");
        return Exit::Usage;
    }
    if args.files.iter().any(|f| f.as_os_str() == "-") {
        eprintln!("error: compile reads files, not stdin: includes are found beside the input");
        return Exit::Usage;
    }
    if let Some(d) = &args.out_dir
        && let Err(e) = std::fs::create_dir_all(d)
    {
        eprintln!("error: cannot create `{}`: {e}", d.display());
        return Exit::Usage;
    }
    let mut env = Envelope::new("compile", Some(game));
    let mut map = SourceMap::new();
    let mut checker = ctx.checker(game);
    let mut all: Vec<Diagnostic> = Vec::new();
    let mut files = Vec::new();
    let mut failed = 0;
    let mut summary = Vec::new();
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
        // Whether this file compiles is decided after the lint levels: what
        // they demote no longer stops it.
        let mut diags = ctx.leveled(out.diagnostics);
        let mut outputs = Vec::new();
        if !has_errors(&diags)
            && let Some(a) = &out.analysis
        {
            match nwscript::compile::compile(&a.program, game) {
                Ok(program) => {
                    let ncs = ncs_path(args, path);
                    let mut write = |p: &Path, b: &[u8]| match write_output(p, b) {
                        Ok(rec) => outputs.push(rec),
                        Err(e) => diags.push(Diagnostic::new(&nwscript::diag::codes::E0001, e)),
                    };
                    if args.emit.contains(&Emit::Ncs) {
                        write(&ncs, &nwscript::ncs::encode(&program));
                    }
                    if args.emit.contains(&Emit::Asm) {
                        let table = out.table.clone();
                        let names = |r: u16| {
                            table
                                .as_ref()
                                .and_then(|t| t.functions.get(usize::from(r)))
                                .map(|f| f.name.clone())
                        };
                        let text = print(
                            &program,
                            &PrintOptions {
                                raw: false,
                                routine_name: Some(&names),
                            },
                        );
                        write(&ncs.with_extension("asm"), text.as_bytes());
                    }
                }
                Err(d) => diags.push(d),
            }
        }
        let errors = diags.iter().filter(|d| d.is_error()).count();
        if let Some(first) = diags.iter().find(|d| d.is_error()) {
            failed += 1;
            summary.push(format!(
                "{name}: error[{}]: {}; nothing written",
                first.code.id, first.message
            ));
        }
        files.push(serde_json::json!({
            "path": name,
            "ok": errors == 0,
            "entry": out.analysis.as_ref().and_then(|a| a.program.entry).map(|e| e.kind.name()),
            "outputs": outputs.clone(),
            "errors": errors,
            "warnings": diags.len() - errors,
        }));
        env.outputs.extend(outputs);
        all.extend(diags);
    }
    let n = args.files.len();
    summary.push(match failed {
        0 => format!("compiled {}", plural(n)),
        f => format!("compiled {} of {}", n - f, plural(n)),
    });
    let text = match ctx.out.format {
        Format::Json => String::new(),
        Format::Text => summary.join("\n"),
    };
    env.result = serde_json::json!({ "files": files });
    ctx.finish(env, &text, all, &map, Exit::Ok)
}

fn plural(n: usize) -> String {
    if n == 1 {
        "1 file".into()
    } else {
        format!("{n} files")
    }
}

fn push_input(inputs: &mut Vec<InputRecord>, i: InputRecord) {
    if !inputs.contains(&i) {
        inputs.push(i);
    }
}
