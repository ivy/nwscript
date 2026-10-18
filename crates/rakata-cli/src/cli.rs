//! Argument definitions. Help text is the manual: every command carries an
//! example, and `rakata explain CODE` covers every diagnostic.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

const ABOUT: &str = "A toolset for the data of BioWare's Aurora-family games, KotOR 1 and 2 first.";

const EXAMPLES: &str = "\
Examples:
  rakata explain RE0001                    explain a diagnostic code
  rakata --format json explain RE0001      the same, as one JSON document

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
    /// Print the full explanation of a diagnostic code.
    #[command(
        after_help = "Example:\n  rakata explain RE0001\n  rakata --format json explain re0001"
    )]
    Explain {
        /// The code, such as RE0001; case does not matter.
        code: String,
    },
}
