//! Why an authoring operation refused. Every variant has a registered
//! code; the formats' own errors pass through with theirs.

use std::fmt;
use std::path::PathBuf;

use nwscript::diag::Diagnostic;
use rakata_dialog::Finding;
use rakata_res::{ContentHash, ResId};

/// Diagnostic codes of the authoring API: the `RE14xx` range of the
/// `rakata` registry.
pub mod codes {
    /// A module copy of a replaced resource that differs from the hash given.
    pub const REPLACED_DIFFERS: &str = "RE1401";
    /// `Override/` already holds a copy of a resource the mod replaces.
    pub const OVERRIDE_HOLDS: &str = "RE1402";
    /// The module holds no copy of a resource the mod replaces.
    pub const NOT_IN_MODULE: &str = "RE1403";
    /// A global `globalcat.2da` already declares.
    pub const GLOBAL_DECLARED: &str = "RE1404";
    /// A 2DA row label that matches more than one row, or a new row's label
    /// the table already carries.
    pub const LABEL_TAKEN: &str = "RE1405";
    /// A blueprint field set to a value of another type.
    pub const FIELD_TYPE: &str = "RE1406";
    /// A dialogue write refused: a node or link it adds or changes refers
    /// to something the install lacks.
    pub const DIALOG_REFUSED: &str = "RE1407";
    /// A script that does not compile.
    pub const DOES_NOT_COMPILE: &str = "RE1408";
    /// A file the mod writes could not be written.
    pub const WRITE_FAILED: &str = "RE1409";
    /// A voice line whose span its take's transcript does not hold.
    pub const BAD_SPAN: &str = "RE1410";
    /// Two voice lines with one resref.
    pub const DUPLICATE_LINE: &str = "RE1411";
    /// A transcript or a set of mouth cues that does not read.
    pub const BAD_CUES: &str = "RE1412";
    /// A voice-over for `streamwaves/` that is not a RIFF WAVE file.
    pub const NOT_A_WAVE: &str = "RE1413";
    /// Bytes written as NCS that do not decode.
    pub const NOT_NCS: &str = "RE1414";
    /// Another module's own copy of a resource that a write to `Override/`
    /// would replace there, and that differs from the copy the mod edits.
    pub const OTHER_MODULE_DIFFERS: &str = "RE1415";
    /// A dialogue node whose script is not the one an edit replaces.
    pub const SCRIPT_DIFFERS: &str = "RE1416";
    /// A global `globalcat.2da` does not declare.
    pub const GLOBAL_UNDECLARED: &str = "RE1417";
    /// A copy in `Override/` that a write into a module's capsule would
    /// stay beneath.
    pub const OVERRIDE_SHADOWS: &str = "RE1418";
    /// A write into a module's capsule from a mod that names no module.
    pub const NO_MODULE: &str = "RE1419";
}

/// An authoring operation's refusal.
#[derive(Debug)]
pub enum Error {
    /// A name that is not `NAME.ext` for a known type; the code is
    /// `rakata-res`'s.
    Res(rakata_res::Error),
    /// The install could not answer; the code is `rakata-game`'s
    /// (`RE0404` when no layer holds what was asked for).
    Game(rakata_game::Error),
    /// The dialogue did not read, or a node key did not resolve; the code
    /// is `rakata-dialog`'s, unchanged.
    Dialog(rakata_dialog::Error),
    /// A GFF did not read, or a path named nothing; `rakata-gff`'s code.
    Gff(rakata_gff::Error),
    /// A table did not read, or a row or column is missing; `rakata-2da`'s.
    TwoDa(rakata_2da::Error),
    /// A lip-sync file did not read; `rakata-lip`'s code.
    Lip(rakata_lip::Error),
    /// The talk table did not read; `rakata-tlk`'s code.
    Tlk(rakata_tlk::Error),
    /// Bytes written as NCS that do not decode.
    Ncs { name: String, message: String },
    /// A copy of the resource in one of the module's capsules is not the
    /// one the mod was written against.
    ReplacedDiffers {
        id: ResId,
        /// The capsule's file name.
        capsule: String,
        expected: ContentHash,
        found: ContentHash,
    },
    /// `Override/` already holds the resource, from another mod.
    OverrideHolds { id: ResId, path: PathBuf },
    /// The module holds no copy of the resource.
    NotInModule { id: ResId, module: Option<String> },
    /// `globalcat.2da` already declares the global.
    GlobalDeclared { name: String, row: usize },
    /// A label naming several rows, or one the table already carries.
    LabelTaken {
        label: String,
        rows: Vec<usize>,
        new: bool,
    },
    /// A field set to a value of another type than it holds.
    FieldType {
        path: String,
        holds: &'static str,
        given: &'static str,
    },
    /// A dialogue write refused: the findings its edit introduced.
    DialogRefused {
        dialog: String,
        findings: Vec<Finding>,
    },
    /// A script that does not compile, with every error and warning.
    DoesNotCompile {
        script: String,
        /// The diagnostics as `nws` prints them.
        rendered: String,
        diagnostics: Vec<Diagnostic>,
    },
    /// A file could not be written.
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A voice line whose span the transcript does not hold.
    BadSpan { line: String, reason: String },
    /// Two lines with one resref.
    DuplicateLine { resref: String },
    /// A transcript or Rhubarb's cues that do not read.
    BadCues { what: String, reason: String },
    /// A voice-over that is not RIFF WAVE.
    NotAWave { name: String },
    /// Another module holds its own copy of the resource, which differs
    /// from the copy the mod edits; `Override/` would replace it there too.
    OtherModuleDiffers {
        id: ResId,
        /// The module the mod works in, if it names one.
        here: Option<String>,
        module: String,
        /// The capsule's file name.
        capsule: String,
    },
    /// The node does not run the script the edit replaces.
    ScriptDiffers {
        dialog: String,
        key: String,
        expected: Option<String>,
        found: Option<String>,
    },
    /// `globalcat.2da` does not declare the global.
    GlobalUndeclared { name: String },
    /// `Override/` holds the resource, so the module would go on loading
    /// that copy over the one written into its capsule.
    OverrideShadows {
        id: ResId,
        module: String,
        path: PathBuf,
    },
    /// A write into a module's capsule, and the mod works in no module.
    NoModule { id: ResId },
}

/// The authoring API's result type.
pub type Result<T> = std::result::Result<T, Error>;

/// A format error's text without its leading `CODE: `.
fn bare(code: &str, text: String) -> String {
    text.strip_prefix(code)
        .and_then(|t| t.strip_prefix(": "))
        .map(str::to_owned)
        .unwrap_or(text)
}

impl Error {
    /// The registered diagnostic code.
    pub fn code(&self) -> &'static str {
        match self {
            Error::Res(e) => e.code(),
            Error::Game(e) => e.code(),
            Error::Dialog(e) => e.code(),
            Error::Gff(e) => e.code(),
            Error::TwoDa(e) => e.code(),
            Error::Lip(e) => e.code(),
            Error::Tlk(e) => e.code(),
            Error::Ncs { .. } => codes::NOT_NCS,
            Error::ReplacedDiffers { .. } => codes::REPLACED_DIFFERS,
            Error::OverrideHolds { .. } => codes::OVERRIDE_HOLDS,
            Error::NotInModule { .. } => codes::NOT_IN_MODULE,
            Error::GlobalDeclared { .. } => codes::GLOBAL_DECLARED,
            Error::LabelTaken { .. } => codes::LABEL_TAKEN,
            Error::FieldType { .. } => codes::FIELD_TYPE,
            Error::DialogRefused { .. } => codes::DIALOG_REFUSED,
            Error::DoesNotCompile { .. } => codes::DOES_NOT_COMPILE,
            Error::Write { .. } => codes::WRITE_FAILED,
            Error::BadSpan { .. } => codes::BAD_SPAN,
            Error::DuplicateLine { .. } => codes::DUPLICATE_LINE,
            Error::BadCues { .. } => codes::BAD_CUES,
            Error::NotAWave { .. } => codes::NOT_A_WAVE,
            Error::OtherModuleDiffers { .. } => codes::OTHER_MODULE_DIFFERS,
            Error::ScriptDiffers { .. } => codes::SCRIPT_DIFFERS,
            Error::GlobalUndeclared { .. } => codes::GLOBAL_UNDECLARED,
            Error::OverrideShadows { .. } => codes::OVERRIDE_SHADOWS,
            Error::NoModule { .. } => codes::NO_MODULE,
        }
    }

    /// The message without the code.
    pub fn message(&self) -> String {
        match self {
            Error::Res(e) => bare(e.code(), e.to_string()),
            Error::Game(e) => e.message(),
            Error::Dialog(e) => e.message(),
            Error::Gff(e) => bare(e.code(), e.to_string()),
            Error::TwoDa(e) => bare(e.code(), e.to_string()),
            Error::Lip(e) => bare(e.code(), e.to_string()),
            Error::Tlk(e) => bare(e.code(), e.to_string()),
            Error::Ncs { name, message } => format!("{name} is not NCS bytecode: {message}"),
            Error::ReplacedDiffers {
                id,
                capsule,
                expected,
                found,
            } => format!(
                "{capsule}'s {id} is not the copy the mod replaces (SHA-256 {found}, expected {expected}); replacing it would drop what that copy does"
            ),
            Error::OverrideHolds { id, path } => format!(
                "Override already holds {id} ({}), from another mod; replacing it would drop that mod's change",
                path.display()
            ),
            Error::NotInModule { id, module } => match module {
                Some(m) => format!("the module {m} holds no {id} to replace"),
                None => format!("no module was named, so there is no copy of {id} to guard"),
            },
            Error::GlobalDeclared { name, row } => {
                format!("globalcat.2da already declares {name}, at row {row}")
            }
            Error::LabelTaken { label, rows, new } => {
                let rows = rows
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                if *new {
                    format!("the table already has a row labelled `{label}` (row {rows})")
                } else {
                    format!("the label `{label}` names rows {rows}; name one by index")
                }
            }
            Error::FieldType { path, holds, given } => {
                format!("{path} holds a {holds}, and the value given is a {given}")
            }
            Error::DialogRefused { dialog, findings } => {
                let each = findings
                    .iter()
                    .map(|f| format!("{} [{}]", f.message(dialog), f.code))
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("{dialog} not written: {each}")
            }
            Error::DoesNotCompile {
                script, rendered, ..
            } => format!("{script} does not compile:\n{rendered}"),
            Error::Write { path, source } => {
                format!("cannot write {}: {source}", path.display())
            }
            Error::BadSpan { line, reason } => format!("the voice line {line}: {reason}"),
            Error::DuplicateLine { resref } => {
                format!("two voice lines are named {resref}; each line is its own file")
            }
            Error::BadCues { what, reason } => format!("{what} does not read: {reason}"),
            Error::NotAWave { name } => format!(
                "{name} does not begin with a RIFF WAVE header, which the game's voice files carry"
            ),
            Error::OtherModuleDiffers {
                id,
                here,
                module,
                capsule,
            } => {
                let edited = match here {
                    Some(h) => format!("the copy {h} loads"),
                    None => "the global copy".to_string(),
                };
                format!(
                    "{capsule} holds {module}'s own {id}, which is not {edited}; written to Override, the edit would replace it in {module} too"
                )
            }
            Error::ScriptDiffers {
                dialog,
                key,
                expected,
                found,
            } => {
                let name = |s: &Option<String>| match s {
                    Some(s) => format!("`{s}`"),
                    None => "no script".to_string(),
                };
                format!(
                    "{dialog}'s node {key} runs {}, not {}, the script the edit replaces",
                    name(found),
                    name(expected)
                )
            }
            Error::GlobalUndeclared { name } => format!(
                "globalcat.2da does not declare {name}; the engine keeps no such global, reading it answers 0 and setting it does nothing"
            ),
            Error::OverrideShadows { id, module, path } => format!(
                "Override holds {id} ({}), which outranks {module}'s capsules; written into {module}.mod, the edit would never be loaded",
                path.display()
            ),
            Error::NoModule { id } => format!(
                "the mod works in no module, so there is no capsule to write {id} into; name the module first"
            ),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for Error {}

macro_rules! from {
    ($($variant:ident($ty:ty)),* $(,)?) => {
        $(impl From<$ty> for Error {
            fn from(e: $ty) -> Self {
                Error::$variant(e)
            }
        })*
    };
}

from!(
    Res(rakata_res::Error),
    Game(rakata_game::Error),
    Dialog(rakata_dialog::Error),
    Gff(rakata_gff::Error),
    TwoDa(rakata_2da::Error),
    Lip(rakata_lip::Error),
    Tlk(rakata_tlk::Error),
);
