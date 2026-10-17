//! Command dispatch and the context every command shares.

pub mod conformance;
pub mod disasm;
mod explain;

use crate::Exit;
use crate::cli::{Cli, Command, GameArg};
use crate::output::Output;
use nwscript::diag::{Diagnostic, Level, Levels, SourceMap, codes, has_errors};
use nwscript::{Envelope, FsResolver, Game};

/// What every command receives: the profile, lint levels, and where output goes.
pub struct Ctx {
    pub game: Option<Game>,
    pub levels: Levels,
    pub out: Output,
    pub global: crate::cli::Global,
}

impl Ctx {
    /// Apply the lint levels to a command's diagnostics, emit the outcome,
    /// and give the exit code: 1 when an error remains, else `ok`.
    pub fn finish(
        &self,
        env: Envelope,
        text: &str,
        diags: Vec<Diagnostic>,
        map: &SourceMap,
        ok: Exit,
    ) -> Exit {
        let diags: Vec<Diagnostic> = diags
            .into_iter()
            .filter_map(|d| self.levels.apply(d))
            .collect();
        self.out.emit(env, text, &diags, map);
        if has_errors(&diags) { Exit::Errors } else { ok }
    }

    /// A resolver over the `-I` directories (after `first`, the input's own
    /// directory) and `--nwscript`.
    pub fn resolver(&self, first: Option<std::path::PathBuf>) -> FsResolver {
        let dirs = first
            .into_iter()
            .chain(self.global.include.iter().cloned())
            .collect();
        FsResolver::new(dirs, self.global.nwscript.clone())
    }

    /// The game profile, or a usage error naming the flag.
    #[expect(dead_code, reason = "the compiling commands use it")]
    pub fn require_game(&self, command: &str) -> Result<Game, Exit> {
        self.game.ok_or_else(|| {
            eprintln!("error: `nws {command}` needs a game profile: pass --game k1 or --game k2");
            Exit::Usage
        })
    }
}

fn levels(g: &crate::cli::Global) -> Result<Levels, Exit> {
    let mut l = Levels::default();
    let mut set = |id: &str, level| match codes::lookup(id) {
        Some(c) => {
            l.set(c.id, level);
            Ok(())
        }
        None => {
            eprintln!(
                "error: unknown diagnostic code `{id}`; `nws explain CODE` lists what a code means"
            );
            Err(Exit::Usage)
        }
    };
    for a in &g.allow {
        set(a, Level::Allow)?;
    }
    for d in &g.deny {
        if d.eq_ignore_ascii_case("warnings") {
            continue;
        }
        set(d, Level::Deny)?;
    }
    l.deny_warnings = g.deny.iter().any(|d| d.eq_ignore_ascii_case("warnings"));
    Ok(l)
}

pub fn run(cli: Cli) -> Exit {
    let levels = match levels(&cli.global) {
        Ok(l) => l,
        Err(e) => return e,
    };
    let ctx = Ctx {
        game: cli.global.game.map(|g| match g {
            GameArg::K1 => Game::K1,
            GameArg::K2 => Game::K2,
        }),
        levels,
        out: Output::new(cli.global.format, cli.global.color, cli.global.quiet),
        global: cli.global,
    };
    match cli.command {
        Command::Explain { code } => explain::run(&ctx, &code),
        Command::Conformance(args) => conformance::run(&ctx, &args),
        Command::Disasm(args) => disasm::disasm(&ctx, &args),
        Command::Asm(args) => disasm::asm(&ctx, &args),
        Command::InternalPanic => {
            crate::working_on("the internal-panic command");
            panic!("deliberate panic requested by `nws internal-panic`")
        }
        other => {
            let name = format!("{other:?}")
                .split(['(', ' '])
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            eprintln!("error: `nws {name}` is not yet implemented");
            Exit::Usage
        }
    }
}
