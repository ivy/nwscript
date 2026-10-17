//! Compatibility shim: nwnnsscomp's command line mapped onto one library call.
//!
//! Mods' installers and pykotor's external-compiler seam run a program named
//! `nwnnsscomp` with the flags of the Windows builds mods ship. This binary
//! accepts those flags, compiles with nws in `--compat nwnnsscomp` mode, and
//! writes where the original wrote: the output beside the source, or `-o`.
//! It is the only place in the workspace that knows these flags.
//!
//! | flag | meaning here |
//! | --- | --- |
//! | `-c [FILE]` | compile (the source may follow `-c` or come last) |
//! | `-d [FILE]` | disassemble to `FILE.pcode` in nws's assembly syntax |
//! | `-o FILE` | the output path |
//! | `-i DIR` | an include directory; repeatable, and `;`-separated lists are split |
//! | `-g [N]` | KotOR output: implied by the profile |
//! | `-v1.00` | pre-1.30 code generation: implied; `-v1.30` is an error |
//! | `-q` | quiet |
//! | `-e -l -r -s -x -t# -p DIR` | accepted and ignored, with a warning |
//!
//! The game profile is `NWS_GAME` (`k1` or `k2`), `k1` when unset. Errors
//! print as `FILE(LINE): Error: MESSAGE` when stdout is not a terminal, the
//! shape installers read; exit 0 on success, 1 on any error.

use nwscript::diag::{Diagnostic, Levels, SourceMap, codes, render};
use nwscript::{Checker, FsResolver, Game};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Default, Debug)]
struct Args {
    mode: Option<char>,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    includes: Vec<PathBuf>,
    quiet: bool,
}

fn usage() -> ExitCode {
    eprintln!("usage: nwnnsscomp [-c|-d] [-g] [-q] [-v1.00] [-i incdir]... [-o outfile] infile");
    eprintln!(
        "This is the nwnnsscomp compatibility shim of nws; `nws compile --help` documents the compiler itself."
    );
    ExitCode::from(2)
}

fn parse(argv: &[String]) -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = argv.iter().peekable();
    let value = |it: &mut std::iter::Peekable<std::slice::Iter<String>>, flag: &str| {
        it.next()
            .cloned()
            .ok_or_else(|| format!("{flag} needs a value"))
    };
    while let Some(arg) = it.next() {
        let Some(flags) = arg.strip_prefix('-').filter(|f| !f.is_empty()) else {
            if a.input.is_none() {
                a.input = Some(arg.into());
            } else if a.output.is_none() {
                a.output = Some(arg.into());
            } else {
                return Err(format!("unexpected argument `{arg}`"));
            }
            continue;
        };
        if let Some(v) = flags.strip_prefix('v') {
            match v {
                "1.00" | "100" | "1" | "1.0" => continue,
                other => {
                    return Err(format!(
                        "-v{other}: KotOR's bytecode is the pre-1.30 form; only -v1.00 is supported"
                    ));
                }
            }
        }
        if let Some(n) = flags
            .strip_prefix('t')
            .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        {
            eprintln!("nwnnsscomp: warning: -t{n} is ignored");
            continue;
        }
        let mut chars = flags.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                'c' | 'd' => {
                    a.mode = Some(c);
                    if chars.peek().is_none()
                        && let Some(next) = it.peek()
                        && !next.starts_with('-')
                        && a.input.is_none()
                    {
                        a.input = Some(PathBuf::from(it.next().unwrap()));
                    }
                }
                'o' => {
                    let rest: String = chars.by_ref().collect();
                    a.output = Some(if rest.is_empty() {
                        value(&mut it, "-o")?.into()
                    } else {
                        rest.into()
                    });
                }
                'i' => {
                    let rest: String = chars.by_ref().collect();
                    let dirs = if rest.is_empty() {
                        value(&mut it, "-i")?
                    } else {
                        rest
                    };
                    a.includes
                        .extend(dirs.split(';').filter(|d| !d.is_empty()).map(PathBuf::from));
                }
                'p' => {
                    let rest: String = chars.by_ref().collect();
                    if rest.is_empty() {
                        value(&mut it, "-p")?;
                    }
                    eprintln!(
                        "nwnnsscomp: warning: -p is ignored; includes come from -i and the source's directory"
                    );
                }
                'g' => {
                    if chars.peek().is_none()
                        && it
                            .peek()
                            .is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()))
                    {
                        it.next();
                    }
                }
                'q' => a.quiet = true,
                'e' | 'l' | 'r' | 's' | 'x' => eprintln!("nwnnsscomp: warning: -{c} is ignored"),
                other => return Err(format!("unknown flag -{other}")),
            }
        }
    }
    Ok(a)
}

fn game() -> Game {
    match std::env::var("NWS_GAME").map(|g| g.to_ascii_lowercase()) {
        Ok(g) if g == "k2" || g == "tsl" => Game::K2,
        _ => Game::K1,
    }
}

/// `FILE(LINE): Error: MESSAGE`, the form installers parse.
fn plain(d: &Diagnostic, map: &SourceMap) -> String {
    let kind = if d.is_error() { "Error" } else { "Warning" };
    match d.primary_span() {
        Some(s) => {
            let f = map.get(s.file);
            let name = Path::new(&f.name)
                .file_name()
                .map_or(f.name.clone(), |n| n.to_string_lossy().into_owned());
            format!(
                "{name}({}): {kind}: {}",
                f.line_col(s.start).line,
                d.message
            )
        }
        None => format!("{kind}: {}", d.message),
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("nwnnsscomp: {e}");
            return usage();
        }
    };
    let Some(input) = args.input.clone() else {
        return usage();
    };
    let bytes = match std::fs::read(&input) {
        Ok(b) => b,
        Err(e) => {
            println!("Error: cannot read {}: {e}", input.display());
            return ExitCode::from(1);
        }
    };
    let dir = input
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let mut map = SourceMap::new();
    let plain_out = !std::io::stdout().is_terminal();
    let report = |ds: &[Diagnostic], map: &SourceMap| {
        for d in ds {
            if args.quiet && !d.is_error() {
                continue;
            }
            if plain_out {
                println!("{}", plain(d, map))
            } else {
                eprint!("{}", render::render(d, map, render::Color::Never))
            }
        }
    };
    let name = input.display().to_string();
    if args.mode == Some('d') {
        let out = args
            .output
            .clone()
            .unwrap_or_else(|| input.with_extension("pcode"));
        return match nwscript::bytecode::disassemble(&name, &bytes, &Default::default(), &mut map) {
            Ok((_, text)) => match std::fs::write(&out, text) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    println!("Error: cannot write {}: {e}", out.display());
                    ExitCode::from(1)
                }
            },
            Err(d) => {
                report(&[d], &map);
                ExitCode::from(1)
            }
        };
    }
    let g = game();
    let mut resolver = FsResolver::new(
        std::iter::once(dir)
            .chain(args.includes.iter().cloned())
            .collect(),
        None,
    );
    let mut checker = Checker::new();
    let out = checker.check(&mut resolver, &mut map, &name, &bytes);
    let mut levels = Levels::default();
    levels.accepted = codes::NWNNSSCOMP_ACCEPTED.iter().map(|c| c.id).collect();
    let mut diags = out.diagnostics;
    if let Some(a) = &out.analysis {
        diags.extend(nwscript::sema::lint::lint(&a.program));
    }
    let mut diags: Vec<Diagnostic> = nwscript::sema::lint::apply_directives(diags, &map)
        .into_iter()
        .filter_map(|d| levels.apply(d))
        .collect();
    let target = args
        .output
        .clone()
        .unwrap_or_else(|| input.with_extension("ncs"));
    if !nwscript::diag::has_errors(&diags)
        && let Some(a) = &out.analysis
    {
        match nwscript::compile::compile(&a.program, g) {
            Ok(p) => {
                if let Err(e) = std::fs::write(&target, nwscript::ncs::encode(&p)) {
                    println!("Error: cannot write {}: {e}", target.display());
                    return ExitCode::from(1);
                }
            }
            Err(d) => diags.push(d),
        }
    }
    report(&diags, &map);
    if nwscript::diag::has_errors(&diags) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    fn p(s: &str) -> super::Args {
        parse(&s.split_whitespace().map(String::from).collect::<Vec<_>>()).unwrap()
    }

    #[test]
    fn flag_forms() {
        let a = p("-c -g -q -v1.00 -i inc a.nss");
        assert_eq!(
            (a.mode, a.input.unwrap().to_str().unwrap(), a.quiet),
            (Some('c'), "a.nss", true)
        );
        assert_eq!(a.includes, [std::path::PathBuf::from("inc")]);
        let b = p("-c a.nss -o out/a.ncs");
        assert_eq!(
            (
                b.input.unwrap().to_str().unwrap(),
                b.output.unwrap().to_str().unwrap()
            ),
            ("a.nss", "out/a.ncs")
        );
        let c = p("-cgq -iinc1;inc2 -g 1 a.nss");
        assert_eq!(c.includes.len(), 2);
        assert!(parse(&["-v1.30".into(), "a.nss".into()]).is_err());
        assert!(parse(&["-z".into()]).is_err());
    }
}
