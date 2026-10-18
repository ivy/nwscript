//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;

/// Diagnostic codes for walkmeshes: the `RE25xx` range of the `rakata`
/// registry. Each is a stable string a caller may match on and a user may
/// look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// The bytes do not begin `BWM V1.0`.
    pub const NOT_A_WALKMESH: &str = "RE2501";
    /// The header, or an array the header places, runs past the end.
    pub const TRUNCATED: &str = "RE2502";
    /// The header's type is neither 0 (an object's) nor 1 (an area room's).
    pub const UNKNOWN_KIND: &str = "RE2503";
    /// An index from one array into another names no element of it.
    pub const BAD_INDEX: &str = "RE2504";
    /// A surface material table without a `walk` column, or with a `walk`
    /// cell that is not a number.
    pub const BAD_SURFACE_TABLE: &str = "RE2505";
    /// A count, offset or index too large for the field the writer stores
    /// it in.
    pub const TOO_LARGE: &str = "RE2506";
}

/// An error in reading a walkmesh or a surface material table.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The bytes do not begin `BWM V1.0`.
    NotAWalkmesh,
    /// The header, or an array the header places, runs past the end.
    Truncated {
        /// What was being read.
        what: &'static str,
        /// The length the layout requires.
        needed: usize,
        /// The length of the bytes.
        available: usize,
    },
    /// The header's type at offset 8 is neither 0 nor 1.
    UnknownKind {
        /// The type read.
        kind: u32,
    },
    /// An index from one array into another names no element of it.
    BadIndex {
        /// The array holding the index, and which of its fields.
        what: &'static str,
        /// The element of that array.
        at: usize,
        /// The value read.
        value: i64,
        /// The number of elements the index may name.
        limit: usize,
    },
    /// A surface material table without a `walk` column, or with a `walk`
    /// cell that is not a number.
    BadSurfaceTable {
        /// Why.
        reason: String,
    },
    /// A count, offset or index too large for the field the writer stores
    /// it in.
    TooLarge {
        /// The value, and where it goes.
        what: &'static str,
        /// The value.
        value: u64,
        /// The largest value the field holds.
        limit: u64,
    },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::NotAWalkmesh => codes::NOT_A_WALKMESH,
            Error::Truncated { .. } => codes::TRUNCATED,
            Error::UnknownKind { .. } => codes::UNKNOWN_KIND,
            Error::BadIndex { .. } => codes::BAD_INDEX,
            Error::BadSurfaceTable { .. } => codes::BAD_SURFACE_TABLE,
            Error::TooLarge { .. } => codes::TOO_LARGE,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Error::NotAWalkmesh => write!(f, "not a walkmesh: the bytes do not begin `BWM V1.0`"),
            Error::Truncated {
                what,
                needed,
                available,
            } => write!(
                f,
                "the walkmesh's {what} needs {needed} bytes and the file has {available}"
            ),
            Error::UnknownKind { kind } => write!(
                f,
                "walkmesh type {kind} is neither 0 (a placeable's or door's) nor 1 (an area room's)"
            ),
            Error::BadIndex {
                what,
                at,
                value,
                limit,
            } => write!(
                f,
                "the walkmesh's {what} {at} is {value}, and the array it names holds {limit}"
            ),
            Error::BadSurfaceTable { reason } => write!(f, "surface material table: {reason}"),
            Error::TooLarge { what, value, limit } => write!(
                f,
                "the walkmesh's {what} is {value}, and the field it is written to holds at most {limit}"
            ),
        }
    }
}

impl std::error::Error for Error {}
