//! Command dispatch and the context every command shares.

pub mod check;
pub mod compile;
pub mod conformance;
pub mod decompile;
pub mod diff;
pub mod disasm;
mod explain;
pub mod fnlookup;
pub mod run;
pub mod test;

use crate::Exit;
use crate::cli::{Cli, Command, GameArg};
use crate::output::Output;
use crate::sources::{GameDir, Sources};
use nwscript::diag::{Diagnostic, Level, Levels, SourceMap, has_errors};
use nwscript::modes::Mode;
use nwscript::{Envelope, FsResolver, Game};

/// The envelope of an `nws` command.
pub fn envelope(command: &str, game: Option<Game>) -> Envelope {
    Envelope::new("nws", env!("CARGO_PKG_VERSION"), command, game)
}

/// What every command receives: the profile, lint levels, and where output goes.
pub struct Ctx {
    pub game: Option<Game>,
    pub levels: Levels,
    pub out: Output,
    pub global: crate::cli::Global,
    /// The `--overlay` files, read: each one's path and bytes.
    pub overlays: Vec<(String, std::sync::Arc<[u8]>)>,
    /// The install `--game-dir` names, opened once.
    pub game_dir: Option<GameDir>,
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

    /// A file's diagnostics with its lints added, comment directives
    /// applied, and the lint levels applied: what decides its status.
    pub fn linted(
        &self,
        analysis: Option<&nwscript::sema::Analysis>,
        diags: Vec<Diagnostic>,
        map: &SourceMap,
    ) -> Vec<Diagnostic> {
        nwscript::modes::finish(analysis, diags, map, &self.levels)
    }

    /// The declarations `--declarations` names.
    pub fn declarations(&self) -> nwscript::Declarations {
        match self.global.declarations {
            crate::cli::DeclarationsArg::Shipped => nwscript::Declarations::Shipped,
            crate::cli::DeclarationsArg::Bioware => nwscript::Declarations::BioWare,
        }
    }

    /// A checker taking the declarations `--declarations` names, then the
    /// `--overlay` files.
    pub fn checker(&self, game: Game) -> nwscript::Checker {
        self.overlays.iter().fold(
            nwscript::Checker::with_declarations(self.declarations(), game),
            |c, (name, bytes)| c.with_overlay(name.clone(), bytes.clone()),
        )
    }

    /// A resolver over the `-I` directories (after `first`, the input's own
    /// directory) and `--nwscript`, then the install `--game-dir` names.
    pub fn resolver(&self, first: Option<std::path::PathBuf>) -> Sources {
        let dirs = first
            .into_iter()
            .chain(self.global.include.iter().cloned())
            .collect();
        Sources::new(
            FsResolver::new(dirs, self.global.nwscript.clone()),
            self.game_dir.clone(),
        )
    }

    /// The game profile, or a usage error naming the flag.
    pub fn require_game(&self, command: &str) -> Result<Game, Exit> {
        self.game.ok_or_else(|| {
            eprintln!("error: `nws {command}` needs a game profile: pass --game k1 or --game k2");
            Exit::Usage
        })
    }
}

fn levels(g: &crate::cli::Global) -> Result<Levels, Exit> {
    let mut l = Levels::default();
    let mut set = |id: &str, level| match crate::codes::lookup(id) {
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
    // Strict is the default; a compat mode accepts what that compiler did.
    let base = match g.compat {
        None => Mode::Strict,
        Some(crate::cli::Compat::Nwnnsscomp) => Mode::Nwnnsscomp,
        Some(crate::cli::Compat::Bioware) => Mode::Bioware,
    }
    .levels();
    l.strict = base.strict;
    l.accepted = base.accepted;
    Ok(l)
}

pub fn run(cli: Cli) -> Exit {
    let levels = match levels(&cli.global) {
        Ok(l) => l,
        Err(e) => return e,
    };
    let mut overlays = Vec::new();
    for p in &cli.global.overlay {
        match std::fs::read(p) {
            Ok(b) => overlays.push((p.display().to_string(), b.into())),
            Err(e) => {
                eprintln!("error: cannot read the overlay {}: {e}", p.display());
                return Exit::Usage;
            }
        }
    }
    let game = cli.global.game.map(|g| match g {
        GameArg::K1 => Game::K1,
        GameArg::K2 => Game::K2,
    });
    let game_dir = match &cli.global.game_dir {
        None => None,
        Some(dir) => {
            let Some(game) = game else {
                eprintln!("error: --game-dir needs the game profile: pass --game k1 or --game k2");
                return Exit::Usage;
            };
            match crate::sources::open(dir, game, cli.global.module.as_deref()) {
                Ok(g) => Some(g),
                Err(e) => {
                    eprintln!("error: {e}");
                    return Exit::Usage;
                }
            }
        }
    };
    let ctx = Ctx {
        game,
        levels,
        out: Output::new(cli.global.format, cli.global.color, cli.global.quiet),
        global: cli.global,
        overlays,
        game_dir,
    };
    match cli.command {
        Command::Explain { code } => explain::run(&ctx, &code),
        Command::Conformance(args) => conformance::run(&ctx, &args),
        Command::Check(args) => check::run(&ctx, &args),
        Command::Compile(args) => compile::run(&ctx, &args),
        Command::Disasm(args) => disasm::disasm(&ctx, &args),
        Command::Decompile(args) => decompile::run(&ctx, &args),
        Command::Asm(args) => disasm::asm(&ctx, &args),
        Command::Run(args) => run::run(&ctx, &args),
        Command::Fn(args) => fnlookup::run(&ctx, &args),
        Command::Diff(args) => diff::run(&ctx, &args),
        Command::Test(args) => test::test(&ctx, &args),
        Command::Lua(args) => test::lua(&ctx, &args),
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
