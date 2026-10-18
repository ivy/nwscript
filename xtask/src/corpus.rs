//! `cargo xtask corpus`: the corpus commands of the `rakata` façade under
//! their development name, so the recipes in the documents keep working.
//! `rakata corpus extract|decompile|verify` is the same code.

use rakata::corpus::ExtractOptions;
pub use rakata::corpus::default_out;
use std::error::Error;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const USAGE: &str = "\
cargo xtask corpus: the script corpus, extracted from your own game
(the same as `rakata corpus`; `rakata corpus decompile` recovers the module scripts)

USAGE:
  cargo xtask corpus extract --game-dir DIR [--out DIR] [--manifest FILE --source TEXT]
  cargo xtask corpus verify [--out DIR] [--manifest FILE]

--out defaults to $NWSCRIPT_CORPUS, then ~/.cache/nwscript/corpus/k1.
--manifest defaults to conformance/k1.toml; module hashes go beside it in
k1-modules.toml. extract writes the manifest only when --manifest is given.

EXAMPLE:
  cargo xtask corpus extract --game-dir ~/games/swkotor
  NWSCRIPT_CORPUS=~/.cache/nwscript/corpus/k1 cargo xtask corpus verify
";

struct Opts {
    game_dir: Option<PathBuf>,
    out: PathBuf,
    manifest: Option<PathBuf>,
    source: String,
}

fn parse(args: &[String]) -> Result<Opts> {
    let mut o = Opts {
        game_dir: None,
        out: default_out(),
        manifest: None,
        source: "unknown".into(),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = || {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{a} needs a value"))
        };
        match a.as_str() {
            "--game-dir" | "--game" => o.game_dir = Some(val()?.into()),
            "--out" => o.out = val()?.into(),
            "--manifest" => o.manifest = Some(val()?.into()),
            "--source" => o.source = val()?,
            other => return Err(format!("unknown argument {other}\n\n{USAGE}").into()),
        }
    }
    Ok(o)
}

pub fn main(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("extract") => {
            let o = parse(&args[1..])?;
            let game = o.game_dir.ok_or("extract needs --game-dir")?;
            let x = rakata::corpus::extract(&ExtractOptions {
                game,
                out: o.out,
                manifest: o.manifest,
                source: o.source,
            })?;
            println!("corpus written to {}", x.out.display());
            for (bif, (nss, ncs)) in &x.per_bif {
                println!("  {bif}: {nss} nss, {ncs} ncs");
            }
            println!(
                "  total: {} nss, {} ncs, {} pairs, {} module ncs in {} archives",
                x.counts.nss, x.counts.ncs, x.counts.pairs, x.counts.module_ncs, x.archives
            );
            if let Some(m) = &x.manifest {
                println!("manifest written to {}", m.display());
            }
            Ok(())
        }
        Some("verify") => {
            let o = parse(&args[1..])?;
            let manifest = o.manifest.unwrap_or_else(|| "conformance/k1.toml".into());
            let v = rakata::corpus::verify(&o.out, &manifest)?;
            if v.is_ok() {
                println!("{} files match {}", v.files, manifest.display());
                Ok(())
            } else {
                for p in v.problems.iter().take(20) {
                    eprintln!("  {p}");
                }
                Err(format!("{} of {} files do not match", v.problems.len(), v.files).into())
            }
        }
        _ => {
            print!("{USAGE}");
            Ok(())
        }
    }
}
