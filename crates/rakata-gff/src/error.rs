//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;

/// Diagnostic codes for GFF: the `RE06xx` range of the `rakata` registry.
/// Each is a stable string a caller may match on and a user may look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// The bytes do not begin with a four-byte type and the version `V3.2`.
    pub const NOT_GFF: &str = "RE0601";
    /// A section, or a field's data, lies beyond the end of the bytes.
    pub const TRUNCATED: &str = "RE0602";
    /// A struct, field or label index is out of range, or a struct is
    /// reached twice.
    pub const BAD_INDEX: &str = "RE0603";
    /// A field type id this crate does not know.
    pub const UNKNOWN_TYPE: &str = "RE0604";
    /// A GFF too large for its 32-bit offsets and counts.
    pub const TOO_LARGE: &str = "RE0605";
    /// A field path that does not parse.
    pub const BAD_PATH: &str = "RE0606";
    /// A field path naming nothing in the tree.
    pub const NO_SUCH_PATH: &str = "RE0607";
    /// A label or a resref longer than sixteen bytes.
    pub const TOO_LONG: &str = "RE0608";
}

/// An error in reading, writing or addressing a GFF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes do not begin with a four-byte type and the version `V3.2`.
    NotGff {
        /// The first eight bytes found, lossily as text.
        found: String,
    },
    /// A section, or a field's data, lies beyond the end of the bytes.
    Truncated {
        /// What was being read.
        what: String,
        /// The offset at which it begins.
        offset: usize,
        /// Its length.
        len: usize,
        /// The length of the bytes.
        available: usize,
    },
    /// A struct, field or label index is out of range, or a struct is
    /// reached twice.
    BadIndex {
        /// What kind of index: `struct`, `field` or `label`.
        what: &'static str,
        /// The index found.
        index: u32,
        /// How many there are.
        count: u32,
    },
    /// A field type id this crate does not know.
    UnknownType {
        /// The id as stored.
        type_id: u32,
        /// The field's position in the field array.
        field: u32,
    },
    /// The GFF is too large for its 32-bit offsets and counts.
    TooLarge {
        /// The size it would have been.
        size: u64,
    },
    /// A field path that does not parse.
    BadPath {
        /// The text.
        text: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A field path naming nothing in the tree.
    NoSuchPath {
        /// The path as far as it resolved.
        at: String,
        /// Why the next step fails.
        reason: String,
    },
    /// A label or a resref longer than sixteen bytes.
    TooLong {
        /// `label` or `resref`.
        what: &'static str,
        /// The text, lossily.
        text: String,
        /// Its length in bytes.
        len: usize,
    },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::NotGff { .. } => codes::NOT_GFF,
            Error::Truncated { .. } => codes::TRUNCATED,
            Error::BadIndex { .. } => codes::BAD_INDEX,
            Error::UnknownType { .. } => codes::UNKNOWN_TYPE,
            Error::TooLarge { .. } => codes::TOO_LARGE,
            Error::BadPath { .. } => codes::BAD_PATH,
            Error::NoSuchPath { .. } => codes::NO_SUCH_PATH,
            Error::TooLong { .. } => codes::TOO_LONG,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Error::NotGff { found } => {
                write!(f, "not a GFF V3.2 file: the file begins `{found}`")
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
            Error::BadIndex { what, index, count } => {
                write!(f, "{what} index {index} is out of range: there are {count}")
            }
            Error::UnknownType { type_id, field } => {
                write!(
                    f,
                    "field {field} has type id {type_id}, which names no GFF type"
                )
            }
            Error::TooLarge { size } => write!(
                f,
                "a GFF of {size} bytes exceeds the format's 32-bit offsets"
            ),
            Error::BadPath { text, reason } => {
                write!(f, "cannot parse the path `{text}`: {reason}")
            }
            Error::NoSuchPath { at, reason } => {
                if at.is_empty() {
                    write!(f, "at the root: {reason}")
                } else {
                    write!(f, "at `{at}`: {reason}")
                }
            }
            Error::TooLong { what, text, len } => {
                write!(f, "the {what} `{text}` is {len} bytes; sixteen is the most")
            }
        }
    }
}

impl std::error::Error for Error {}
