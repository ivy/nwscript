//! Command dispatch and the context every command shares.

mod conformance;
pub use conformance::ConformanceArgs;
mod corpus;
pub use corpus::CorpusArgs;
mod cat;
mod dialog;
pub use dialog::DialogArgs;
mod diff;
pub use diff::DiffArgs;
mod explain;
mod index;
pub use index::IndexArgs;
mod install;
mod save;
pub use save::SaveArgs;
mod which;
pub use cat::CatArgs;
mod gff;
pub use gff::GffArgs;
mod inspect;
mod lip;
pub use lip::LipArgs;
mod lua;
pub use lua::LuaArgs;
mod tlk;
pub use tlk::TlkArgs;
mod twoda;
pub use twoda::TwoDaArgs;

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
    pub fn require_game(&self, command: &str) -> Result<&Path, Exit> {
        self.game.as_deref().ok_or_else(|| {
            eprintln!("{}", missing_game(command));
            Exit::Usage
        })
    }
}

/// The message a command needing an install prints without one.
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
        Command::Install {
            tslpatchdata,
            option,
            dry_run,
            report,
        } => install::run(&ctx, &tslpatchdata, option, dry_run, report.as_deref()),
        Command::Conformance(args) => conformance::run(&ctx, &args),
        Command::Corpus(args) => corpus::run(&ctx, &args),
        Command::Cat(args) => cat::run(&ctx, &args),
        Command::Gff(args) => gff::run(&ctx, &args),
        Command::TwoDa(args) => twoda::run(&ctx, &args),
        Command::Tlk(args) => tlk::run(&ctx, &args),
        Command::Lip(args) => lip::run(&ctx, &args),
        Command::Lua(args) => lua::run(&ctx, &args),
        Command::Save(args) => save::run(&ctx, &args),
        Command::Dialog(args) => dialog::run(&ctx, &args),
        Command::Diff(args) => diff::run(&ctx, &args),
        Command::Index(args) => index::run(&ctx, &args),
        Command::Which {
            name,
            module,
            profile,
            duplicates,
        } => which::run(
            &ctx,
            which::Args {
                name: name.as_deref(),
                module: module.as_deref(),
                profile,
                duplicates,
            },
        ),
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
