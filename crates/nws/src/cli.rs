//! Argument definitions. Help text is the manual: every command carries an
//! example, and `nws explain CODE` covers every diagnostic.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

const ABOUT: &str =
    "A toolchain for NWScript, the scripting language of BioWare's Aurora-family games.";

const EXAMPLES: &str = "\
Examples:
  nws explain E0001                    explain a diagnostic code
  nws --format json explain E0001      the same, as one JSON document
  nws --game k1 -I scripts compile k_my_script.nss

Exit codes: 0 success, 1 errors reported, 2 bad invocation,
3 a comparison found a difference, 101 internal error.";

#[derive(Parser, Debug)]
#[command(name = "nws", version, about = ABOUT, after_help = EXAMPLES, propagate_version = true)]
pub struct Cli {
    #[command(flatten)]
    pub global: Global,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug)]
pub struct Global {
    /// The game profile: which nwscript.nss forms, codegen forms and engine semantics. Required by commands that compile or run; it has no default.
    #[arg(long, global = true, value_enum)]
    pub game: Option<GameArg>,

    /// An include directory, searched in order after the input's own directory; nwscript.nss is found the same way. Repeatable.
    #[arg(short = 'I', long = "include", global = true, value_name = "DIR")]
    pub include: Vec<PathBuf>,

    /// Use this nwscript.nss instead of searching the include directories.
    #[arg(long, global = true, value_name = "FILE")]
    pub nwscript: Option<PathBuf>,

    /// Output format. json writes exactly one document to stdout.
    #[arg(long, global = true, value_enum, default_value_t = Format::Text)]
    pub format: Format,

    /// Do not report this lint. Repeatable.
    #[arg(long, global = true, value_name = "CODE")]
    pub allow: Vec<String>,

    /// Report this lint as an error; `--deny warnings` promotes every warning. Repeatable.
    #[arg(long, global = true, value_name = "CODE")]
    pub deny: Vec<String>,

    /// Accept what another compiler accepted: `nwnnsscomp` (the Windows builds mods ship) or `bioware` (the vanilla scripts).
    #[arg(long, global = true, value_enum)]
    pub compat: Option<Compat>,

    /// Suppress warnings in text mode.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Colour in text output. NO_COLOR is honoured.
    #[arg(long, global = true, value_enum, default_value_t = ColorArg::Auto)]
    pub color: ColorArg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum GameArg {
    K1,
    K2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Compat {
    Nwnnsscomp,
    Bioware,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ColorArg {
    Auto,
    Always,
    Never,
}

/// Arguments of a command whose implementation has not landed yet.
#[derive(Args, Debug)]
pub struct Reserved {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    pub args: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print the full explanation of a diagnostic code.
    #[command(after_help = "Example:\n  nws explain E0001\n  nws --format json explain e0001")]
    Explain {
        /// The code, such as E0001; case does not matter.
        code: String,
    },

    /// Compile NWScript source to NCS bytecode.
    Compile(crate::cmd::compile::CompileArgs),
    /// Check source without writing output: syntax, names, types, calls and includes.
    Check(crate::cmd::check::CheckArgs),
    /// Disassemble NCS bytecode into assembly text.
    Disasm(crate::cmd::disasm::DisasmArgs),
    /// Assemble assembly text into NCS bytecode.
    Asm(crate::cmd::disasm::AsmArgs),
    /// Recover source from bytecode, verified by recompiling. (not yet implemented)
    #[command(after_help = "Example:\n  nws --game k1 decompile k_my_script.ncs")]
    Decompile(Reserved),
    /// Run a script under the recording engine or the world model.
    Run(crate::cmd::run::Args),
    /// Run Lua tests.
    Test(crate::cmd::test::TestArgs),
    /// Compare two scripts by bytes, instructions and trace.
    Diff(crate::cmd::diff::Args),
    /// Look up an engine routine in nwscript.nss.
    Fn(crate::cmd::fnlookup::Args),
    /// Measure compilers against the vanilla corpus.
    Conformance(Box<crate::cmd::conformance::Args>),
    /// Run a Lua script against the library.
    Lua(crate::cmd::test::LuaArgs),
    /// Format NWScript source. (not yet implemented)
    #[command(after_help = "Example:\n  nws fmt k_my_script.nss")]
    Fmt(Reserved),
    /// Serve the language server protocol on stdio. (not yet implemented)
    #[command(after_help = "Example:\n  nws lsp")]
    Lsp(Reserved),

    /// Panic on purpose, to test the internal-error path.
    #[command(hide = true)]
    InternalPanic,
}
