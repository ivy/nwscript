//! Argument definitions. Help text is the manual: every command carries an
//! example, and `rakata explain CODE` covers every diagnostic.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

const ABOUT: &str = "A toolset for the data of BioWare's Aurora-family games, KotOR 1 and 2 first.";

const EXAMPLES: &str = "\
Examples:
  rakata install MOD/tslpatchdata --game ~/games/swkotor --dry-run
                                           print what installing the mod would do
  rakata install MOD/tslpatchdata --game DIR --option 2 --dry-run --format json
                                           the same for the installer's second option, as JSON
  rakata explain RE0001                    explain a diagnostic code
  rakata --format json explain RE0001      the same, as one JSON document
  rakata conformance --game DIR            round-trip every file of an install

Exit codes: 0 success, 1 errors reported, 2 bad invocation,
3 a comparison found a difference, 101 internal error.";

#[derive(Parser, Debug)]
#[command(name = "rakata", version = rakata::VERSION, about = ABOUT, after_help = EXAMPLES, propagate_version = true)]
pub struct Cli {
    #[command(flatten)]
    pub global: Global,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug)]
pub struct Global {
    /// The game install: the folder holding chitin.key. Required by commands that read the game; it has no default.
    #[arg(long, global = true, value_name = "DIR")]
    pub game: Option<PathBuf>,

    /// Output format. json writes exactly one document to stdout.
    #[arg(long, global = true, value_enum, default_value_t = Format::Text)]
    pub format: Format,

    /// Suppress warnings in text mode.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Colour in text output. NO_COLOR is honoured.
    #[arg(long, global = true, value_enum, default_value_t = ColorArg::Auto)]
    pub color: ColorArg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ColorArg {
    Auto,
    Always,
    Never,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Install a TSLPatcher mod into the game, or with --dry-run print the plan.
    ///
    /// TSLPATCHDATA is the mod's tslpatchdata folder. An installer with a
    /// namespaces.ini offers options; --option picks one by its number in
    /// [Namespaces]. Today only --dry-run is implemented: the plan is the
    /// operations in the order TSLPatcher runs them, with the 2DAMEMORY# and
    /// StrRef# tokens as placeholders.
    #[command(
        after_help = "Examples:\n  rakata install MOD/tslpatchdata --game ~/games/swkotor --dry-run\n  rakata install MOD/tslpatchdata --game ~/games/swkotor --option 1 --dry-run --format json"
    )]
    Install {
        /// The mod's tslpatchdata folder.
        tslpatchdata: PathBuf,
        /// Which option of a namespaces.ini to install, by its number in [Namespaces].
        #[arg(long, value_name = "N")]
        option: Option<u32>,
        /// Print the plan and change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Print the full explanation of a diagnostic code.
    #[command(
        after_help = "Example:\n  rakata explain RE0001\n  rakata --format json explain re0001"
    )]
    Explain {
        /// The code, such as RE0001; case does not matter.
        code: String,
    },
    /// Read, write and compare every file of an install, per format.
    Conformance(crate::cmd::ConformanceArgs),
}
