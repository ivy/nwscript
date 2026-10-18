//! `nws disasm` and `nws asm`: NCS bytecode to assembly text and back.

use super::Ctx;
use crate::Exit;
use crate::cli::Format;
use crate::io::{display_name, read_input, write_output};
use nwscript::bytecode::{self, PrintOptions};
use nwscript::diag::SourceMap;
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  nws disasm k_act_mispaz.ncs                 print the assembly
  nws disasm k_act_mispaz.ncs -o out.asm      write it to a file
  nws disasm --raw k_act_mispaz.ncs           offsets on each line, relative jumps
  nws disasm x.ncs | nws asm - -o y.ncs       y.ncs is byte-identical to x.ncs
  nws disasm --cfg x.ncs | dot -Tsvg > x.svg   the control-flow graph

The syntax is documented in docs/asm.md.")]
pub struct DisasmArgs {
    /// The NCS file, or - for stdin.
    pub input: PathBuf,
    /// Write the assembly here instead of stdout.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// Prefix each line with its file offset and keep jumps as relative offsets.
    #[arg(long)]
    pub raw: bool,
    /// Print the control-flow graph as Graphviz instead.
    #[arg(long)]
    pub cfg: bool,
}

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  nws asm k_act_mispaz.asm                    writes k_act_mispaz.ncs
  nws asm k_act_mispaz.asm -o out.ncs
  nws disasm x.ncs | nws asm - -o y.ncs

The syntax is documented in docs/asm.md.")]
pub struct AsmArgs {
    /// The assembly file, or - for stdin.
    pub input: PathBuf,
    /// Where to write the NCS: defaults to the input with an .ncs extension; - for stdout.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,
}

pub fn disasm(ctx: &Ctx, args: &DisasmArgs) -> Exit {
    crate::working_on(display_name(&args.input));
    let (bytes, input) = match read_input(&args.input, "bytecode") {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            return Exit::Usage;
        }
    };
    let mut env = super::envelope("disasm", ctx.game);
    env.inputs.push(input);
    let mut map = SourceMap::new();
    // Routine names come from nwscript.nss when one resolves: --nwscript,
    // or the input's directory and -I. What reading it reports (KotOR 1's
    // line-5145 typo) does not concern a disassembly and is not shown.
    let dir = args
        .input
        .parent()
        .filter(|_| args.input.as_os_str() != "-")
        .map(|d| {
            if d.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                d.to_path_buf()
            }
        });
    let mut resolver = ctx.resolver(dir);
    let mut table_map = SourceMap::new();
    let table = nwscript::engine::load_table(&mut resolver, &mut table_map)
        .ok()
        .flatten()
        .map(|(t, _)| t);
    env.inputs
        .extend(nwscript::Resolver::inputs(&resolver).iter().cloned());
    let names = |r: u16| {
        table
            .as_ref()
            .and_then(|t| t.functions.get(usize::from(r)))
            .map(|f| f.name.clone())
    };
    let opts = PrintOptions {
        raw: args.raw,
        routine_name: table
            .as_ref()
            .map(|_| &names as &dyn Fn(u16) -> Option<String>),
    };
    let name = display_name(&args.input);
    let made = if args.cfg {
        bytecode::control_flow_dot(&name, &bytes, &mut map)
    } else {
        bytecode::disassemble(&name, &bytes, &opts, &mut map)
    };
    match made {
        Err(d) => ctx.finish(env, "", vec![d], &map, Exit::Ok),
        Ok((program, text)) => {
            env.result =
                serde_json::json!({ "instructions": program.len(), "bytes": program.file_len() });
            let mut shown = String::new();
            match &args.output {
                Some(p) if p.as_os_str() != "-" => match write_output(p, text.as_bytes()) {
                    Ok(rec) => env.outputs.push(rec),
                    Err(e) => {
                        eprintln!("error: {e}");
                        return Exit::Errors;
                    }
                },
                _ if ctx.out.format == Format::Json => {
                    env.result[if args.cfg { "dot" } else { "asm" }] = text.into()
                }
                _ => shown = text,
            }
            ctx.finish(env, &shown, Vec::new(), &map, Exit::Ok)
        }
    }
}

pub fn asm(ctx: &Ctx, args: &AsmArgs) -> Exit {
    crate::working_on(display_name(&args.input));
    let out_path = match (&args.output, args.input.as_os_str() == "-") {
        (Some(p), _) => p.clone(),
        (None, false) => args.input.with_extension("ncs"),
        (None, true) => {
            eprintln!("error: reading assembly from stdin needs -o FILE (or -o - for stdout)");
            return Exit::Usage;
        }
    };
    if out_path.as_os_str() == "-" && ctx.out.format == Format::Json {
        eprintln!(
            "error: --format json writes the report to stdout, so the bytecode needs -o FILE"
        );
        return Exit::Usage;
    }
    let (text, input) = match read_input(&args.input, "assembly") {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            return Exit::Usage;
        }
    };
    let mut env = super::envelope("asm", ctx.game);
    env.inputs.push(input);
    let mut map = SourceMap::new();
    match bytecode::assemble(&display_name(&args.input), &text, &mut map) {
        Err(ds) => ctx.finish(env, "", ds, &map, Exit::Ok),
        Ok(bytes) => {
            match write_output(&out_path, &bytes) {
                Ok(rec) => env.outputs.push(rec),
                Err(e) => {
                    eprintln!("error: {e}");
                    return Exit::Errors;
                }
            }
            env.result = serde_json::json!({ "bytes": bytes.len() });
            if out_path.as_os_str() == "-" {
                // The bytecode is the output; nothing else goes to stdout.
                Exit::Ok
            } else {
                ctx.finish(env, "", Vec::new(), &map, Exit::Ok)
            }
        }
    }
}
