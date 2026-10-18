//! Argument definitions. Help text is the manual: every command carries an
//! example, and `rakata explain CODE` covers every diagnostic.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

const ABOUT: &str = "A toolset for the data of BioWare's Aurora-family games, KotOR 1 and 2 first.";

const EXAMPLES: &str = "\
Examples:
  rakata install MOD/tslpatchdata --game ~/games/swkotor
                                           install a TSLPatcher mod, every write read back
  rakata install MOD/tslpatchdata --game ~/games/swkotor --dry-run
                                           print what installing the mod would do
  rakata install MOD/tslpatchdata --game DIR --option 2 --dry-run --format json
                                           the same for the installer's second option, as JSON
  rakata explain RE0001                    explain a diagnostic code
  rakata --format json explain RE0001      the same, as one JSON document
  rakata conformance --game DIR            round-trip every file of an install
  rakata which appearance.2da --game DIR   which copy the engine loads, and every other
  rakata which dan14_juhani.utc --module danm14ac --game DIR
  rakata which --duplicates --game DIR     Override files that differ only in case
  rakata corpus extract --game DIR         every script of the install, to the corpus
  rakata corpus decompile --game DIR       every bytecode-only script, recovered to source
  rakata cat dan14_juhani.utc --module danm14ac --game DIR > dan14_juhani.utc
  rakata gff show dan14_juhani.utc --module danm14ac --game DIR --path Tag
  rakata 2da diff globalcat.2da globalcat.2da --game-a VANILLA --game-b BUILD
  rakata tlk show 42724 --game DIR         the string a strref names
  rakata lip show ./n_juhani_01.lip        a lip-sync file's keyframes
  rakata save show SAVEDIR --module danm14ac --creature carth
                                           a creature as the save has it
  rakata dialog show banter.dlg --game DIR a conversation's transcript, conditions decompiled
  rakata diff PARENT/tree CHILD/tree       what a checkpoint changed, by field, row and node
  rakata index build --game DIR --tree vanilla
                                           index a tree: facts, references and full text
  rakata index query \"SELECT * FROM refs WHERE value = 'k_pman_aioff'\"
  rakata lua build.lua OUT --game DIR     a Lua script over the façade, writing below OUT
  rakata refs 42724 --kind strref          what refers to a strref, and where each lies
  rakata lint --game DIR --known conformance/tree-known.toml
                                           dangling references, strrefs past the TLK, Override faults

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
    /// The game install: the folder holding chitin.key. Required by commands that read the game; without it, --db supplies the root of the one tree the index holds, or of the tree --tree names.
    #[arg(long, global = true, value_name = "DIR")]
    pub game: Option<PathBuf>,

    /// The index database, as `rakata index build` wrote it. Without --game, the root of its tree is the install.
    #[arg(long, global = true, value_name = "FILE")]
    pub db: Option<PathBuf>,

    /// The tree of --db whose root is the install, when --game is absent and the index holds several.
    #[arg(long, global = true, value_name = "LABEL")]
    pub tree: Option<String>,

    /// Output format; json writes exactly one document to stdout. Without it, text, and compact when an agent runs rakata (CLAUDECODE=1, or another variable DataDog pup's user agent checks): tables unpadded, `index query` as TSV.
    #[arg(long, global = true, value_enum)]
    pub format: Option<Format>,

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ProfileArg {
    K1,
    K2,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Install a TSLPatcher mod into the game, or with --dry-run print the plan.
    ///
    /// TSLPATCHDATA is the mod's tslpatchdata folder. An installer with a
    /// namespaces.ini offers options; --option picks one by its number in
    /// [Namespaces]. The lists run in TSLPatcher's order (TLKList,
    /// InstallList, 2DAList, GFFList, HACKList, CompileList, SSFList) with the
    /// 2DAMEMORY# and StrRef# tokens filled as they are set; a CompileList
    /// compiles through nwscript against the install, never through a
    /// compiler the mod ships. After every write the file or capsule member
    /// is read back: the report lists each operation, its target and the
    /// hash found there, and a write that did not land is an error (exit 1).
    /// --dry-run prints the plan, the tokens as placeholders, and changes
    /// nothing.
    #[command(
        after_help = "Examples:\n  rakata install MOD/tslpatchdata --game ~/games/swkotor\n  rakata install MOD/tslpatchdata --game ~/games/swkotor --option 1 --report install.json\n  rakata install MOD/tslpatchdata --game ~/games/swkotor --dry-run\n  rakata install MOD/tslpatchdata --game ~/games/swkotor --option 1 --dry-run --format json"
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
        /// Also write the report, as JSON, to FILE.
        #[arg(long, value_name = "FILE")]
        report: Option<PathBuf>,
    },
    /// Which copy of a resource the engine loads: the winner, every other copy the install holds, and why each lost.
    #[command(
        after_help = "Examples:\n  rakata which appearance.2da --game DIR\n  rakata which k_pdan_juhani31.ncs --module danm14ac --game DIR\n  rakata --format json which dan14_juhani.utc --module danm14ac --game DIR\n  rakata which --duplicates --game DIR\n\nThe search order is the engine's: Override/, then the module's capsules (.mod over .rim over _s.rim, then lips/STEM_loc.mod), then patch.erf (KotOR 1), then the BIFs chitin.key indexes. Names compare without regard to case."
    )]
    Which {
        /// The resource, as name.ext; case does not matter.
        #[arg(value_name = "NAME.EXT", required_unless_present = "duplicates")]
        name: Option<String>,
        /// The module whose capsules to search: the stem of modules/STEM.mod, STEM.rim and STEM_s.rim.
        #[arg(long, short, value_name = "STEM")]
        module: Option<String>,
        /// The game the install belongs to.
        #[arg(long, value_enum, default_value_t = ProfileArg::K1)]
        profile: ProfileArg,
        /// List every group of Override files whose names differ only in case, instead of resolving a name.
        #[arg(long)]
        duplicates: bool,
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
    /// The script corpus: extract every script of an install, decompile every bytecode-only script, verify against the manifest.
    Corpus(crate::cmd::CorpusArgs),
    /// Write a resource's bytes to stdout: the copy the engine loads, a file, or an archive member.
    Cat(crate::cmd::CatArgs),
    /// Show a GFF's tree or one field, set fields by path, or diff two GFFs by path.
    Gff(crate::cmd::GffArgs),
    /// Show a 2DA table, row, column or cell, or diff two tables by row and column.
    #[command(name = "2da")]
    TwoDa(crate::cmd::TwoDaArgs),
    /// Show a talk table's string by strref or text, or append one and print its strref.
    Tlk(crate::cmd::TlkArgs),
    /// Show a lip-sync file's keyframes.
    Lip(crate::cmd::LipArgs),
    /// Run a Lua script with the façade loaded, hermetically: its inputs are its arguments and --game.
    Lua(crate::cmd::LuaArgs),
    /// A saved game by name: globals, the party, each module's creatures with their flags, effects and action queues, and the pending events.
    Save(crate::cmd::SaveArgs),
    /// Conversations: show prints a dialogue's transcript, conditions decompiled.
    Dialog(crate::cmd::DialogArgs),
    /// Compare two trees or two resources by field, row, strref and dialogue node; exit 3 on a difference.
    Diff(crate::cmd::DiffArgs),
    /// The index of a tree: facts, references and full text in SQLite; build, query, digest, coverage.
    Index(crate::cmd::IndexArgs),
    /// Every reference to a strref, resref, tag or global, or every reference a resource makes, from the index.
    Refs(crate::cmd::RefsArgs),
    /// The tree lints: references that resolve to nothing, strrefs past the talk table, case-variant and .tpc-shadowed Override files.
    Lint(crate::cmd::LintArgs),
}
