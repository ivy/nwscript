//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;
use std::path::PathBuf;

/// Diagnostic codes for the talk table: the `RE08xx` range of the `rakata`
/// registry, plus `RE0001` for a file that could not be read. Each is a
/// stable string a caller may match on and a user may look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// A file could not be read or written.
    pub const IO: &str = "RE0001";
    /// The bytes do not begin `TLK V3.0`.
    pub const NOT_A_TALK_TABLE: &str = "RE0801";
    /// The header, the entry table or a string lies beyond the end of the
    /// bytes.
    pub const TRUNCATED: &str = "RE0802";
    /// An entry's text holds a character Windows-1252 cannot encode.
    pub const NOT_ENCODABLE: &str = "RE0803";
    /// An entry with text whose flags do not say text is present.
    pub const TEXT_WITHOUT_FLAG: &str = "RE0804";
    /// A talk table too large for its 32-bit offsets and counts.
    pub const TOO_LARGE: &str = "RE0805";
}

/// An error in reading or writing a talk table.
#[derive(Debug)]
pub enum Error {
    /// A file could not be read or written.
    Io {
        /// The file.
        path: PathBuf,
        /// What the operating system said.
        source: std::io::Error,
    },
    /// The bytes do not begin with `TLK V3.0`.
    NotATalkTable {
        /// The first eight bytes found, lossily as text.
        found: String,
    },
    /// The header, the entry table or a string lies beyond the end of the
    /// bytes.
    Truncated {
        /// What was being read: `header`, `entry table`, or `the text of
        /// strref N`.
        what: String,
        /// The offset at which it begins.
        offset: usize,
        /// Its length in bytes.
        len: usize,
        /// The length of the bytes.
        available: usize,
    },
    /// An entry's text holds a character Windows-1252 cannot encode.
    NotEncodable {
        /// The entry's strref.
        strref: u32,
        /// The character.
        ch: char,
    },
    /// An entry has text, but its flags do not say text is present, so a
    /// writer would drop it and the engine would never show it.
    TextWithoutFlag {
        /// The entry's strref.
        strref: u32,
    },
    /// The entries or their text exceed what 32-bit fields can address.
    TooLarge {
        /// What overflowed: `entry count` or `string data`.
        what: &'static str,
        /// Its size.
        size: usize,
    },
}

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::Io { .. } => codes::IO,
            Error::NotATalkTable { .. } => codes::NOT_A_TALK_TABLE,
            Error::Truncated { .. } => codes::TRUNCATED,
            Error::NotEncodable { .. } => codes::NOT_ENCODABLE,
            Error::TextWithoutFlag { .. } => codes::TEXT_WITHOUT_FLAG,
            Error::TooLarge { .. } => codes::TOO_LARGE,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Error::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Error::NotATalkTable { found } => {
                write!(f, "not a `TLK V3.0` talk table: the file begins `{found}`")
            }
            Error::Truncated {
                what,
                offset,
                len,
                available,
            } => write!(
                f,
                "{what} at offset {offset} for {len} bytes lies beyond the end of the {available}-byte file"
            ),
            Error::NotEncodable { strref, ch } => write!(
                f,
                "the text of strref {strref} holds {ch:?} (U+{:04X}), which Windows-1252 cannot encode",
                *ch as u32
            ),
            Error::TextWithoutFlag { strref } => write!(
                f,
                "strref {strref} has text but its flags do not set TEXT_PRESENT (0x1)"
            ),
            Error::TooLarge { what, size } => {
                write!(f, "{what} of {size} exceeds the 32-bit field that holds it")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
