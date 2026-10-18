//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;

/// Diagnostic codes for resource identity: the `RE01xx` range of the
/// `rakata` registry. Each is a stable string a caller may match on and a
/// user may look up.
// TODO(rakata): register these, with their explanations, in
// `crates/rakata/src/codes.rs` so `rakata explain RE0101` prints them. This
// crate depends on nothing, so it carries only the strings.
pub mod codes {
    /// A resource name longer than the sixteen bytes a resref holds.
    pub const RESREF_TOO_LONG: &str = "RE0101";
    /// A resource id without the `.ext` that names its type.
    pub const RESID_NO_EXTENSION: &str = "RE0102";
    /// A file extension that names no resource type this crate knows.
    pub const UNKNOWN_EXTENSION: &str = "RE0103";
}

/// An error in building a resource identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The name is longer than the sixteen bytes a resref holds.
    ResRefTooLong {
        /// The name as given.
        name: String,
        /// Its length in bytes.
        len: usize,
    },
    /// The resource id has no `.ext` to name its type.
    NoExtension {
        /// The text as given.
        text: String,
    },
    /// The extension names no known resource type.
    UnknownExtension {
        /// The extension as given, without its dot.
        extension: String,
    },
}

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::ResRefTooLong { .. } => codes::RESREF_TOO_LONG,
            Error::NoExtension { .. } => codes::RESID_NO_EXTENSION,
            Error::UnknownExtension { .. } => codes::UNKNOWN_EXTENSION,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ResRefTooLong { name, len } => write!(
                f,
                "{}: resource name `{name}` is {len} bytes; a resref holds at most 16",
                self.code()
            ),
            Error::NoExtension { text } => write!(
                f,
                "{}: resource id `{text}` has no `.ext` naming its type",
                self.code()
            ),
            Error::UnknownExtension { extension } => write!(
                f,
                "{}: `.{extension}` names no known resource type",
                self.code()
            ),
        }
    }
}

impl std::error::Error for Error {}
