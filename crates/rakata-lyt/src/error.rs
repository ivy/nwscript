//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;

/// Diagnostic codes for layouts and visibility: the `RE23xx` range of the
/// `rakata` registry. Each is a stable string a caller may match on and a
/// user may look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// No `beginlayout` line: not a room layout.
    pub const NOT_A_LYT: &str = "RE2301";
    /// A layout line in no form the layout takes where it stands.
    pub const UNEXPECTED_LINE: &str = "RE2302";
    /// A count or coordinate that does not read as a number.
    pub const BAD_NUMBER: &str = "RE2303";
    /// A layout section whose rows disagree with its count.
    pub const COUNT_MISMATCH: &str = "RE2304";
    /// A layout with no `donelayout` line.
    pub const UNTERMINATED: &str = "RE2305";
    /// A byte outside ASCII.
    pub const NOT_ASCII: &str = "RE2306";
    /// A value the text form cannot hold, refused by a writer.
    pub const UNWRITABLE: &str = "RE2307";
    /// A visibility file's indented room before any room it belongs to.
    pub const ORPHAN_VISIBLE: &str = "RE2311";
    /// A visibility line in no form the format takes.
    pub const BAD_VIS_LINE: &str = "RE2312";
}

/// An error in reading or writing a layout or a visibility file. Line
/// numbers count from one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No `beginlayout` line.
    NotALyt,
    /// A layout line in no form the layout takes where it stands.
    UnexpectedLine {
        line: usize,
        text: String,
        /// What the layout takes there.
        expected: &'static str,
    },
    /// A count or coordinate that does not read as a number.
    BadNumber { line: usize, text: String },
    /// A section whose count line declares one number of rows and is
    /// followed by another.
    CountMismatch {
        /// The line of the count.
        line: usize,
        section: &'static str,
        declared: usize,
        found: usize,
    },
    /// No `donelayout` line.
    Unterminated,
    /// A byte outside ASCII.
    NotAscii { offset: usize, byte: u8 },
    /// A value the text form cannot hold: a float that is not finite, or a
    /// name that is empty or holds whitespace or a byte outside ASCII.
    Unwritable { what: String, reason: &'static str },
    /// An indented room before any room line it could belong to.
    OrphanVisible { line: usize, text: String },
    /// A visibility line in no form the format takes.
    BadVisLine { line: usize, text: String },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::NotALyt => codes::NOT_A_LYT,
            Error::UnexpectedLine { .. } => codes::UNEXPECTED_LINE,
            Error::BadNumber { .. } => codes::BAD_NUMBER,
            Error::CountMismatch { .. } => codes::COUNT_MISMATCH,
            Error::Unterminated => codes::UNTERMINATED,
            Error::NotAscii { .. } => codes::NOT_ASCII,
            Error::Unwritable { .. } => codes::UNWRITABLE,
            Error::OrphanVisible { .. } => codes::ORPHAN_VISIBLE,
            Error::BadVisLine { .. } => codes::BAD_VIS_LINE,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Error::NotALyt => f.write_str("not a room layout: no `beginlayout` line"),
            Error::UnexpectedLine {
                line,
                text,
                expected,
            } => write!(f, "line {line} `{text}` is not {expected}"),
            Error::BadNumber { line, text } => {
                write!(f, "line {line}: `{text}` is not a finite number")
            }
            Error::CountMismatch {
                line,
                section,
                declared,
                found,
            } => write!(
                f,
                "line {line}: `{section}` declares {declared} rows and {found} follow"
            ),
            Error::Unterminated => f.write_str("the layout has no `donelayout` line"),
            Error::NotAscii { offset, byte } => {
                write!(f, "byte {byte:#04x} at offset {offset:#x} is not ASCII")
            }
            Error::Unwritable { what, reason } => write!(f, "cannot write {what}: {reason}"),
            Error::OrphanVisible { line, text } => write!(
                f,
                "line {line}: indented room `{text}` comes before any room line"
            ),
            Error::BadVisLine { line, text } => write!(
                f,
                "line {line} `{text}` is neither `ROOM COUNT` nor an indented room"
            ),
        }
    }
}

impl std::error::Error for Error {}
