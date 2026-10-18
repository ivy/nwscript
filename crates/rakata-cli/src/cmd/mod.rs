//! Command dispatch and the context every command shares.

mod conformance;
pub use conformance::ConformanceArgs;
mod corpus;
pub use corpus::CorpusArgs;
mod cat;
mod decompile;
pub use decompile::DecompileArgs;
mod dialog;
pub use dialog::DialogArgs;
mod diff;
pub use diff::DiffArgs;
mod explain;
mod index;
pub use index::IndexArgs;
mod install;
mod plot;
pub use plot::{FlagsArgs, QuestArgs};
mod refs;
pub use refs::RefsArgs;
mod lint;
pub use lint::LintArgs;
mod save;
pub use save::SaveArgs;
mod show;
pub use show::ShowArgs;
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
    /// Without `--game`, the root of the index's tree when `--db` names an
    /// index that settles which tree.
    pub game: Option<PathBuf>,
    /// Why `--db` gave no install, when it was passed without `--game`.
    pub no_game_from_db: Option<String>,
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

    /// The install, or a usage error naming the flags.
    pub fn require_game(&self, command: &str) -> Result<&Path, Exit> {
        self.game.as_deref().ok_or_else(|| {
            eprintln!("{}", missing_game(command));
            if let Some(why) = &self.no_game_from_db {
                eprintln!("note: {why}");
            }
            Exit::Usage
        })
    }
}

/// The message a command needing an install prints without one.
fn missing_game(command: &str) -> String {
    format!(
        "error: `rakata {command}` needs a game install: pass --game DIR, the folder holding chitin.key, or --db FILE, an index holding one tree (--tree LABEL names one of several)"
    )
}

/// The root of the tree of the index at `db` that `tree` names, or of its
/// one tree; otherwise why there is none.
fn tree_root(db: &Path, tree: Option<&str>) -> Result<PathBuf, String> {
    let at = db.display();
    let conn = rakata::index::open(db).map_err(|e| format!("--db {at}: {}", e.message()))?;
    let rows = rakata::index::query(&conn, "SELECT label, root FROM trees ORDER BY id")
        .map_err(|e| format!("--db {at}: {}", e.message()))?;
    let trees: Vec<(&str, &str)> = rows
        .rows
        .iter()
        .filter_map(|r| Some((r.first()?.as_str()?, r.get(1)?.as_str()?)))
        .collect();
    let labels = || {
        trees
            .iter()
            .map(|(l, _)| format!("`{l}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    match (tree, trees.as_slice()) {
        (Some(t), _) => trees
            .iter()
            .find(|(l, _)| *l == t)
            .map(|(_, r)| PathBuf::from(r))
            .ok_or_else(|| format!("--db {at} holds no tree `{t}`; it holds {}", labels())),
        (None, [(_, root)]) => Ok(PathBuf::from(root)),
        (None, []) => Err(format!("--db {at} holds no tree")),
        (None, many) => Err(format!(
            "--db {at} holds {} trees ({}); name one with --tree",
            many.len(),
            labels()
        )),
    }
}

/// The `--db` and `--tree` a command was given: its own, or the global ones.
fn index_flags(cli: &Cli) -> (Option<PathBuf>, Option<String>) {
    use index::IndexAction as A;
    let (db, tree) = match &cli.command {
        Command::Refs(a) => (a.db.db.clone(), a.tree.clone()),
        Command::Show(a) => (a.db.clone(), None),
        Command::Index(a) => match &a.action {
            A::Build { db, tree, .. } | A::Digest { db, tree } | A::Coverage { db, tree, .. } => {
                (db.db.clone(), tree.clone())
            }
            A::Query { db, .. } => (db.db.clone(), None),
        },
        _ => (None, None),
    };
    (
        db.or_else(|| cli.global.db.clone()),
        tree.or_else(|| cli.global.tree.clone()),
    )
}

pub fn run(cli: Cli) -> Exit {
    let (mut game, mut no_game_from_db) = (cli.global.game.clone(), None);
    if game.is_none()
        && let (Some(db), tree) = index_flags(&cli)
    {
        match tree_root(&db, tree.as_deref()) {
            Ok(root) => game = Some(root),
            Err(why) => no_game_from_db = Some(why),
        }
    }
    let ctx = Ctx {
        game,
        no_game_from_db,
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
        Command::Decompile(args) => decompile::run(&ctx, &args),
        Command::Show(args) => show::run(&ctx, &args),
        Command::Diff(args) => diff::run(&ctx, &args),
        Command::Index(args) => index::run(&ctx, &args),
        Command::Refs(args) => refs::run(&ctx, &args),
        Command::Flags(args) => plot::flags(&ctx, &args),
        Command::Quest(args) => plot::quest(&ctx, &args),
        Command::Lint(args) => lint::run(&ctx, &args),
        Command::Which {
            name,
            module,
            profile,
            texture_pack,
            duplicates,
        } => which::run(
            &ctx,
            which::Args {
                name: name.as_deref(),
                module: module.as_deref(),
                profile,
                texture_pack: texture_pack.into(),
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
            no_game_from_db: None,
            out: Output::new(Some(Format::Text), ColorArg::Never, false),
        }
    }

    #[test]
    fn a_command_needing_the_game_without_it_is_a_usage_error_naming_the_flags() {
        assert_eq!(ctx(None).require_game("which"), Err(Exit::Usage));
        let msg = missing_game("which");
        assert!(msg.contains("--game"), "{msg}");
        assert!(msg.contains("--db"), "{msg}");
        assert!(msg.contains("`rakata which`"), "{msg}");
    }

    #[test]
    fn with_the_game_the_directory_is_given_as_named() {
        let c = ctx(Some("games/swkotor"));
        assert_eq!(c.require_game("which"), Ok(Path::new("games/swkotor")));
    }
}
