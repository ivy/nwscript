//! `rakata cat`: a resource's bytes, the copy the engine loads or one named
//! by path or archive member, to stdout or a file.

use std::io::Write;
use std::path::PathBuf;

use clap::Args;
use serde_json::json;

use super::Ctx;
use super::inspect::{self, LookupArgs, resource_forms};
use crate::Exit;
use crate::cli::Format;
use rakata::diag::SourceMap;
use rakata::res::ContentHash;

#[derive(Args, Debug)]
#[command(after_help = concat!(
    "\
The bytes go to stdout, or to -o OUT. --from ARCHIVE reads RESOURCE as a
member of that archive, the same as ARCHIVE!RESOURCE. In --format json the
result names the source, the length and the SHA-256, and carries the bytes
as hexadecimal when there is no -o.

",
    resource_forms!(),
    "

Examples:
  rakata cat dan14_juhani.utc --game DIR --module danm14ac > dan14_juhani.utc
  rakata cat k_pdan_juhani31.ncs --game DIR -m danm14ac -o k_pdan_juhani31.ncs
  rakata cat dan14_juhani.utc --from ./danm14ac.mod -o dan14_juhani.utc
  rakata cat 'data/2da.bif!#74.2da' --game DIR"
))]
pub struct CatArgs {
    /// The resource: NAME.EXT, PATH or ARCHIVE!MEMBER.
    #[arg(value_name = "RESOURCE")]
    pub name: String,
    /// Read RESOURCE from this archive: an ERF, MOD, SAV, HAK, RIM or BIF.
    #[arg(long, value_name = "ARCHIVE")]
    pub from: Option<String>,
    /// Write the bytes here instead of to stdout.
    #[arg(short, long, value_name = "OUT")]
    pub output: Option<PathBuf>,
    #[command(flatten)]
    pub lookup: LookupArgs,
}

pub fn run(ctx: &Ctx, args: &CatArgs) -> Exit {
    let mut env = ctx.envelope("cat");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let name = match &args.from {
        Some(archive) => format!("{archive}!{}", args.name),
        None => args.name.clone(),
    };
    let loaded = match inspect::load(&mut env, "resource", &name, &args.lookup.lookup(ctx)) {
        Ok(l) => l,
        Err(d) => return fail(env, d),
    };
    let mut result = json!({
        "source": loaded.source,
        "len": loaded.bytes.len(),
        "sha256": ContentHash::of(&loaded.bytes).to_hex(),
    });
    match &args.output {
        Some(out) => {
            if let Err(d) = inspect::write(&mut env, out, &loaded.bytes) {
                return fail(env, d);
            }
            result["output"] = json!(out.display().to_string());
        }
        None if ctx.out.format == Format::Text => {
            let mut stdout = std::io::stdout().lock();
            if let Err(e) = stdout
                .write_all(&loaded.bytes)
                .and_then(|()| stdout.flush())
                && e.kind() != std::io::ErrorKind::BrokenPipe
            {
                panic!("writing output: {e}");
            }
        }
        None => {
            result["hex"] = json!(
                loaded
                    .bytes
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
        }
    }
    env.result = result;
    ctx.finish(env, "", Vec::new(), &map, Exit::Ok)
}
