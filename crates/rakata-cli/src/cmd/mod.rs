//! Command dispatch and the context every command shares.

mod explain;

use crate::Exit;
use crate::cli::{Cli, Command};
use crate::output::Output;
use rakata::Envelope;
use rakata::diag::{Diagnostic, SourceMap, has_errors};
use std::path::{Path, PathBuf};

/// What every command receives: the install, and where output goes.
pub struct Ctx {
    /// The install `--game` named, unchecked: a command that reads it opens
    /// it and reports what it finds there.
    pub game: Option<PathBuf>,
    pub out: Output,
}

impl Ctx {
    /// The envelope of a command, naming the install `--game` named.
    pub fn envelope(&self, command: &str) -> Envelope {
        rakata::envelope(command, self.game.clone())
    }

    /// Emit the outcome and give the exit code: 1 when an error remains,
    /// else `ok`.
    pub fn finish(
        &self,
        env: Envelope,
        text: &str,
        diags: Vec<Diagnostic>,
        map: &SourceMap,
        ok: Exit,
    ) -> Exit {
        self.out.emit(env, text, &diags, map);
        if has_errors(&diags) { Exit::Errors } else { ok }
    }

    /// The install, or a usage error naming the flag.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "no command reads the install yet")
    )]
    pub fn require_game(&self, command: &str) -> Result<&Path, Exit> {
        self.game.as_deref().ok_or_else(|| {
            eprintln!("{}", missing_game(command));
            Exit::Usage
        })
    }
}

/// The message a command needing an install prints without one.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "no command reads the install yet")
)]
fn missing_game(command: &str) -> String {
    format!(
        "error: `rakata {command}` needs a game install: pass --game DIR, the folder holding chitin.key"
    )
}

pub fn run(cli: Cli) -> Exit {
    let ctx = Ctx {
        game: cli.global.game,
        out: Output::new(cli.global.format, cli.global.color, cli.global.quiet),
    };
    match cli.command {
        Command::Explain { code } => explain::run(&ctx, &code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{ColorArg, Format};

    fn ctx(game: Option<&str>) -> Ctx {
        Ctx {
            game: game.map(PathBuf::from),
            out: Output::new(Format::Text, ColorArg::Never, false),
        }
    }

    #[test]
    fn a_command_needing_the_game_without_it_is_a_usage_error_naming_the_flag() {
        assert_eq!(ctx(None).require_game("which"), Err(Exit::Usage));
        let msg = missing_game("which");
        assert!(msg.contains("--game"), "{msg}");
        assert!(msg.contains("`rakata which`"), "{msg}");
    }

    #[test]
    fn with_the_game_the_directory_is_given_as_named() {
        let c = ctx(Some("games/swkotor"));
        assert_eq!(c.require_game("which"), Ok(Path::new("games/swkotor")));
    }
}
