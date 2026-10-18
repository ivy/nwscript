//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;
use std::path::PathBuf;

/// Diagnostic codes for lip-sync files: the `RE09xx` range of the `rakata`
/// registry, plus `RE0001` for a file that could not be read. Each is a
/// stable string a caller may match on and a user may look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// A file could not be read.
    pub const IO: &str = "RE0001";
    /// The bytes do not begin with `LIP V1.0`.
    pub const NOT_A_LIP: &str = "RE0901";
    /// The header or the keyframes it counts lie beyond the end of the bytes.
    pub const TRUNCATED: &str = "RE0902";
    /// A keyframe's shape byte is above 15, naming no mouth shape.
    pub const BAD_SHAPE: &str = "RE0903";
    /// Bytes follow the last keyframe.
    pub const TRAILING_BYTES: &str = "RE0904";
    /// More keyframes than a 32-bit count can hold.
    pub const TOO_MANY_KEYS: &str = "RE0905";
}

/// An error in reading or writing a lip-sync file.
#[derive(Debug)]
pub enum Error {
    /// A file could not be read.
    Io {
        /// The file.
        path: PathBuf,
        /// What the operating system said.
        source: std::io::Error,
    },
    /// The bytes do not begin with `LIP V1.0`.
    NotALip {
        /// The first eight bytes found, lossily as text.
        found: String,
    },
    /// The header, or the keyframes it counts, lie beyond the end of the
    /// bytes.
    Truncated {
        /// What was being read.
        what: &'static str,
        /// The length the layout requires.
        needed: usize,
        /// The length of the bytes.
        available: usize,
    },
    /// A keyframe's shape byte names no mouth shape.
    BadShape {
        /// The keyframe's position, from zero.
        index: usize,
        /// The byte as stored.
        value: u8,
    },
    /// Bytes follow the last keyframe.
    TrailingBytes {
        /// How many.
        len: usize,
    },
    /// More keyframes than the format's 32-bit count can hold.
    TooManyKeys {
        /// How many there are.
        len: usize,
    },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::Io { .. } => codes::IO,
            Error::NotALip { .. } => codes::NOT_A_LIP,
            Error::Truncated { .. } => codes::TRUNCATED,
            Error::BadShape { .. } => codes::BAD_SHAPE,
            Error::TrailingBytes { .. } => codes::TRAILING_BYTES,
            Error::TooManyKeys { .. } => codes::TOO_MANY_KEYS,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Error::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Error::NotALip { found } => {
                write!(f, "not a `LIP V1.0` file: the file begins `{found}`")
            }
            Error::Truncated {
                what,
                needed,
                available,
            } => write!(
                f,
                "the {what} needs {needed} bytes and the file has {available}"
            ),
            Error::BadShape { index, value } => write!(
                f,
                "keyframe {index} has shape {value}; the sixteen shapes are 0 to 15"
            ),
            Error::TrailingBytes { len } => {
                write!(f, "{len} bytes follow the last keyframe")
            }
            Error::TooManyKeys { len } => {
                write!(f, "{len} keyframes exceed the format's 32-bit count")
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
