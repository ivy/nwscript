//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;
use std::path::PathBuf;

/// Diagnostic codes for saves: the `RE12xx` and `RW12xx` range of the
/// `rakata` registry. A member that does not read as an archive or a GFF
/// keeps the code of the crate that read it.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// The directory holds no `SAVEGAME.sav`.
    pub const NOT_A_SAVE: &str = "RE1201";
    /// The save holds no state for the module asked for.
    pub const NO_SUCH_MODULE: &str = "RE1202";
    /// A member of the save lacks a field the game writes, or holds it with
    /// another type.
    pub const BAD_SHAPE: &str = "RE1203";
    /// No creature in the places searched carries the tag asked for.
    pub const NO_SUCH_CREATURE: &str = "RE1204";
    /// The names of the creatures' local variables could not be read, so
    /// they are shown by index alone.
    pub const NO_LOCAL_NAMES: &str = "RW1201";
    /// A global the install's `globalcat.2da` does not declare, set without
    /// `--declare`.
    pub const UNDECLARED: &str = "RE1205";
    /// A `NAME=VALUE` that does not parse, or whose value does not fit the
    /// global's type.
    pub const BAD_ASSIGNMENT: &str = "RE1206";
    /// A party member the save holds no `AVAILNPC<n>.utc` for.
    pub const NO_SUCH_NPC: &str = "RE1207";
    /// A party the game cannot take: a member named twice, or more than two.
    pub const BAD_PARTY: &str = "RE1208";
    /// The new save's directory exists and is not empty, or is the save
    /// being read.
    pub const OUT_EXISTS: &str = "RE1209";
    /// A file of the new save could not be written.
    pub const WRITE: &str = "RE1210";
    /// A global written that the install does not declare, so its scripts
    /// do not read it.
    pub const UNDECLARED_WRITTEN: &str = "RW1202";
}

/// An error in reading or writing a save.
#[derive(Debug)]
pub enum Error {
    /// The directory holds no `SAVEGAME.sav`.
    NotASave {
        /// The directory named.
        dir: PathBuf,
    },
    /// A file of the save could not be read.
    Io {
        /// The file.
        path: PathBuf,
        /// What the operating system said.
        source: std::io::Error,
    },
    /// A member does not read as an archive.
    Archive {
        /// Where the member is, as `SAVEGAME.sav/danm14ac.sav`.
        member: String,
        /// The archive reader's error.
        source: rakata_archive::Error,
    },
    /// A member does not read as a GFF.
    Gff {
        /// Where the member is, as `SAVEGAME.sav/danm14ac.sav/m14ac.git`.
        member: String,
        /// The GFF reader's error.
        source: rakata_gff::Error,
    },
    /// The save holds no state for this module.
    NoSuchModule {
        /// The module asked for.
        name: String,
        /// The modules the save holds, in archive order.
        available: Vec<String>,
    },
    /// A member lacks a field the game writes, or holds it with another
    /// type.
    BadShape {
        /// Where the member is.
        member: String,
        /// The field, as a GFF path.
        path: String,
        /// What was expected there.
        expected: &'static str,
    },
    /// A global the install does not declare, set without `declare`.
    Undeclared { name: String },
    /// An assignment that does not parse or does not fit.
    BadAssignment {
        /// The assignment as given.
        text: String,
        reason: String,
    },
    /// A party member the save holds no `AVAILNPC<n>.utc` for.
    NoSuchNpc {
        /// The number or tag given.
        name: String,
        /// The NPCs the save holds, as `2 (carth)`.
        available: Vec<String>,
    },
    /// A party the game cannot take.
    BadParty { reason: String },
    /// The new save's directory cannot take it.
    OutExists { dir: PathBuf, reason: String },
    /// A file of the new save could not be written.
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::NotASave { .. } => codes::NOT_A_SAVE,
            Error::Io { .. } => rakata_archive::codes::IO,
            Error::Archive { source, .. } => source.code(),
            Error::Gff { source, .. } => source.code(),
            Error::NoSuchModule { .. } => codes::NO_SUCH_MODULE,
            Error::BadShape { .. } => codes::BAD_SHAPE,
            Error::Undeclared { .. } => codes::UNDECLARED,
            Error::BadAssignment { .. } => codes::BAD_ASSIGNMENT,
            Error::NoSuchNpc { .. } => codes::NO_SUCH_NPC,
            Error::BadParty { .. } => codes::BAD_PARTY,
            Error::OutExists { .. } => codes::OUT_EXISTS,
            Error::Write { .. } => codes::WRITE,
        }
    }

    /// The message without the code in front of it.
    pub fn message(&self) -> String {
        match self {
            Error::NotASave { dir } => format!(
                "{} is not a saved game: it holds no SAVEGAME.sav",
                dir.display()
            ),
            Error::Io { path, source } => format!("cannot read {}: {source}", path.display()),
            Error::Archive { member, source } => {
                format!(
                    "{member}: {}",
                    strip_code(source.code(), &source.to_string())
                )
            }
            Error::Gff { member, source } => {
                format!(
                    "{member}: {}",
                    strip_code(source.code(), &source.to_string())
                )
            }
            Error::NoSuchModule { name, available } => format!(
                "the save holds no state for module `{name}`; it holds {}",
                if available.is_empty() {
                    "none".to_owned()
                } else {
                    available.join(", ")
                }
            ),
            Error::BadShape {
                member,
                path,
                expected,
            } if path.is_empty() => format!("{member} is not {expected}"),
            Error::BadShape {
                member,
                path,
                expected,
            } => format!("{member}: `{path}` is not {expected}"),
            Error::Undeclared { name } => format!(
                "the install's globalcat.2da does not declare the global `{name}`, so the game's scripts would not read it"
            ),
            Error::BadAssignment { text, reason } => format!("`{text}`: {reason}"),
            Error::NoSuchNpc { name, available } => format!(
                "the save holds no AVAILNPC for party member `{name}`; it holds {}",
                if available.is_empty() {
                    "none".to_owned()
                } else {
                    available.join(", ")
                }
            ),
            Error::BadParty { reason } => format!("the party cannot be written: {reason}"),
            Error::OutExists { dir, reason } => {
                format!("cannot write the new save to {}: {reason}", dir.display())
            }
            Error::Write { path, source } => format!("cannot write {}: {source}", path.display()),
        }
    }
}

/// The reader crates render `CODE: message`; the code travels apart here.
fn strip_code<'a>(code: &str, text: &'a str) -> &'a str {
    text.strip_prefix(code)
        .and_then(|t| t.strip_prefix(": "))
        .unwrap_or(text)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } | Error::Write { source, .. } => Some(source),
            Error::Archive { source, .. } => Some(source),
            Error::Gff { source, .. } => Some(source),
            _ => None,
        }
    }
}
